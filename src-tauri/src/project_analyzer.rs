use crate::error::{AutoLaunchError, Result};
use crate::models::{ProjectInfo, TechStack, Dependency, ConfigFile, ConfigFileType, SecurityWarning};
use std::path::{Path, PathBuf};
use std::fs;
use regex::Regex;
use serde_json::Value;

// Трейт для анализа проектов
pub trait ProjectAnalyzerTrait {
    fn analyze_project(&self, path: &Path) -> Result<ProjectInfo>;
    fn detect_stack(&self, files: &[PathBuf]) -> TechStack;
    fn find_entry_point(&self, stack: &TechStack, config_files: &[ConfigFile], project_path: &Path) -> Result<Option<String>>;
    fn parse_dependencies(&self, config_files: &[ConfigFile], project_path: &Path) -> Result<Vec<Dependency>>;
}

pub struct ProjectAnalyzer;


impl ProjectAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze_project(&self, path: &Path) -> Result<ProjectInfo> {
        let files = self.scan_directory(path)?;
        let stack = self.detect_stack(&files);
        let config_files = self.find_config_files(&files);
        let entry_command = self.find_entry_point(&stack, &config_files, path)?;
        let dependencies = self.parse_dependencies(&config_files, path)?;
        let security_warnings = self.scan_for_security_issues(&config_files, path)?;

        Ok(ProjectInfo {
            stack,
            entry_command,
            dependencies,
            config_files,
            security_warnings,
            trust_level: crate::models::TrustLevel::Unknown, // По умолчанию проект неизвестный
        })
    }

    fn scan_directory(&self, path: &Path) -> Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        
        if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_file() {
                    files.push(path);
                } else if path.is_dir() && !self.should_skip_directory(&path) {
                    // Рекурсивно сканируем только важные директории
                    if let Ok(mut subfiles) = self.scan_directory(&path) {
                        files.append(&mut subfiles);
                    }
                }
            }
        }
        
        Ok(files)
    }

    fn should_skip_directory(&self, path: &Path) -> bool {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            matches!(name, "node_modules" | ".git" | "target" | "__pycache__" | ".venv" | "venv")
        } else {
            false
        }
    }

    pub fn detect_stack(&self, files: &[PathBuf]) -> TechStack {
        // Проверяем наличие специфичных файлов конфигурации
        for file in files {
            if let Some(name) = file.file_name().and_then(|n| n.to_str()) {
                match name {
                    "package.json" => return TechStack::NodeJs { version: None },
                    "requirements.txt" | "pyproject.toml" | "setup.py" => return TechStack::Python { version: None },
                    "Cargo.toml" => return TechStack::Rust { edition: None },
                    "go.mod" => return TechStack::Go { version: None },
                    "pom.xml" | "build.gradle" => return TechStack::Java { version: None },
                    "Dockerfile" => return TechStack::Docker { compose: false },
                    "docker-compose.yml" | "docker-compose.yaml" => return TechStack::Docker { compose: true },
                    _ => {}
                }
            }
        }

        // Проверяем наличие HTML/CSS/JS файлов для статических сайтов
        let has_html = files.iter().any(|f| f.extension().map_or(false, |ext| ext == "html"));
        let has_css = files.iter().any(|f| f.extension().map_or(false, |ext| ext == "css"));
        let has_js = files.iter().any(|f| f.extension().map_or(false, |ext| ext == "js"));

        if has_html || (has_css && has_js) {
            return TechStack::Static { framework: None };
        }

        TechStack::Unknown
    }

    fn find_config_files(&self, files: &[PathBuf]) -> Vec<ConfigFile> {
        let mut config_files = Vec::new();

        for file in files {
            if let Some(name) = file.file_name().and_then(|n| n.to_str()) {
                let file_type = match name {
                    "package.json" => Some(ConfigFileType::PackageJson),
                    "requirements.txt" => Some(ConfigFileType::RequirementsTxt),
                    "pyproject.toml" => Some(ConfigFileType::PyprojectToml),
                    "Cargo.toml" => Some(ConfigFileType::CargoToml),
                    "Dockerfile" => Some(ConfigFileType::Dockerfile),
                    "docker-compose.yml" | "docker-compose.yaml" => Some(ConfigFileType::DockerCompose),
                    "go.mod" => Some(ConfigFileType::GoMod),
                    "pom.xml" => Some(ConfigFileType::PomXml),
                    _ => None,
                };

                if let Some(file_type) = file_type {
                    config_files.push(ConfigFile {
                        path: file.clone(),
                        file_type,
                    });
                }
            }
        }

        config_files
    }

    pub fn find_entry_point(&self, stack: &TechStack, config_files: &[ConfigFile], project_path: &Path) -> Result<Option<String>> {
        match stack {
            TechStack::NodeJs { .. } => {
                // Ищем в package.json
                for config in config_files {
                    if matches!(config.file_type, ConfigFileType::PackageJson) {
                        if let Ok(content) = fs::read_to_string(&config.path) {
                            if let Ok(json) = serde_json::from_str::<Value>(&content) {
                                if let Some(scripts) = json.get("scripts") {
                                    if let Some(start) = scripts.get("start") {
                                        if let Some(command) = start.as_str() {
                                            return Ok(Some(command.to_string()));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                
                // Fallback: ищем index.js
                if project_path.join("index.js").exists() {
                    return Ok(Some("node index.js".to_string()));
                }
            }
            TechStack::Python { .. } => {
                // Ищем main.py
                if project_path.join("main.py").exists() {
                    return Ok(Some("python main.py".to_string()));
                }
                // Ищем app.py
                if project_path.join("app.py").exists() {
                    return Ok(Some("python app.py".to_string()));
                }
            }
            TechStack::Rust { .. } => {
                return Ok(Some("cargo run".to_string()));
            }
            TechStack::Go { .. } => {
                return Ok(Some("go run .".to_string()));
            }
            TechStack::Docker { .. } => {
                return Ok(Some("docker-compose up".to_string()));
            }
            _ => {}
        }

        Ok(None)
    }

    fn parse_dependencies(&self, config_files: &[ConfigFile], _project_path: &Path) -> Result<Vec<Dependency>> {
        let mut dependencies = Vec::new();

        for config in config_files {
            match config.file_type {
                ConfigFileType::PackageJson => {
                    if let Ok(content) = fs::read_to_string(&config.path) {
                        if let Ok(json) = serde_json::from_str::<Value>(&content) {
                            // Обычные зависимости
                            if let Some(deps) = json.get("dependencies").and_then(|d| d.as_object()) {
                                for (name, version) in deps {
                                    dependencies.push(Dependency {
                                        name: name.clone(),
                                        version: version.as_str().map(|s| s.to_string()),
                                        dev: false,
                                    });
                                }
                            }
                            
                            // Dev зависимости
                            if let Some(dev_deps) = json.get("devDependencies").and_then(|d| d.as_object()) {
                                for (name, version) in dev_deps {
                                    dependencies.push(Dependency {
                                        name: name.clone(),
                                        version: version.as_str().map(|s| s.to_string()),
                                        dev: true,
                                    });
                                }
                            }
                        }
                    }
                }
                ConfigFileType::RequirementsTxt => {
                    if let Ok(content) = fs::read_to_string(&config.path) {
                        for line in content.lines() {
                            let line = line.trim();
                            if !line.is_empty() && !line.starts_with('#') {
                                let parts: Vec<&str> = line.split("==").collect();
                                let name = parts[0].to_string();
                                let version = if parts.len() > 1 {
                                    Some(parts[1].to_string())
                                } else {
                                    None
                                };
                                
                                dependencies.push(Dependency {
                                    name,
                                    version,
                                    dev: false,
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        Ok(dependencies)
    }

    fn scan_for_security_issues(&self, _config_files: &[ConfigFile], _project_path: &Path) -> Result<Vec<SecurityWarning>> {
        // Базовая реализация - будет расширена в следующих задачах
        Ok(Vec::new())
    }
}

// Реализация трейта ProjectAnalyzerTrait для ProjectAnalyzer
impl ProjectAnalyzerTrait for ProjectAnalyzer {
    fn analyze_project(&self, path: &Path) -> Result<ProjectInfo> {
        self.analyze_project(path)
    }

    fn detect_stack(&self, files: &[PathBuf]) -> TechStack {
        self.detect_stack(files)
    }

    fn find_entry_point(&self, stack: &TechStack, config_files: &[ConfigFile], project_path: &Path) -> Result<Option<String>> {
        self.find_entry_point(stack, config_files, project_path)
    }

    fn parse_dependencies(&self, config_files: &[ConfigFile], project_path: &Path) -> Result<Vec<Dependency>> {
        self.parse_dependencies(config_files, project_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_project(files: &[(&str, &str)]) -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        
        for (path, content) in files {
            let file_path = temp_dir.path().join(path);
            if let Some(parent) = file_path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(file_path, content).unwrap();
        }
        
        temp_dir
    }

    #[test]
    fn test_detect_nodejs_stack() {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = create_test_project(&[
            ("package.json", r#"{"name": "test", "scripts": {"start": "node index.js"}}"#),
            ("index.js", "console.log('Hello World');"),
        ]);
        
        let files = analyzer.scan_directory(temp_dir.path()).unwrap();
        let stack = analyzer.detect_stack(&files);
        
        match stack {
            TechStack::NodeJs { .. } => {},
            _ => panic!("Expected NodeJs stack, got {:?}", stack),
        }
    }

    #[test]
    fn test_detect_python_stack() {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = create_test_project(&[
            ("requirements.txt", "flask==2.0.0\nrequests==2.25.0"),
            ("main.py", "print('Hello World')"),
        ]);
        
        let files = analyzer.scan_directory(temp_dir.path()).unwrap();
        let stack = analyzer.detect_stack(&files);
        
        match stack {
            TechStack::Python { .. } => {},
            _ => panic!("Expected Python stack, got {:?}", stack),
        }
    }

    #[test]
    fn test_detect_rust_stack() {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = create_test_project(&[
            ("Cargo.toml", r#"[package]
name = "test"
version = "0.1.0"
edition = "2021""#),
            ("src/main.rs", "fn main() { println!(\"Hello World\"); }"),
        ]);
        
        let files = analyzer.scan_directory(temp_dir.path()).unwrap();
        let stack = analyzer.detect_stack(&files);
        
        match stack {
            TechStack::Rust { .. } => {},
            _ => panic!("Expected Rust stack, got {:?}", stack),
        }
    }

    #[test]
    fn test_find_nodejs_entry_point() {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = create_test_project(&[
            ("package.json", r#"{"name": "test", "scripts": {"start": "npm run dev"}}"#),
        ]);
        
        let files = analyzer.scan_directory(temp_dir.path()).unwrap();
        let stack = TechStack::NodeJs { version: None };
        let config_files = analyzer.find_config_files(&files);
        let entry_point = analyzer.find_entry_point(&stack, &config_files, temp_dir.path()).unwrap();
        
        assert_eq!(entry_point, Some("npm run dev".to_string()));
    }

    #[test]
    fn test_find_python_entry_point() {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = create_test_project(&[
            ("main.py", "print('Hello World')"),
        ]);
        
        let stack = TechStack::Python { version: None };
        let config_files = Vec::new();
        let entry_point = analyzer.find_entry_point(&stack, &config_files, temp_dir.path()).unwrap();
        
        assert_eq!(entry_point, Some("python main.py".to_string()));
    }

    #[test]
    fn test_parse_package_json_dependencies() {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = create_test_project(&[
            ("package.json", r#"{
                "name": "test",
                "dependencies": {
                    "react": "^18.0.0",
                    "lodash": "4.17.21"
                },
                "devDependencies": {
                    "typescript": "^5.0.0"
                }
            }"#),
        ]);
        
        let files = analyzer.scan_directory(temp_dir.path()).unwrap();
        let config_files = analyzer.find_config_files(&files);
        let dependencies = analyzer.parse_dependencies(&config_files, temp_dir.path()).unwrap();
        
        assert_eq!(dependencies.len(), 3);
        
        let react_dep = dependencies.iter().find(|d| d.name == "react").unwrap();
        assert_eq!(react_dep.version, Some("^18.0.0".to_string()));
        assert!(!react_dep.dev);
        
        let ts_dep = dependencies.iter().find(|d| d.name == "typescript").unwrap();
        assert!(ts_dep.dev);
    }

    #[test]
    fn test_parse_requirements_txt() {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = create_test_project(&[
            ("requirements.txt", "flask==2.0.0\nrequests>=2.25.0\n# comment\nnumpy"),
        ]);
        
        let files = analyzer.scan_directory(temp_dir.path()).unwrap();
        let config_files = analyzer.find_config_files(&files);
        let dependencies = analyzer.parse_dependencies(&config_files, temp_dir.path()).unwrap();
        
        assert_eq!(dependencies.len(), 3);
        
        let flask_dep = dependencies.iter().find(|d| d.name == "flask").unwrap();
        assert_eq!(flask_dep.version, Some("2.0.0".to_string()));
        
        let numpy_dep = dependencies.iter().find(|d| d.name == "numpy").unwrap();
        assert_eq!(numpy_dep.version, None);
    }

    #[test]
    fn test_detect_docker_stack() {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = create_test_project(&[
            ("Dockerfile", "FROM node:18\nCOPY . .\nRUN npm install"),
        ]);
        
        let files = analyzer.scan_directory(temp_dir.path()).unwrap();
        let stack = analyzer.detect_stack(&files);
        
        match stack {
            TechStack::Docker { compose: false } => {},
            _ => panic!("Expected Docker stack, got {:?}", stack),
        }
    }

    #[test]
    fn test_detect_docker_compose_stack() {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = create_test_project(&[
            ("docker-compose.yml", "version: '3'\nservices:\n  web:\n    build: ."),
        ]);
        
        let files = analyzer.scan_directory(temp_dir.path()).unwrap();
        let stack = analyzer.detect_stack(&files);
        
        match stack {
            TechStack::Docker { compose: true } => {},
            _ => panic!("Expected Docker Compose stack, got {:?}", stack),
        }
    }

    #[test]
    fn test_detect_go_stack() {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = create_test_project(&[
            ("go.mod", "module example.com/myapp\n\ngo 1.21"),
        ]);
        
        let files = analyzer.scan_directory(temp_dir.path()).unwrap();
        let stack = analyzer.detect_stack(&files);
        
        match stack {
            TechStack::Go { .. } => {},
            _ => panic!("Expected Go stack, got {:?}", stack),
        }
    }

    #[test]
    fn test_detect_static_stack() {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = create_test_project(&[
            ("index.html", "<!DOCTYPE html><html><body>Hello</body></html>"),
            ("style.css", "body { margin: 0; }"),
        ]);
        
        let files = analyzer.scan_directory(temp_dir.path()).unwrap();
        let stack = analyzer.detect_stack(&files);
        
        match stack {
            TechStack::Static { .. } => {},
            _ => panic!("Expected Static stack, got {:?}", stack),
        }
    }

    #[test]
    fn test_detect_unknown_stack() {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = create_test_project(&[
            ("README.md", "# My Project"),
        ]);
        
        let files = analyzer.scan_directory(temp_dir.path()).unwrap();
        let stack = analyzer.detect_stack(&files);
        
        match stack {
            TechStack::Unknown => {},
            _ => panic!("Expected Unknown stack, got {:?}", stack),
        }
    }

    #[test]
    fn test_find_rust_entry_point() {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = create_test_project(&[
            ("Cargo.toml", "[package]\nname = \"test\""),
        ]);
        
        let stack = TechStack::Rust { edition: None };
        let config_files = Vec::new();
        let entry_point = analyzer.find_entry_point(&stack, &config_files, temp_dir.path()).unwrap();
        
        assert_eq!(entry_point, Some("cargo run".to_string()));
    }

    #[test]
    fn test_find_go_entry_point() {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = create_test_project(&[
            ("go.mod", "module test"),
        ]);
        
        let stack = TechStack::Go { version: None };
        let config_files = Vec::new();
        let entry_point = analyzer.find_entry_point(&stack, &config_files, temp_dir.path()).unwrap();
        
        assert_eq!(entry_point, Some("go run .".to_string()));
    }

    #[test]
    fn test_analyze_project_full() {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = create_test_project(&[
            ("package.json", r#"{
                "name": "test-app",
                "scripts": {"start": "node server.js"},
                "dependencies": {"express": "^4.18.0"}
            }"#),
            ("server.js", "const express = require('express');"),
        ]);
        
        let project_info = analyzer.analyze_project(temp_dir.path()).unwrap();
        
        match project_info.stack {
            TechStack::NodeJs { .. } => {},
            _ => panic!("Expected NodeJs stack"),
        }
        
        assert_eq!(project_info.entry_command, Some("node server.js".to_string()));
        assert_eq!(project_info.dependencies.len(), 1);
        assert_eq!(project_info.config_files.len(), 1);
    }

    #[test]
    fn test_skip_node_modules_directory() {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = create_test_project(&[
            ("package.json", r#"{"name": "test"}"#),
            ("node_modules/express/package.json", r#"{"name": "express"}"#),
        ]);
        
        let files = analyzer.scan_directory(temp_dir.path()).unwrap();
        
        // Проверяем, что файлы из node_modules не включены
        let has_node_modules = files.iter().any(|f| f.to_str().unwrap().contains("node_modules"));
        assert!(!has_node_modules, "node_modules should be skipped");
    }
}
}

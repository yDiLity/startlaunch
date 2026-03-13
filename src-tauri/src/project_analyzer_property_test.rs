use crate::project_analyzer::ProjectAnalyzer;
use crate::models::TechStack;
use proptest::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// **Feature: autolaunch-core, Property 4: Детекция стека технологий**
/// 
/// Для любой структуры файлов проекта, система должна корректно определить 
/// основной стек технологий на основе присутствующих конфигурационных файлов.
/// 
/// Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5

// Генератор для имен файлов конфигурации
fn config_file_strategy() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(
        prop_oneof![
            Just("package.json".to_string()),
            Just("requirements.txt".to_string()),
            Just("pyproject.toml".to_string()),
            Just("setup.py".to_string()),
            Just("Cargo.toml".to_string()),
            Just("go.mod".to_string()),
            Just("pom.xml".to_string()),
            Just("build.gradle".to_string()),
            Just("Dockerfile".to_string()),
            Just("docker-compose.yml".to_string()),
            Just("docker-compose.yaml".to_string()),
            Just("index.html".to_string()),
            Just("style.css".to_string()),
            Just("script.js".to_string()),
            Just("README.md".to_string()),
        ],
        0..10,
    )
}

// Вспомогательная функция для создания временного проекта с файлами
fn create_temp_project_with_files(files: &[String]) -> TempDir {
    let temp_dir = TempDir::new().unwrap();
    
    for file_name in files {
        let file_path = temp_dir.path().join(file_name);
        
        // Создаем директории если нужно
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        
        // Создаем файл с минимальным содержимым
        let content = match file_name.as_str() {
            "package.json" => r#"{"name": "test", "version": "1.0.0"}"#,
            "requirements.txt" => "flask==2.0.0",
            "pyproject.toml" => "[tool.poetry]\nname = \"test\"",
            "setup.py" => "from setuptools import setup\nsetup(name='test')",
            "Cargo.toml" => "[package]\nname = \"test\"\nversion = \"0.1.0\"",
            "go.mod" => "module test\n\ngo 1.21",
            "pom.xml" => "<project></project>",
            "build.gradle" => "plugins { id 'java' }",
            "Dockerfile" => "FROM node:18",
            "docker-compose.yml" | "docker-compose.yaml" => "version: '3'\nservices:\n  web:\n    build: .",
            "index.html" => "<!DOCTYPE html><html></html>",
            "style.css" => "body { margin: 0; }",
            "script.js" => "console.log('test');",
            _ => "test content",
        };
        
        fs::write(file_path, content).ok();
    }
    
    temp_dir
}

// Функция для проверки корректности детекции стека
fn verify_stack_detection(files: &[String], detected_stack: &TechStack) -> bool {
    // Приоритет детекции (первый найденный файл определяет стек)
    
    // 1. Node.js - package.json
    if files.iter().any(|f| f == "package.json") {
        return matches!(detected_stack, TechStack::NodeJs { .. });
    }
    
    // 2. Python - requirements.txt, pyproject.toml, setup.py
    if files.iter().any(|f| f == "requirements.txt" || f == "pyproject.toml" || f == "setup.py") {
        return matches!(detected_stack, TechStack::Python { .. });
    }
    
    // 3. Rust - Cargo.toml
    if files.iter().any(|f| f == "Cargo.toml") {
        return matches!(detected_stack, TechStack::Rust { .. });
    }
    
    // 4. Go - go.mod
    if files.iter().any(|f| f == "go.mod") {
        return matches!(detected_stack, TechStack::Go { .. });
    }
    
    // 5. Java - pom.xml, build.gradle
    if files.iter().any(|f| f == "pom.xml" || f == "build.gradle") {
        return matches!(detected_stack, TechStack::Java { .. });
    }
    
    // 6. Docker - Dockerfile
    if files.iter().any(|f| f == "Dockerfile") {
        return matches!(detected_stack, TechStack::Docker { compose: false });
    }
    
    // 7. Docker Compose - docker-compose.yml/yaml
    if files.iter().any(|f| f == "docker-compose.yml" || f == "docker-compose.yaml") {
        return matches!(detected_stack, TechStack::Docker { compose: true });
    }
    
    // 8. Static - HTML/CSS/JS файлы
    let has_html = files.iter().any(|f| f == "index.html");
    let has_css = files.iter().any(|f| f.ends_with(".css"));
    let has_js = files.iter().any(|f| f.ends_with(".js"));
    
    if has_html || (has_css && has_js) {
        return matches!(detected_stack, TechStack::Static { .. });
    }
    
    // 9. Unknown - нет известных конфигурационных файлов
    matches!(detected_stack, TechStack::Unknown)
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    /// Property Test: Детекция стека технологий
    /// 
    /// Для любой комбинации конфигурационных файлов, система должна:
    /// 1. Корректно определить стек на основе присутствующих файлов
    /// 2. Следовать приоритету детекции (Node.js > Python > Rust > Go > Java > Docker > Static > Unknown)
    /// 3. Возвращать Unknown только если нет известных конфигурационных файлов
    #[test]
    fn prop_detect_stack_from_config_files(files in config_file_strategy()) {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = create_temp_project_with_files(&files);
        
        // Сканируем директорию
        let scanned_files = analyzer.scan_directory(temp_dir.path())
            .expect("Failed to scan directory");
        
        // Детектируем стек
        let detected_stack = analyzer.detect_stack(&scanned_files);
        
        // Проверяем корректность детекции
        prop_assert!(
            verify_stack_detection(&files, &detected_stack),
            "Stack detection failed for files: {:?}, detected: {:?}",
            files,
            detected_stack
        );
    }
    
    /// Property Test: Node.js детекция
    /// 
    /// Для любого проекта с package.json, система должна определить стек как Node.js
    #[test]
    fn prop_detect_nodejs_with_package_json(
        extra_files in prop::collection::vec(
            prop_oneof![
                Just("README.md".to_string()),
                Just("index.js".to_string()),
                Just("app.js".to_string()),
            ],
            0..5
        )
    ) {
        let analyzer = ProjectAnalyzer::new();
        let mut files = vec!["package.json".to_string()];
        files.extend(extra_files);
        
        let temp_dir = create_temp_project_with_files(&files);
        let scanned_files = analyzer.scan_directory(temp_dir.path()).unwrap();
        let detected_stack = analyzer.detect_stack(&scanned_files);
        
        prop_assert!(
            matches!(detected_stack, TechStack::NodeJs { .. }),
            "Expected NodeJs stack for package.json, got {:?}",
            detected_stack
        );
    }
    
    /// Property Test: Python детекция
    /// 
    /// Для любого проекта с requirements.txt или pyproject.toml, 
    /// система должна определить стек как Python
    #[test]
    fn prop_detect_python_with_config(
        python_config in prop_oneof![
            Just("requirements.txt".to_string()),
            Just("pyproject.toml".to_string()),
            Just("setup.py".to_string()),
        ],
        extra_files in prop::collection::vec(
            prop_oneof![
                Just("main.py".to_string()),
                Just("app.py".to_string()),
                Just("README.md".to_string()),
            ],
            0..5
        )
    ) {
        let analyzer = ProjectAnalyzer::new();
        let mut files = vec![python_config];
        files.extend(extra_files);
        
        let temp_dir = create_temp_project_with_files(&files);
        let scanned_files = analyzer.scan_directory(temp_dir.path()).unwrap();
        let detected_stack = analyzer.detect_stack(&scanned_files);
        
        prop_assert!(
            matches!(detected_stack, TechStack::Python { .. }),
            "Expected Python stack, got {:?}",
            detected_stack
        );
    }
    
    /// Property Test: Rust детекция
    /// 
    /// Для любого проекта с Cargo.toml, система должна определить стек как Rust
    #[test]
    fn prop_detect_rust_with_cargo_toml(
        extra_files in prop::collection::vec(
            prop_oneof![
                Just("src/main.rs".to_string()),
                Just("src/lib.rs".to_string()),
                Just("README.md".to_string()),
            ],
            0..5
        )
    ) {
        let analyzer = ProjectAnalyzer::new();
        let mut files = vec!["Cargo.toml".to_string()];
        files.extend(extra_files);
        
        let temp_dir = create_temp_project_with_files(&files);
        let scanned_files = analyzer.scan_directory(temp_dir.path()).unwrap();
        let detected_stack = analyzer.detect_stack(&scanned_files);
        
        prop_assert!(
            matches!(detected_stack, TechStack::Rust { .. }),
            "Expected Rust stack for Cargo.toml, got {:?}",
            detected_stack
        );
    }
    
    /// Property Test: Docker детекция
    /// 
    /// Для любого проекта с Dockerfile, система должна определить стек как Docker
    #[test]
    fn prop_detect_docker_with_dockerfile(
        extra_files in prop::collection::vec(
            Just("README.md".to_string()),
            0..3
        )
    ) {
        let analyzer = ProjectAnalyzer::new();
        let mut files = vec!["Dockerfile".to_string()];
        files.extend(extra_files);
        
        let temp_dir = create_temp_project_with_files(&files);
        let scanned_files = analyzer.scan_directory(temp_dir.path()).unwrap();
        let detected_stack = analyzer.detect_stack(&scanned_files);
        
        prop_assert!(
            matches!(detected_stack, TechStack::Docker { compose: false }),
            "Expected Docker stack for Dockerfile, got {:?}",
            detected_stack
        );
    }
    
    /// Property Test: Docker Compose детекция
    /// 
    /// Для любого проекта с docker-compose.yml, система должна определить стек как Docker с compose
    #[test]
    fn prop_detect_docker_compose(
        compose_file in prop_oneof![
            Just("docker-compose.yml".to_string()),
            Just("docker-compose.yaml".to_string()),
        ]
    ) {
        let analyzer = ProjectAnalyzer::new();
        let files = vec![compose_file];
        
        let temp_dir = create_temp_project_with_files(&files);
        let scanned_files = analyzer.scan_directory(temp_dir.path()).unwrap();
        let detected_stack = analyzer.detect_stack(&scanned_files);
        
        prop_assert!(
            matches!(detected_stack, TechStack::Docker { compose: true }),
            "Expected Docker Compose stack, got {:?}",
            detected_stack
        );
    }
    
    /// Property Test: Unknown стек
    /// 
    /// Для проекта без известных конфигурационных файлов, 
    /// система должна вернуть Unknown
    #[test]
    fn prop_detect_unknown_stack_without_config(
        readme_files in prop::collection::vec(
            prop_oneof![
                Just("README.md".to_string()),
                Just("LICENSE".to_string()),
                Just(".gitignore".to_string()),
            ],
            0..5
        )
    ) {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = create_temp_project_with_files(&readme_files);
        let scanned_files = analyzer.scan_directory(temp_dir.path()).unwrap();
        let detected_stack = analyzer.detect_stack(&scanned_files);
        
        prop_assert!(
            matches!(detected_stack, TechStack::Unknown),
            "Expected Unknown stack for non-config files, got {:?}",
            detected_stack
        );
    }
}

/// **Feature: autolaunch-core, Property 5: Извлечение команд запуска**
/// 
/// Для любого конфигурационного файла (package.json, pyproject.toml и т.д.), 
/// система должна корректно извлечь команду запуска из соответствующих полей.
/// 
/// Validates: Requirements 3.1, 3.2, 3.3

// Генератор для команд запуска в package.json
fn package_json_start_command_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("node index.js".to_string()),
        Just("npm run dev".to_string()),
        Just("npm start".to_string()),
        Just("node server.js".to_string()),
        Just("nodemon app.js".to_string()),
        "[a-z]{3,10} [a-z]{3,10}\\.js".prop_map(|s| s),
    ]
}

// Генератор для Python entry файлов
fn python_entry_file_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("main.py".to_string()),
        Just("app.py".to_string()),
    ]
}

// Вспомогательная функция для создания package.json с командой start
fn create_package_json_content(start_command: &str) -> String {
    format!(
        r#"{{
    "name": "test-project",
    "version": "1.0.0",
    "scripts": {{
        "start": "{}"
    }}
}}"#,
        start_command
    )
}

// Вспомогательная функция для проверки корректности извлечения команды
fn verify_entry_command(
    stack: &TechStack,
    config_files: &[(&str, &str)],
    expected_command: Option<&str>,
) -> bool {
    let temp_dir = TempDir::new().unwrap();
    
    // Создаем файлы конфигурации
    for (file_name, content) in config_files {
        let file_path = temp_dir.path().join(file_name);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).ok();
        }
        fs::write(file_path, content).ok();
    }
    
    let analyzer = ProjectAnalyzer::new();
    let scanned_files = analyzer.scan_directory(temp_dir.path()).unwrap();
    let config_files_vec = analyzer.find_config_files(&scanned_files);
    
    let entry_command = analyzer
        .find_entry_point(stack, &config_files_vec, temp_dir.path())
        .unwrap();
    
    match (entry_command.as_deref(), expected_command) {
        (Some(cmd), Some(expected)) => cmd == expected,
        (None, None) => true,
        _ => false,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    /// Property Test: Извлечение команды запуска из package.json
    /// 
    /// Для любого package.json с полем scripts.start, система должна:
    /// 1. Корректно извлечь команду из поля scripts.start
    /// 2. Вернуть точное значение команды без изменений
    #[test]
    fn prop_extract_nodejs_start_command(start_command in package_json_start_command_strategy()) {
        let analyzer = ProjectAnalyzer::new();
        let package_json_content = create_package_json_content(&start_command);
        
        let temp_dir = TempDir::new().unwrap();
        let package_json_path = temp_dir.path().join("package.json");
        fs::write(&package_json_path, &package_json_content).unwrap();
        
        let scanned_files = analyzer.scan_directory(temp_dir.path()).unwrap();
        let config_files = analyzer.find_config_files(&scanned_files);
        let stack = TechStack::NodeJs { version: None };
        
        let entry_command = analyzer
            .find_entry_point(&stack, &config_files, temp_dir.path())
            .unwrap();
        
        prop_assert_eq!(
            entry_command,
            Some(start_command.clone()),
            "Expected start command '{}' to be extracted from package.json",
            start_command
        );
    }
    
    /// Property Test: Fallback на index.js для Node.js проектов
    /// 
    /// Для Node.js проекта без package.json или без scripts.start,
    /// но с файлом index.js, система должна предложить "node index.js"
    #[test]
    fn prop_nodejs_fallback_to_index_js(
        has_package_json in prop::bool::ANY,
        has_start_script in prop::bool::ANY,
    ) {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Создаем index.js
        fs::write(temp_dir.path().join("index.js"), "console.log('test');").unwrap();
        
        // Создаем package.json если нужно
        if has_package_json {
            let package_json_content = if has_start_script {
                r#"{"name": "test", "scripts": {"start": "npm run dev"}}"#
            } else {
                r#"{"name": "test", "version": "1.0.0"}"#
            };
            fs::write(temp_dir.path().join("package.json"), package_json_content).unwrap();
        }
        
        let scanned_files = analyzer.scan_directory(temp_dir.path()).unwrap();
        let config_files = analyzer.find_config_files(&scanned_files);
        let stack = TechStack::NodeJs { version: None };
        
        let entry_command = analyzer
            .find_entry_point(&stack, &config_files, temp_dir.path())
            .unwrap();
        
        // Если есть package.json с start script, используем его
        // Иначе используем fallback на index.js
        if has_package_json && has_start_script {
            prop_assert_eq!(entry_command, Some("npm run dev".to_string()));
        } else {
            prop_assert_eq!(entry_command, Some("node index.js".to_string()));
        }
    }
    
    /// Property Test: Извлечение команды для Python проектов
    /// 
    /// Для Python проекта с main.py или app.py, система должна:
    /// 1. Предложить "python main.py" если существует main.py
    /// 2. Предложить "python app.py" если существует app.py (и нет main.py)
    #[test]
    fn prop_extract_python_entry_command(
        entry_file in python_entry_file_strategy(),
        has_other_files in prop::bool::ANY,
    ) {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Создаем entry файл
        fs::write(temp_dir.path().join(&entry_file), "print('test')").unwrap();
        
        // Опционально создаем другие файлы
        if has_other_files {
            fs::write(temp_dir.path().join("requirements.txt"), "flask==2.0.0").unwrap();
        }
        
        let stack = TechStack::Python { version: None };
        let config_files = vec![];
        
        let entry_command = analyzer
            .find_entry_point(&stack, &config_files, temp_dir.path())
            .unwrap();
        
        let expected_command = format!("python {}", entry_file);
        prop_assert_eq!(
            entry_command,
            Some(expected_command.clone()),
            "Expected '{}' for Python project with {}",
            expected_command,
            entry_file
        );
    }
    
    /// Property Test: Приоритет main.py над app.py
    /// 
    /// Для Python проекта с обоими файлами main.py и app.py,
    /// система должна предпочесть main.py
    #[test]
    fn prop_python_main_py_priority(has_requirements in prop::bool::ANY) {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = TempDir::new().unwrap();
        
        // Создаем оба файла
        fs::write(temp_dir.path().join("main.py"), "print('main')").unwrap();
        fs::write(temp_dir.path().join("app.py"), "print('app')").unwrap();
        
        if has_requirements {
            fs::write(temp_dir.path().join("requirements.txt"), "flask==2.0.0").unwrap();
        }
        
        let stack = TechStack::Python { version: None };
        let config_files = vec![];
        
        let entry_command = analyzer
            .find_entry_point(&stack, &config_files, temp_dir.path())
            .unwrap();
        
        prop_assert_eq!(
            entry_command,
            Some("python main.py".to_string()),
            "Expected 'python main.py' to have priority over app.py"
        );
    }
    
    /// Property Test: Команды для других стеков
    /// 
    /// Для различных стеков технологий, система должна возвращать
    /// соответствующие команды запуска по умолчанию
    #[test]
    fn prop_extract_default_commands_for_stacks(
        stack in prop_oneof![
            Just(TechStack::Rust { edition: None }),
            Just(TechStack::Go { version: None }),
            Just(TechStack::Docker { compose: true }),
        ]
    ) {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = TempDir::new().unwrap();
        
        let config_files = vec![];
        let entry_command = analyzer
            .find_entry_point(&stack, &config_files, temp_dir.path())
            .unwrap();
        
        let expected = match stack {
            TechStack::Rust { .. } => Some("cargo run".to_string()),
            TechStack::Go { .. } => Some("go run .".to_string()),
            TechStack::Docker { compose: true } => Some("docker-compose up".to_string()),
            _ => None,
        };
        
        prop_assert_eq!(
            entry_command,
            expected,
            "Expected correct default command for stack {:?}",
            stack
        );
    }
    
    /// Property Test: Отсутствие команды для неизвестных стеков
    /// 
    /// Для неизвестных или статических стеков без явной конфигурации,
    /// система должна вернуть None
    #[test]
    fn prop_no_command_for_unknown_stack(
        stack in prop_oneof![
            Just(TechStack::Unknown),
            Just(TechStack::Static { framework: None }),
        ]
    ) {
        let analyzer = ProjectAnalyzer::new();
        let temp_dir = TempDir::new().unwrap();
        
        let config_files = vec![];
        let entry_command = analyzer
            .find_entry_point(&stack, &config_files, temp_dir.path())
            .unwrap();
        
        prop_assert_eq!(
            entry_command,
            None,
            "Expected None for stack {:?}",
            stack
        );
    }
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    
    #[test]
    fn test_verify_stack_detection_nodejs() {
        let files = vec!["package.json".to_string()];
        let stack = TechStack::NodeJs { version: None };
        assert!(verify_stack_detection(&files, &stack));
    }
    
    #[test]
    fn test_verify_stack_detection_python() {
        let files = vec!["requirements.txt".to_string()];
        let stack = TechStack::Python { version: None };
        assert!(verify_stack_detection(&files, &stack));
    }
    
    #[test]
    fn test_verify_stack_detection_rust() {
        let files = vec!["Cargo.toml".to_string()];
        let stack = TechStack::Rust { edition: None };
        assert!(verify_stack_detection(&files, &stack));
    }
    
    #[test]
    fn test_verify_stack_detection_docker() {
        let files = vec!["Dockerfile".to_string()];
        let stack = TechStack::Docker { compose: false };
        assert!(verify_stack_detection(&files, &stack));
    }
    
    #[test]
    fn test_verify_stack_detection_unknown() {
        let files = vec!["README.md".to_string()];
        let stack = TechStack::Unknown;
        assert!(verify_stack_detection(&files, &stack));
    }
    
    #[test]
    fn test_create_package_json_content() {
        let content = create_package_json_content("npm run dev");
        assert!(content.contains("\"start\": \"npm run dev\""));
    }
    
    #[test]
    fn test_verify_entry_command_nodejs() {
        let stack = TechStack::NodeJs { version: None };
        let config_files = vec![(
            "package.json",
            r#"{"name": "test", "scripts": {"start": "node index.js"}}"#,
        )];
        
        assert!(verify_entry_command(&stack, &config_files, Some("node index.js")));
    }
    
    #[test]
    fn test_verify_entry_command_python() {
        let stack = TechStack::Python { version: None };
        let config_files = vec![("main.py", "print('test')")];
        
        assert!(verify_entry_command(&stack, &config_files, Some("python main.py")));
    }
}

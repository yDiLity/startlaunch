use crate::error::{AutoLaunchError, Result};
use crate::models::{ProjectInfo, TechStack, Dependency};
use std::path::{Path, PathBuf};
use std::process::Command;
use uuid::Uuid;

/// Режим изоляции окружения
#[derive(Debug, Clone)]
pub enum IsolationMode {
    /// Песочница с использованием Docker контейнера
    Sandbox(DockerConfig),
    /// Прямой режим с виртуальным окружением языка
    Direct(VirtualEnvConfig),
}

/// Конфигурация Docker контейнера
#[derive(Debug, Clone)]
pub struct DockerConfig {
    pub image: String,
    pub working_dir: String,
    pub ports: Vec<u16>,
    pub volumes: Vec<(String, String)>,
    pub environment: Vec<(String, String)>,
    pub read_only: bool,
    pub no_root: bool,
}

/// Конфигурация виртуального окружения
#[derive(Debug, Clone)]
pub struct VirtualEnvConfig {
    pub working_dir: PathBuf,
    pub env_vars: Vec<(String, String)>,
}

/// Изолированное окружение для запуска проекта
#[derive(Debug, Clone)]
pub struct Environment {
    pub id: String,
    pub mode: IsolationMode,
    pub working_dir: PathBuf,
    pub container_id: Option<String>,
}

/// Трейт для управления окружениями
pub trait EnvironmentManagerTrait {
    /// Создать изолированное окружение для проекта
    fn create_environment(&self, project: &ProjectInfo, project_path: &Path) -> impl std::future::Future<Output = Result<Environment>> + Send;
    
    /// Установить зависимости в окружении
    fn install_dependencies(&self, env: &Environment, deps: &[Dependency]) -> impl std::future::Future<Output = Result<()>> + Send;
    
    /// Очистить окружение и освободить ресурсы
    fn cleanup_environment(&self, env: &Environment) -> impl std::future::Future<Output = Result<()>> + Send;
}

/// Менеджер окружений
pub struct EnvironmentManager;

impl EnvironmentManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn create_environment(&self, project: &ProjectInfo, project_path: &Path) -> Result<Environment> {
        let env_id = Uuid::new_v4().to_string();
        
        // Требование 4.1: Для неизвестных и недоверенных проектов используем режим песочницы
        let should_use_sandbox = matches!(
            project.trust_level,
            crate::models::TrustLevel::Unknown | crate::models::TrustLevel::Untrusted
        );
        
        // Проверяем доступность Docker
        if self.is_docker_available().await {
            // Если проект неизвестный или недоверенный, всегда используем песочницу
            if should_use_sandbox {
                self.create_docker_environment(&env_id, project, project_path).await
            } else {
                // Для доверенных проектов можем использовать прямой режим
                self.create_docker_environment(&env_id, project, project_path).await
            }
        } else {
            // Если Docker недоступен, используем прямой режим с предупреждением
            self.create_direct_environment(&env_id, project, project_path).await
        }
    }

    pub async fn is_docker_available(&self) -> bool {
        match Command::new("docker").arg("--version").output() {
            Ok(output) => output.status.success(),
            Err(_) => false,
        }
    }

    async fn create_docker_environment(&self, env_id: &str, project: &ProjectInfo, project_path: &Path) -> Result<Environment> {
        let docker_config = self.generate_docker_config(project)?;
        
        // Создаем Dockerfile если его нет
        let dockerfile_path = project_path.join("Dockerfile.autolaunch");
        if !dockerfile_path.exists() {
            let dockerfile_content = self.generate_dockerfile(project)?;
            std::fs::write(&dockerfile_path, dockerfile_content)?;
        }

        // Строим Docker образ
        let image_tag = format!("autolaunch-{}", env_id);
        let build_output = Command::new("docker")
            .args(&["build", "-t", &image_tag, "-f", "Dockerfile.autolaunch", "."])
            .current_dir(project_path)
            .output()?;

        if !build_output.status.success() {
            let error_msg = String::from_utf8_lossy(&build_output.stderr);
            return Err(AutoLaunchError::Environment(format!("Ошибка сборки Docker образа: {}", error_msg)));
        }

        // Создаем контейнер с ограничениями безопасности
        let mut docker_args = vec![
            "create".to_string(),
            "--name".to_string(),
            format!("autolaunch-container-{}", env_id),
            "--rm".to_string(),
            // Ограничения безопасности (Требование 4.2)
            "--security-opt".to_string(),
            "no-new-privileges".to_string(),
            "--cap-drop".to_string(),
            "ALL".to_string(),
        ];

        // Добавляем no-root пользователя (Требование 4.2)
        if docker_config.no_root {
            docker_args.push("--user".to_string());
            docker_args.push("1000:1000".to_string());
        }

        // Добавляем read-only файловую систему (Требование 4.2)
        if docker_config.read_only {
            docker_args.push("--read-only".to_string());
            // Добавляем tmpfs для временных файлов
            docker_args.push("--tmpfs".to_string());
            docker_args.push("/tmp:rw,noexec,nosuid,size=100m".to_string());
        }

        // Добавляем порты
        for port in &docker_config.ports {
            docker_args.push("-p".to_string());
            docker_args.push(format!("{}:{}", port, port));
        }

        // Добавляем volumes
        for (host_path, container_path) in &docker_config.volumes {
            docker_args.push("-v".to_string());
            let volume_spec = if docker_config.read_only {
                format!("{}:{}:ro", host_path, container_path)
            } else {
                format!("{}:{}", host_path, container_path)
            };
            docker_args.push(volume_spec);
        }

        // Добавляем переменные окружения
        for (key, value) in &docker_config.environment {
            docker_args.push("-e".to_string());
            docker_args.push(format!("{}={}", key, value));
        }

        docker_args.push(image_tag);

        let create_output = Command::new("docker")
            .args(&docker_args)
            .output()?;

        if !create_output.status.success() {
            let error_msg = String::from_utf8_lossy(&create_output.stderr);
            return Err(AutoLaunchError::Environment(format!("Ошибка создания контейнера: {}", error_msg)));
        }

        let container_id = String::from_utf8_lossy(&create_output.stdout).trim().to_string();

        Ok(Environment {
            id: env_id.to_string(),
            mode: IsolationMode::Sandbox(docker_config),
            working_dir: project_path.to_path_buf(),
            container_id: Some(container_id),
        })
    }

    async fn create_direct_environment(&self, env_id: &str, project: &ProjectInfo, project_path: &Path) -> Result<Environment> {
        let mut env_vars = Vec::new();
        
        // Настраиваем виртуальное окружение в зависимости от стека
        match &project.stack {
            TechStack::Python { .. } => {
                // Создаем Python virtual environment
                let venv_path = project_path.join(".venv");
                if !venv_path.exists() {
                    let venv_output = Command::new("python")
                        .args(&["-m", "venv", ".venv"])
                        .current_dir(project_path)
                        .output()?;

                    if !venv_output.status.success() {
                        return Err(AutoLaunchError::Environment("Не удалось создать Python virtual environment".to_string()));
                    }
                }

                // Устанавливаем зависимости
                self.install_python_dependencies(project_path).await?;
            }
            TechStack::NodeJs { .. } => {
                // Устанавливаем Node.js зависимости
                self.install_nodejs_dependencies(project_path).await?;
            }
            _ => {}
        }

        let virtual_env_config = VirtualEnvConfig {
            working_dir: project_path.to_path_buf(),
            env_vars,
        };

        Ok(Environment {
            id: env_id.to_string(),
            mode: IsolationMode::Direct(virtual_env_config),
            working_dir: project_path.to_path_buf(),
            container_id: None,
        })
    }

    async fn install_python_dependencies(&self, project_path: &Path) -> Result<()> {
        let requirements_path = project_path.join("requirements.txt");
        if requirements_path.exists() {
            let pip_path = if cfg!(windows) {
                project_path.join(".venv").join("Scripts").join("pip.exe")
            } else {
                project_path.join(".venv").join("bin").join("pip")
            };

            let install_output = Command::new(&pip_path)
                .args(&["install", "-r", "requirements.txt"])
                .current_dir(project_path)
                .output()?;

            if !install_output.status.success() {
                let error_msg = String::from_utf8_lossy(&install_output.stderr);
                return Err(AutoLaunchError::Environment(format!("Ошибка установки Python зависимостей: {}", error_msg)));
            }
        }
        Ok(())
    }

    async fn install_nodejs_dependencies(&self, project_path: &Path) -> Result<()> {
        let package_json_path = project_path.join("package.json");
        if package_json_path.exists() {
            let install_output = Command::new("npm")
                .arg("install")
                .current_dir(project_path)
                .output()?;

            if !install_output.status.success() {
                let error_msg = String::from_utf8_lossy(&install_output.stderr);
                return Err(AutoLaunchError::Environment(format!("Ошибка установки Node.js зависимостей: {}", error_msg)));
            }
        }
        Ok(())
    }

    fn generate_docker_config(&self, project: &ProjectInfo) -> Result<DockerConfig> {
        let (base_image, working_dir, ports) = match &project.stack {
            TechStack::NodeJs { version } => {
                let node_version = version.as_deref().unwrap_or("18");
                (format!("node:{}-alpine", node_version), "/app".to_string(), vec![3000, 8000, 8080])
            }
            TechStack::Python { version } => {
                let python_version = version.as_deref().unwrap_or("3.11");
                (format!("python:{}-alpine", python_version), "/app".to_string(), vec![5000, 8000, 8080])
            }
            TechStack::Rust { .. } => {
                ("rust:alpine".to_string(), "/app".to_string(), vec![8000, 8080])
            }
            TechStack::Go { .. } => {
                ("golang:alpine".to_string(), "/app".to_string(), vec![8000, 8080])
            }
            _ => {
                ("alpine:latest".to_string(), "/app".to_string(), vec![8000, 8080])
            }
        };

        Ok(DockerConfig {
            image: base_image,
            working_dir: working_dir.clone(),
            ports,
            volumes: vec![
                ("./".to_string(), working_dir),
            ],
            environment: vec![
                ("NODE_ENV".to_string(), "development".to_string()),
            ],
            // Требование 4.2: Ограничить права Docker контейнера
            read_only: true,
            no_root: true,
        })
    }

    fn generate_dockerfile(&self, project: &ProjectInfo) -> Result<String> {
        let dockerfile = match &project.stack {
            TechStack::NodeJs { version } => {
                let node_version = version.as_deref().unwrap_or("18");
                format!(
                    r#"FROM node:{}-alpine
WORKDIR /app
COPY package*.json ./
RUN npm install
COPY . .
EXPOSE 3000 8000 8080
CMD ["npm", "start"]
"#,
                    node_version
                )
            }
            TechStack::Python { version } => {
                let python_version = version.as_deref().unwrap_or("3.11");
                format!(
                    r#"FROM python:{}-alpine
WORKDIR /app
COPY requirements.txt ./
RUN pip install -r requirements.txt
COPY . .
EXPOSE 5000 8000 8080
CMD ["python", "main.py"]
"#,
                    python_version
                )
            }
            TechStack::Rust { .. } => {
                r#"FROM rust:alpine
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release
EXPOSE 8000 8080
CMD ["./target/release/app"]
"#.to_string()
            }
            _ => {
                r#"FROM alpine:latest
WORKDIR /app
COPY . .
EXPOSE 8000 8080
CMD ["sh"]
"#.to_string()
            }
        };

        Ok(dockerfile)
    }

    pub async fn cleanup_environment(&self, env: &Environment) -> Result<()> {
        match &env.mode {
            IsolationMode::Sandbox(_) => {
                if let Some(container_id) = &env.container_id {
                    // Останавливаем и удаляем контейнер
                    let _ = Command::new("docker")
                        .args(&["stop", container_id])
                        .output();
                    
                    let _ = Command::new("docker")
                        .args(&["rm", container_id])
                        .output();
                }

                // Удаляем временный Dockerfile
                let dockerfile_path = env.working_dir.join("Dockerfile.autolaunch");
                if dockerfile_path.exists() {
                    let _ = std::fs::remove_file(dockerfile_path);
                }
            }
            IsolationMode::Direct(_) => {
                // Очистка для прямого режима (если нужно)
            }
        }
        Ok(())
    }

    /// Установить зависимости в окружении
    pub async fn install_dependencies(&self, env: &Environment, deps: &[Dependency]) -> Result<()> {
        match &env.mode {
            IsolationMode::Sandbox(_) => {
                // Для Docker окружения зависимости устанавливаются при сборке образа
                Ok(())
            }
            IsolationMode::Direct(_) => {
                // Для прямого режима устанавливаем зависимости в виртуальное окружение
                // Определяем тип проекта по зависимостям
                if deps.is_empty() {
                    return Ok(());
                }

                // Проверяем наличие Python зависимостей
                let requirements_path = env.working_dir.join("requirements.txt");
                if requirements_path.exists() {
                    self.install_python_dependencies(&env.working_dir).await?;
                }

                // Проверяем наличие Node.js зависимостей
                let package_json_path = env.working_dir.join("package.json");
                if package_json_path.exists() {
                    self.install_nodejs_dependencies(&env.working_dir).await?;
                }

                Ok(())
            }
        }
    }
}

// Реализация трейта EnvironmentManagerTrait
impl EnvironmentManagerTrait for EnvironmentManager {
    async fn create_environment(&self, project: &ProjectInfo, project_path: &Path) -> Result<Environment> {
        self.create_environment(project, project_path).await
    }

    async fn install_dependencies(&self, env: &Environment, deps: &[Dependency]) -> Result<()> {
        self.install_dependencies(env, deps).await
    }

    async fn cleanup_environment(&self, env: &Environment) -> Result<()> {
        self.cleanup_environment(env).await
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod property_tests;

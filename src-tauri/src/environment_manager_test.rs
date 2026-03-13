#[cfg(test)]
mod tests {
    use super::super::environment_manager::*;
    use crate::models::{ProjectInfo, TechStack, Dependency, ConfigFile, SecurityWarning};
    use std::path::PathBuf;

    fn create_test_project_info(stack: TechStack) -> ProjectInfo {
        ProjectInfo {
            stack,
            entry_command: Some("npm start".to_string()),
            dependencies: vec![],
            config_files: vec![],
            security_warnings: vec![],
            trust_level: crate::models::TrustLevel::Unknown,
        }
    }

    #[test]
    fn test_docker_config_has_security_restrictions() {
        let manager = EnvironmentManager::new();
        let project = create_test_project_info(TechStack::NodeJs { version: Some("18".to_string()) });
        
        let config = manager.generate_docker_config(&project).unwrap();
        
        // Требование 4.2: Проверяем что включены ограничения безопасности
        assert!(config.no_root, "Docker контейнер должен работать без root прав");
        assert!(config.read_only, "Docker контейнер должен иметь read-only файловую систему");
    }

    #[test]
    fn test_docker_config_for_nodejs() {
        let manager = EnvironmentManager::new();
        let project = create_test_project_info(TechStack::NodeJs { version: Some("18".to_string()) });
        
        let config = manager.generate_docker_config(&project).unwrap();
        
        assert!(config.image.contains("node:18"));
        assert_eq!(config.working_dir, "/app");
        assert!(config.ports.contains(&3000));
    }

    #[test]
    fn test_docker_config_for_python() {
        let manager = EnvironmentManager::new();
        let project = create_test_project_info(TechStack::Python { version: Some("3.11".to_string()) });
        
        let config = manager.generate_docker_config(&project).unwrap();
        
        assert!(config.image.contains("python:3.11"));
        assert_eq!(config.working_dir, "/app");
        assert!(config.ports.contains(&5000));
    }

    #[test]
    fn test_docker_config_for_rust() {
        let manager = EnvironmentManager::new();
        let project = create_test_project_info(TechStack::Rust { edition: Some("2021".to_string()) });
        
        let config = manager.generate_docker_config(&project).unwrap();
        
        assert!(config.image.contains("rust"));
        assert_eq!(config.working_dir, "/app");
    }

    #[test]
    fn test_generate_dockerfile_nodejs() {
        let manager = EnvironmentManager::new();
        let project = create_test_project_info(TechStack::NodeJs { version: Some("18".to_string()) });
        
        let dockerfile = manager.generate_dockerfile(&project).unwrap();
        
        assert!(dockerfile.contains("FROM node:18-alpine"));
        assert!(dockerfile.contains("WORKDIR /app"));
        assert!(dockerfile.contains("npm install"));
        assert!(dockerfile.contains("EXPOSE"));
    }

    #[test]
    fn test_generate_dockerfile_python() {
        let manager = EnvironmentManager::new();
        let project = create_test_project_info(TechStack::Python { version: Some("3.11".to_string()) });
        
        let dockerfile = manager.generate_dockerfile(&project).unwrap();
        
        assert!(dockerfile.contains("FROM python:3.11-alpine"));
        assert!(dockerfile.contains("WORKDIR /app"));
        assert!(dockerfile.contains("pip install"));
        assert!(dockerfile.contains("requirements.txt"));
    }

    #[tokio::test]
    async fn test_is_docker_available() {
        let manager = EnvironmentManager::new();
        
        // Этот тест просто проверяет что метод не падает
        let _result = manager.is_docker_available().await;
    }

    #[test]
    fn test_isolation_mode_variants() {
        let docker_config = DockerConfig {
            image: "test:latest".to_string(),
            working_dir: "/app".to_string(),
            ports: vec![8080],
            volumes: vec![],
            environment: vec![],
            read_only: true,
            no_root: true,
        };

        let sandbox_mode = IsolationMode::Sandbox(docker_config);
        
        match sandbox_mode {
            IsolationMode::Sandbox(config) => {
                assert!(config.read_only);
                assert!(config.no_root);
            }
            _ => panic!("Ожидался режим Sandbox"),
        }
    }

    #[test]
    fn test_virtual_env_config() {
        let config = VirtualEnvConfig {
            working_dir: PathBuf::from("/test/path"),
            env_vars: vec![
                ("PATH".to_string(), "/usr/bin".to_string()),
            ],
        };

        assert_eq!(config.working_dir, PathBuf::from("/test/path"));
        assert_eq!(config.env_vars.len(), 1);
    }

    #[test]
    fn test_environment_structure() {
        let env = Environment {
            id: "test-123".to_string(),
            mode: IsolationMode::Direct(VirtualEnvConfig {
                working_dir: PathBuf::from("/test"),
                env_vars: vec![],
            }),
            working_dir: PathBuf::from("/test"),
            container_id: None,
        };

        assert_eq!(env.id, "test-123");
        assert!(env.container_id.is_none());
    }
}

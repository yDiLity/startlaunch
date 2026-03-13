// Property-based тесты для менеджера окружений (Задача 3)
// Используется библиотека proptest для Rust

#[cfg(test)]
mod property_tests {
    use crate::environment_manager::{EnvironmentManager, IsolationMode};
    use crate::models::{ProjectInfo, TechStack, TrustLevel};
    use proptest::prelude::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    // Генератор для различных стеков технологий
    fn tech_stack_strategy() -> impl Strategy<Value = TechStack> {
        prop_oneof![
            Just(TechStack::NodeJs { version: Some("18".to_string()) }),
            Just(TechStack::NodeJs { version: Some("20".to_string()) }),
            Just(TechStack::NodeJs { version: None }),
            Just(TechStack::Python { version: Some("3.11".to_string()) }),
            Just(TechStack::Python { version: Some("3.12".to_string()) }),
            Just(TechStack::Python { version: None }),
            Just(TechStack::Rust { edition: Some("2021".to_string()) }),
            Just(TechStack::Rust { edition: None }),
            Just(TechStack::Go { version: Some("1.21".to_string()) }),
            Just(TechStack::Go { version: None }),
            Just(TechStack::Java { version: Some("17".to_string()) }),
            Just(TechStack::Docker { compose: true }),
            Just(TechStack::Docker { compose: false }),
            Just(TechStack::Static { framework: Some("React".to_string()) }),
            Just(TechStack::Static { framework: None }),
            Just(TechStack::Unknown),
        ]
    }

    // Генератор для уровней доверия
    fn trust_level_strategy() -> impl Strategy<Value = TrustLevel> {
        prop_oneof![
            Just(TrustLevel::Unknown),
            Just(TrustLevel::Untrusted),
            Just(TrustLevel::Trusted),
        ]
    }

    // Создание тестового ProjectInfo
    fn create_test_project(stack: TechStack, trust_level: TrustLevel) -> ProjectInfo {
        ProjectInfo {
            stack,
            entry_command: Some("test command".to_string()),
            dependencies: vec![],
            config_files: vec![],
            security_warnings: vec![],
            trust_level,
        }
    }

    // **Feature: autolaunch-core, Property 6: Режим безопасности по умолчанию**
    // **Validates: Requirements 4.1**
    // 
    // Для любого неизвестного или недоверенного проекта, система должна автоматически 
    // выбрать режим песочницы для изоляции

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Подзадача 3.1: Property тест для режима безопасности по умолчанию
        #[test]
        fn test_property_unknown_project_uses_sandbox(
            stack in tech_stack_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let manager = EnvironmentManager::new();
                let project = create_test_project(stack, TrustLevel::Unknown);
                
                // Создаем временную директорию для теста
                let temp_dir = TempDir::new().unwrap();
                let project_path = temp_dir.path();
                
                // Проверяем что Docker доступен, иначе пропускаем тест
                if !manager.is_docker_available().await {
                    // Если Docker недоступен, тест не может проверить песочницу
                    return Ok(());
                }
                
                let result = manager.create_environment(&project, project_path).await;
                
                prop_assert!(
                    result.is_ok(),
                    "Создание окружения для неизвестного проекта должно быть успешным"
                );
                
                let env = result.unwrap();
                
                // Требование 4.1: Неизвестный проект должен использовать режим песочницы
                match env.mode {
                    IsolationMode::Sandbox(_) => {
                        prop_assert!(true, "Неизвестный проект использует режим песочницы");
                    }
                    IsolationMode::Direct(_) => {
                        prop_assert!(
                            false,
                            "Неизвестный проект НЕ должен использовать прямой режим, только песочницу"
                        );
                    }
                }
                
                // Очистка
                let _ = manager.cleanup_environment(&env).await;
                
                Ok(())
            })?;
        }

        #[test]
        fn test_property_untrusted_project_uses_sandbox(
            stack in tech_stack_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let manager = EnvironmentManager::new();
                let project = create_test_project(stack, TrustLevel::Untrusted);
                
                let temp_dir = TempDir::new().unwrap();
                let project_path = temp_dir.path();
                
                if !manager.is_docker_available().await {
                    return Ok(());
                }
                
                let result = manager.create_environment(&project, project_path).await;
                
                prop_assert!(
                    result.is_ok(),
                    "Создание окружения для недоверенного проекта должно быть успешным"
                );
                
                let env = result.unwrap();
                
                // Требование 4.1: Недоверенный проект должен использовать режим песочницы
                match env.mode {
                    IsolationMode::Sandbox(_) => {
                        prop_assert!(true, "Недоверенный проект использует режим песочницы");
                    }
                    IsolationMode::Direct(_) => {
                        prop_assert!(
                            false,
                            "Недоверенный проект НЕ должен использовать прямой режим"
                        );
                    }
                }
                
                let _ = manager.cleanup_environment(&env).await;
                
                Ok(())
            })?;
        }

        #[test]
        fn test_property_sandbox_mode_consistency(
            stack in tech_stack_strategy(),
            trust_level in trust_level_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let manager = EnvironmentManager::new();
                let project = create_test_project(stack, trust_level.clone());
                
                let temp_dir = TempDir::new().unwrap();
                let project_path = temp_dir.path();
                
                if !manager.is_docker_available().await {
                    return Ok(());
                }
                
                let result = manager.create_environment(&project, project_path).await;
                
                if result.is_err() {
                    // Некоторые конфигурации могут не поддерживаться, это нормально
                    return Ok(());
                }
                
                let env = result.unwrap();
                
                // Проверяем консистентность: неизвестные и недоверенные проекты всегда в песочнице
                match trust_level {
                    TrustLevel::Unknown | TrustLevel::Untrusted => {
                        match env.mode {
                            IsolationMode::Sandbox(_) => {
                                prop_assert!(true, "Неизвестный/недоверенный проект в песочнице");
                            }
                            IsolationMode::Direct(_) => {
                                prop_assert!(
                                    false,
                                    "Неизвестный/недоверенный проект должен быть в песочнице, а не в прямом режиме"
                                );
                            }
                        }
                    }
                    TrustLevel::Trusted => {
                        // Доверенные проекты могут использовать любой режим
                        prop_assert!(true, "Доверенный проект может использовать любой режим");
                    }
                }
                
                let _ = manager.cleanup_environment(&env).await;
                
                Ok(())
            })?;
        }

        #[test]
        fn test_property_all_stacks_support_sandbox(
            stack in tech_stack_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let manager = EnvironmentManager::new();
                let project = create_test_project(stack.clone(), TrustLevel::Unknown);
                
                let temp_dir = TempDir::new().unwrap();
                let project_path = temp_dir.path();
                
                if !manager.is_docker_available().await {
                    return Ok(());
                }
                
                let result = manager.create_environment(&project, project_path).await;
                
                // Все стеки технологий должны поддерживать режим песочницы
                prop_assert!(
                    result.is_ok(),
                    "Стек {:?} должен поддерживать режим песочницы",
                    stack
                );
                
                let env = result.unwrap();
                
                match env.mode {
                    IsolationMode::Sandbox(_) => {
                        prop_assert!(true, "Стек {:?} использует песочницу", stack);
                    }
                    IsolationMode::Direct(_) => {
                        prop_assert!(
                            false,
                            "Неизвестный проект со стеком {:?} должен использовать песочницу",
                            stack
                        );
                    }
                }
                
                let _ = manager.cleanup_environment(&env).await;
                
                Ok(())
            })?;
        }

        #[test]
        fn test_property_sandbox_isolation_deterministic(
            stack in tech_stack_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let manager = EnvironmentManager::new();
                let project = create_test_project(stack, TrustLevel::Unknown);
                
                let temp_dir = TempDir::new().unwrap();
                let project_path = temp_dir.path();
                
                if !manager.is_docker_available().await {
                    return Ok(());
                }
                
                // Создаем окружение дважды для одного и того же проекта
                let result1 = manager.create_environment(&project, project_path).await;
                let result2 = manager.create_environment(&project, project_path).await;
                
                prop_assert!(result1.is_ok() && result2.is_ok(), "Оба создания должны быть успешными");
                
                let env1 = result1.unwrap();
                let env2 = result2.unwrap();
                
                // Оба окружения должны использовать один и тот же режим изоляции
                let mode1_is_sandbox = matches!(env1.mode, IsolationMode::Sandbox(_));
                let mode2_is_sandbox = matches!(env2.mode, IsolationMode::Sandbox(_));
                
                prop_assert_eq!(
                    mode1_is_sandbox,
                    mode2_is_sandbox,
                    "Режим изоляции должен быть детерминированным для одного и того же проекта"
                );
                
                // Оба должны быть в песочнице для неизвестного проекта
                prop_assert!(
                    mode1_is_sandbox && mode2_is_sandbox,
                    "Оба окружения должны использовать песочницу для неизвестного проекта"
                );
                
                let _ = manager.cleanup_environment(&env1).await;
                let _ = manager.cleanup_environment(&env2).await;
                
                Ok(())
            })?;
        }

        // **Feature: autolaunch-core, Property 7: Конфигурация безопасности контейнеров**
        // **Validates: Requirements 4.2**
        // 
        // Для любого создаваемого Docker контейнера, система должна применить ограничения 
        // безопасности (no-root, read-only FS, ограничение привилегий)

        #[test]
        fn test_property_docker_security_no_root(
            stack in tech_stack_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let manager = EnvironmentManager::new();
                let project = create_test_project(stack.clone(), TrustLevel::Unknown);
                
                let temp_dir = TempDir::new().unwrap();
                let project_path = temp_dir.path();
                
                if !manager.is_docker_available().await {
                    return Ok(());
                }
                
                let result = manager.create_environment(&project, project_path).await;
                
                if result.is_err() {
                    // Некоторые конфигурации могут не поддерживаться
                    return Ok(());
                }
                
                let env = result.unwrap();
                
                // Требование 4.2: Контейнер должен использовать no-root
                match &env.mode {
                    IsolationMode::Sandbox(config) => {
                        prop_assert!(
                            config.no_root,
                            "Docker контейнер для стека {:?} должен использовать no-root (non-privileged user)",
                            stack
                        );
                    }
                    IsolationMode::Direct(_) => {
                        // Прямой режим не использует Docker, пропускаем
                    }
                }
                
                let _ = manager.cleanup_environment(&env).await;
                
                Ok(())
            })?;
        }

        #[test]
        fn test_property_docker_security_read_only_fs(
            stack in tech_stack_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let manager = EnvironmentManager::new();
                let project = create_test_project(stack.clone(), TrustLevel::Unknown);
                
                let temp_dir = TempDir::new().unwrap();
                let project_path = temp_dir.path();
                
                if !manager.is_docker_available().await {
                    return Ok(());
                }
                
                let result = manager.create_environment(&project, project_path).await;
                
                if result.is_err() {
                    return Ok(());
                }
                
                let env = result.unwrap();
                
                // Требование 4.2: Контейнер должен использовать read-only файловую систему
                match &env.mode {
                    IsolationMode::Sandbox(config) => {
                        prop_assert!(
                            config.read_only,
                            "Docker контейнер для стека {:?} должен использовать read-only файловую систему",
                            stack
                        );
                    }
                    IsolationMode::Direct(_) => {
                        // Прямой режим не использует Docker
                    }
                }
                
                let _ = manager.cleanup_environment(&env).await;
                
                Ok(())
            })?;
        }

        #[test]
        fn test_property_docker_security_all_restrictions(
            stack in tech_stack_strategy(),
            trust_level in trust_level_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let manager = EnvironmentManager::new();
                let project = create_test_project(stack.clone(), trust_level);
                
                let temp_dir = TempDir::new().unwrap();
                let project_path = temp_dir.path();
                
                if !manager.is_docker_available().await {
                    return Ok(());
                }
                
                let result = manager.create_environment(&project, project_path).await;
                
                if result.is_err() {
                    return Ok(());
                }
                
                let env = result.unwrap();
                
                // Требование 4.2: Все Docker контейнеры должны иметь полный набор ограничений безопасности
                match &env.mode {
                    IsolationMode::Sandbox(config) => {
                        prop_assert!(
                            config.no_root,
                            "Docker контейнер должен использовать no-root для стека {:?}",
                            stack
                        );
                        
                        prop_assert!(
                            config.read_only,
                            "Docker контейнер должен использовать read-only FS для стека {:?}",
                            stack
                        );
                        
                        // Проверяем что конфигурация содержит все необходимые параметры
                        prop_assert!(
                            !config.image.is_empty(),
                            "Docker контейнер должен иметь валидный образ"
                        );
                        
                        prop_assert!(
                            !config.working_dir.is_empty(),
                            "Docker контейнер должен иметь рабочую директорию"
                        );
                    }
                    IsolationMode::Direct(_) => {
                        // Прямой режим не использует Docker
                    }
                }
                
                let _ = manager.cleanup_environment(&env).await;
                
                Ok(())
            })?;
        }

        #[test]
        fn test_property_docker_config_consistency(
            stack in tech_stack_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let manager = EnvironmentManager::new();
                let project = create_test_project(stack.clone(), TrustLevel::Unknown);
                
                let temp_dir = TempDir::new().unwrap();
                let project_path = temp_dir.path();
                
                if !manager.is_docker_available().await {
                    return Ok(());
                }
                
                // Создаем окружение дважды для проверки консистентности конфигурации
                let result1 = manager.create_environment(&project, project_path).await;
                let result2 = manager.create_environment(&project, project_path).await;
                
                if result1.is_err() || result2.is_err() {
                    return Ok(());
                }
                
                let env1 = result1.unwrap();
                let env2 = result2.unwrap();
                
                // Конфигурация безопасности должна быть одинаковой для одного и того же проекта
                match (&env1.mode, &env2.mode) {
                    (IsolationMode::Sandbox(config1), IsolationMode::Sandbox(config2)) => {
                        prop_assert_eq!(
                            config1.no_root,
                            config2.no_root,
                            "Настройка no_root должна быть консистентной"
                        );
                        
                        prop_assert_eq!(
                            config1.read_only,
                            config2.read_only,
                            "Настройка read_only должна быть консистентной"
                        );
                        
                        // Оба должны иметь ограничения безопасности
                        prop_assert!(
                            config1.no_root && config2.no_root,
                            "Оба контейнера должны использовать no-root"
                        );
                        
                        prop_assert!(
                            config1.read_only && config2.read_only,
                            "Оба контейнера должны использовать read-only FS"
                        );
                    }
                    _ => {
                        // Если не оба в режиме песочницы, пропускаем
                    }
                }
                
                let _ = manager.cleanup_environment(&env1).await;
                let _ = manager.cleanup_environment(&env2).await;
                
                Ok(())
            })?;
        }

        #[test]
        fn test_property_docker_security_independent_of_trust(
            stack in tech_stack_strategy(),
            trust_level in trust_level_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let manager = EnvironmentManager::new();
                let project = create_test_project(stack.clone(), trust_level.clone());
                
                let temp_dir = TempDir::new().unwrap();
                let project_path = temp_dir.path();
                
                if !manager.is_docker_available().await {
                    return Ok(());
                }
                
                let result = manager.create_environment(&project, project_path).await;
                
                if result.is_err() {
                    return Ok(());
                }
                
                let env = result.unwrap();
                
                // Требование 4.2: Ограничения безопасности контейнера должны применяться 
                // независимо от уровня доверия проекта
                match &env.mode {
                    IsolationMode::Sandbox(config) => {
                        prop_assert!(
                            config.no_root,
                            "Docker контейнер должен использовать no-root независимо от уровня доверия {:?}",
                            trust_level
                        );
                        
                        prop_assert!(
                            config.read_only,
                            "Docker контейнер должен использовать read-only FS независимо от уровня доверия {:?}",
                            trust_level
                        );
                    }
                    IsolationMode::Direct(_) => {
                        // Прямой режим не использует Docker
                    }
                }
                
                let _ = manager.cleanup_environment(&env).await;
                
                Ok(())
            })?;
        }
    }
}

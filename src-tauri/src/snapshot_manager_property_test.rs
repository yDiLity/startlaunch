// Property-based тесты для менеджера снимков (Задачи 6.1, 6.2)
// Используется библиотека proptest для Rust

#[cfg(test)]
mod tests {
    use crate::snapshot_manager::SnapshotManager;
    use crate::models::{ProjectInfo, TechStack, EnvironmentType, Dependency, TrustLevel};
    use proptest::prelude::*;
    use tempfile::TempDir;
    use std::fs;

    // Генератор для стеков технологий
    fn tech_stack_strategy() -> impl Strategy<Value = TechStack> {
        prop_oneof![
            Just(TechStack::NodeJs { version: Some("18".to_string()) }),
            Just(TechStack::Python { version: Some("3.11".to_string()) }),
            Just(TechStack::Rust { edition: Some("2021".to_string()) }),
            Just(TechStack::Go { version: Some("1.21".to_string()) }),
            Just(TechStack::Static { framework: Some("html".to_string()) }),
        ]
    }

    // Генератор для типов окружения
    fn environment_type_strategy() -> impl Strategy<Value = EnvironmentType> {
        prop_oneof![
            Just(EnvironmentType::Docker),
            Just(EnvironmentType::Direct),
        ]
    }

    // Генератор для портов
    fn ports_strategy() -> impl Strategy<Value = Vec<u16>> {
        prop::collection::vec(1024u16..65535u16, 0..4)
    }

    // Генератор для переменных окружения
    fn env_vars_strategy() -> impl Strategy<Value = Vec<(String, String)>> {
        prop::collection::vec(
            (
                prop_oneof![
                    Just("PORT".to_string()),
                    Just("NODE_ENV".to_string()),
                    Just("DEBUG".to_string()),
                    Just("DATABASE_URL".to_string()),
                ],
                prop_oneof![
                    Just("3000".to_string()),
                    Just("production".to_string()),
                    Just("true".to_string()),
                    Just("false".to_string()),
                ],
            ),
            0..4,
        )
    }

    // Генератор для команд запуска
    fn entry_command_strategy() -> impl Strategy<Value = Option<String>> {
        prop_oneof![
            Just(None),
            Just(Some("npm start".to_string())),
            Just(Some("python main.py".to_string())),
            Just(Some("cargo run".to_string())),
            Just(Some("go run main.go".to_string())),
        ]
    }

    // Создание тестового ProjectInfo
    fn create_project_info(stack: TechStack, entry_command: Option<String>) -> ProjectInfo {
        ProjectInfo {
            stack,
            entry_command,
            dependencies: vec![
                Dependency {
                    name: "test-dep".to_string(),
                    version: Some("1.0.0".to_string()),
                    dev: false,
                },
            ],
            config_files: vec![],
            security_warnings: vec![],
            trust_level: TrustLevel::Unknown,
        }
    }

    // Создание тестового проекта с файлами
    fn create_test_project(dir: &std::path::Path) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("index.js"), b"console.log('test');").unwrap();
        fs::write(dir.join("package.json"), b"{\"name\":\"test\"}").unwrap();
    }

    // **Feature: autolaunch-core, Property 17: Сериализация снимков проекта**
    // **Validates: Requirements 7.2, 7.4**
    //
    // Для любого сохраняемого снимка проекта, все зависимости, конфигурация
    // и метаданные должны быть корректно сериализованы и восстановимы

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_property_snapshot_metadata_roundtrip(
            stack in tech_stack_strategy(),
            entry_command in entry_command_strategy(),
            env_type in environment_type_strategy(),
            ports in ports_strategy(),
            env_vars in env_vars_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let project_path = temp_dir.path().join("project");
                create_test_project(&project_path);

                let manager = SnapshotManager::new().unwrap();
                let project_info = create_project_info(stack, entry_command.clone());

                // Требование 7.2: Создаём снимок
                let snapshot = manager.create_snapshot(
                    "test-project-id",
                    &project_path,
                    &project_info,
                    env_type,
                    ports.clone(),
                    env_vars.clone(),
                ).await.unwrap();

                prop_assert!(!snapshot.id.is_empty(), "ID снимка не должен быть пустым");
                prop_assert_eq!(&snapshot.project_id, "test-project-id");

                // Требование 7.4: Загружаем и проверяем метаданные
                let (loaded_path, metadata) = manager.load_snapshot(&snapshot.id).await.unwrap();

                prop_assert!(loaded_path.exists(), "Путь снимка должен существовать");

                prop_assert_eq!(
                    metadata.entry_command,
                    entry_command,
                    "Команда запуска должна быть корректно сериализована"
                );

                prop_assert_eq!(
                    metadata.ports,
                    ports,
                    "Порты должны быть корректно сериализованы"
                );

                prop_assert_eq!(
                    metadata.environment_variables,
                    env_vars,
                    "Переменные окружения должны быть корректно сериализованы"
                );

                prop_assert_eq!(
                    metadata.dependencies.len(),
                    project_info.dependencies.len(),
                    "Зависимости должны быть корректно сериализованы"
                );

                // Очистка
                manager.delete_snapshot(&snapshot.id).await.unwrap();
                Ok(())
            })?;
        }

        #[test]
        fn test_property_snapshot_files_preserved(
            stack in tech_stack_strategy(),
            env_type in environment_type_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let project_path = temp_dir.path().join("project");
                create_test_project(&project_path);

                let manager = SnapshotManager::new().unwrap();
                let project_info = create_project_info(stack, Some("npm start".to_string()));

                let snapshot = manager.create_snapshot(
                    "test-project-id",
                    &project_path,
                    &project_info,
                    env_type,
                    vec![3000],
                    vec![],
                ).await.unwrap();

                // Требование 7.2: Файлы проекта должны быть сохранены
                let (loaded_path, _) = manager.load_snapshot(&snapshot.id).await.unwrap();

                prop_assert!(
                    loaded_path.join("index.js").exists(),
                    "Файл index.js должен быть сохранён в снимке"
                );
                prop_assert!(
                    loaded_path.join("package.json").exists(),
                    "Файл package.json должен быть сохранён в снимке"
                );

                manager.delete_snapshot(&snapshot.id).await.unwrap();
                Ok(())
            })?;
        }

        #[test]
        fn test_property_snapshot_size_positive(
            stack in tech_stack_strategy(),
            env_type in environment_type_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let project_path = temp_dir.path().join("project");
                create_test_project(&project_path);

                let manager = SnapshotManager::new().unwrap();
                let project_info = create_project_info(stack, None);

                let snapshot = manager.create_snapshot(
                    "test-project-id",
                    &project_path,
                    &project_info,
                    env_type,
                    vec![],
                    vec![],
                ).await.unwrap();

                // Требование 7.2: Размер снимка должен быть положительным
                prop_assert!(
                    snapshot.size_bytes > 0,
                    "Размер снимка должен быть больше нуля"
                );

                manager.delete_snapshot(&snapshot.id).await.unwrap();
                Ok(())
            })?;
        }

        #[test]
        fn test_property_multiple_snapshots_independent(
            stack in tech_stack_strategy(),
            ports1 in ports_strategy(),
            ports2 in ports_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let project_path = temp_dir.path().join("project");
                create_test_project(&project_path);

                let manager = SnapshotManager::new().unwrap();
                let project_info = create_project_info(stack, Some("npm start".to_string()));

                // Создаём два снимка для одного проекта
                let snap1 = manager.create_snapshot(
                    "project-id",
                    &project_path,
                    &project_info,
                    EnvironmentType::Direct,
                    ports1.clone(),
                    vec![],
                ).await.unwrap();

                let snap2 = manager.create_snapshot(
                    "project-id",
                    &project_path,
                    &project_info,
                    EnvironmentType::Docker,
                    ports2.clone(),
                    vec![],
                ).await.unwrap();

                // Снимки должны быть независимы
                prop_assert_ne!(snap1.id, snap2.id, "ID снимков должны быть уникальными");

                let (_, meta1) = manager.load_snapshot(&snap1.id).await.unwrap();
                let (_, meta2) = manager.load_snapshot(&snap2.id).await.unwrap();

                prop_assert_eq!(meta1.ports, ports1, "Порты первого снимка должны совпадать");
                prop_assert_eq!(meta2.ports, ports2, "Порты второго снимка должны совпадать");

                manager.delete_snapshot(&snap1.id).await.unwrap();
                manager.delete_snapshot(&snap2.id).await.unwrap();
                Ok(())
            })?;
        }

        #[test]
        fn test_property_snapshot_metadata_json_valid(
            stack in tech_stack_strategy(),
            env_type in environment_type_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let project_path = temp_dir.path().join("project");
                create_test_project(&project_path);

                let manager = SnapshotManager::new().unwrap();
                let project_info = create_project_info(stack, Some("npm start".to_string()));

                let snapshot = manager.create_snapshot(
                    "project-id",
                    &project_path,
                    &project_info,
                    env_type,
                    vec![3000],
                    vec![("NODE_ENV".to_string(), "production".to_string())],
                ).await.unwrap();

                // Требование 7.4: metadata в БД должна быть валидным JSON
                let json_result: Result<serde_json::Value, _> =
                    serde_json::from_str(&snapshot.metadata);

                prop_assert!(
                    json_result.is_ok(),
                    "Метаданные снимка должны быть валидным JSON"
                );

                manager.delete_snapshot(&snapshot.id).await.unwrap();
                Ok(())
            })?;
        }
    }

    // **Feature: autolaunch-core, Property 18: Полная очистка при удалении снимков**
    // **Validates: Requirements 7.5**
    //
    // Для любого удаляемого снимка проекта, все связанные файлы и директории
    // должны быть полностью удалены без остатков

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_property_delete_removes_all_files(
            stack in tech_stack_strategy(),
            env_type in environment_type_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let project_path = temp_dir.path().join("project");
                create_test_project(&project_path);

                let manager = SnapshotManager::new().unwrap();
                let project_info = create_project_info(stack, None);

                let snapshot = manager.create_snapshot(
                    "project-id",
                    &project_path,
                    &project_info,
                    env_type,
                    vec![],
                    vec![],
                ).await.unwrap();

                let snapshot_path = std::path::PathBuf::from(&snapshot.snapshot_path);
                prop_assert!(snapshot_path.exists(), "Снимок должен существовать до удаления");

                // Требование 7.5: Удаляем снимок
                manager.delete_snapshot(&snapshot.id).await.unwrap();

                // Все файлы должны быть удалены
                prop_assert!(
                    !snapshot_path.exists(),
                    "Директория снимка должна быть полностью удалена"
                );

                Ok(())
            })?;
        }

        #[test]
        fn test_property_delete_nonexistent_is_safe(
            fake_id in "[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}"
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let manager = SnapshotManager::new().unwrap();

                // Требование 7.5: Удаление несуществующего снимка не должно вызывать ошибку
                let result = manager.delete_snapshot(&fake_id).await;

                prop_assert!(
                    result.is_ok(),
                    "Удаление несуществующего снимка должно быть безопасным"
                );

                Ok(())
            })?;
        }

        #[test]
        fn test_property_delete_idempotent(
            stack in tech_stack_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let project_path = temp_dir.path().join("project");
                create_test_project(&project_path);

                let manager = SnapshotManager::new().unwrap();
                let project_info = create_project_info(stack, None);

                let snapshot = manager.create_snapshot(
                    "project-id",
                    &project_path,
                    &project_info,
                    EnvironmentType::Direct,
                    vec![],
                    vec![],
                ).await.unwrap();

                // Удаляем дважды — должно быть идемпотентно
                manager.delete_snapshot(&snapshot.id).await.unwrap();
                let second_delete = manager.delete_snapshot(&snapshot.id).await;

                prop_assert!(
                    second_delete.is_ok(),
                    "Повторное удаление снимка должно быть безопасным (идемпотентность)"
                );

                Ok(())
            })?;
        }

        #[test]
        fn test_property_delete_one_does_not_affect_others(
            stack in tech_stack_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let temp_dir = TempDir::new().unwrap();
                let project_path = temp_dir.path().join("project");
                create_test_project(&project_path);

                let manager = SnapshotManager::new().unwrap();
                let project_info = create_project_info(stack, None);

                // Создаём два снимка
                let snap1 = manager.create_snapshot(
                    "project-id",
                    &project_path,
                    &project_info,
                    EnvironmentType::Direct,
                    vec![3000],
                    vec![],
                ).await.unwrap();

                let snap2 = manager.create_snapshot(
                    "project-id",
                    &project_path,
                    &project_info,
                    EnvironmentType::Direct,
                    vec![4000],
                    vec![],
                ).await.unwrap();

                let snap2_path = std::path::PathBuf::from(&snap2.snapshot_path);

                // Удаляем только первый снимок
                manager.delete_snapshot(&snap1.id).await.unwrap();

                // Второй снимок должен остаться нетронутым
                prop_assert!(
                    snap2_path.exists(),
                    "Второй снимок не должен быть затронут при удалении первого"
                );

                let load_result = manager.load_snapshot(&snap2.id).await;
                prop_assert!(
                    load_result.is_ok(),
                    "Второй снимок должен быть доступен после удаления первого"
                );

                // Очистка
                manager.delete_snapshot(&snap2.id).await.unwrap();
                Ok(())
            })?;
        }
    }
}

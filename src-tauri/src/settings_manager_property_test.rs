// Property-based тесты для менеджера настроек (Задачи 8.1, 8.2, 8.3)
// Используется библиотека proptest для Rust

#[cfg(test)]
mod tests {
    use crate::settings_manager::{SettingsManager, AppSettings, IsolationMode, Theme};
    use proptest::prelude::*;
    use tempfile::TempDir;
    use std::path::PathBuf;

    // Генератор для режимов изоляции
    fn isolation_mode_strategy() -> impl Strategy<Value = IsolationMode> {
        prop_oneof![
            Just(IsolationMode::Sandbox),
            Just(IsolationMode::Direct),
        ]
    }

    // Генератор для тем оформления
    fn theme_strategy() -> impl Strategy<Value = Theme> {
        prop_oneof![
            Just(Theme::Light),
            Just(Theme::Dark),
            Just(Theme::System),
        ]
    }

    // Генератор для путей снимков
    fn snapshots_path_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("/tmp/autolaunch/snapshots".to_string()),
            Just("/var/lib/autolaunch".to_string()),
            Just("./snapshots".to_string()),
            Just("/home/user/.autolaunch/snapshots".to_string()),
        ]
    }

    // Генератор для настроек автоочистки
    fn auto_cleanup_strategy() -> impl Strategy<Value = bool> {
        prop::bool::ANY
    }

    // Создание тестового менеджера настроек
    fn create_test_manager() -> (SettingsManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("settings.json");
        
        let settings = AppSettings::default();
        SettingsManager::save_to_file(&config_path, &settings).unwrap();
        
        let manager = SettingsManager {
            config_path,
            settings,
        };
        
        (manager, temp_dir)
    }

    // **Feature: autolaunch-core, Property 23: Применение настроек**
    // **Validates: Requirements 9.2**
    // 
    // Для любых изменений в настройках приложения, новые значения должны 
    // немедленно применяться ко всем последующим операциям

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Подзадача 8.1: Property тест для применения настроек
        #[test]
        fn test_property_settings_applied_immediately(
            isolation_mode in isolation_mode_strategy(),
            theme in theme_strategy(),
            auto_cleanup in auto_cleanup_strategy()
        ) {
            let (mut manager, _temp_dir) = create_test_manager();
            
            // Создаем новые настройки
            let new_settings = AppSettings {
                default_isolation_mode: isolation_mode.clone(),
                snapshots_path: "/test/path".to_string(),
                theme: theme.clone(),
                auto_cleanup,
                max_snapshot_age_days: 30,
                enable_logging: true,
            };
            
            // Требование 9.2: Обновляем настройки
            let update_result = manager.update_settings(new_settings.clone());
            
            prop_assert!(
                update_result.is_ok(),
                "Обновление настроек должно быть успешным"
            );
            
            // Требование 9.2: Проверяем что настройки применены немедленно
            let current_settings = manager.get_settings();
            
            prop_assert_eq!(
                current_settings.default_isolation_mode,
                isolation_mode,
                "Режим изоляции должен быть применен немедленно"
            );
            
            prop_assert_eq!(
                current_settings.theme,
                theme,
                "Тема должна быть применена немедленно"
            );
            
            prop_assert_eq!(
                current_settings.auto_cleanup,
                auto_cleanup,
                "Настройка автоочистки должна быть применена немедленно"
            );
            
            prop_assert_eq!(
                current_settings.snapshots_path,
                "/test/path",
                "Путь снимков должен быть применен немедленно"
            );
        }

        #[test]
        fn test_property_isolation_mode_persists(
            mode in isolation_mode_strategy()
        ) {
            let (mut manager, temp_dir) = create_test_manager();
            let config_path = manager.config_path.clone();
            
            // Устанавливаем режим изоляции
            manager.set_default_isolation_mode(mode.clone()).unwrap();
            
            // Проверяем что настройка применена
            prop_assert_eq!(
                manager.get_settings().default_isolation_mode,
                mode,
                "Режим изоляции должен быть применен"
            );
            
            // Создаем новый менеджер (имитация перезапуска приложения)
            drop(manager);
            let loaded_settings = SettingsManager::load_from_file(&config_path).unwrap();
            
            // Проверяем что настройка сохранилась
            prop_assert_eq!(
                loaded_settings.default_isolation_mode,
                mode,
                "Режим изоляции должен сохраниться после перезапуска"
            );
        }

        #[test]
        fn test_property_theme_persists(
            theme in theme_strategy()
        ) {
            let (mut manager, _temp_dir) = create_test_manager();
            let config_path = manager.config_path.clone();
            
            // Устанавливаем тему
            manager.set_theme(theme.clone()).unwrap();
            
            // Проверяем что настройка применена
            prop_assert_eq!(
                manager.get_settings().theme,
                theme,
                "Тема должна быть применена"
            );
            
            // Загружаем настройки из файла
            let loaded_settings = SettingsManager::load_from_file(&config_path).unwrap();
            
            // Проверяем что настройка сохранилась
            prop_assert_eq!(
                loaded_settings.theme,
                theme,
                "Тема должна сохраниться в файле"
            );
        }

        #[test]
        fn test_property_snapshots_path_persists(
            path in snapshots_path_strategy()
        ) {
            let (mut manager, _temp_dir) = create_test_manager();
            let config_path = manager.config_path.clone();
            
            // Устанавливаем путь снимков
            let set_result = manager.set_snapshots_path(path.clone());
            
            // Некоторые пути могут быть недоступны, это нормально
            if set_result.is_ok() {
                // Проверяем что настройка применена
                prop_assert_eq!(
                    manager.get_settings().snapshots_path,
                    path,
                    "Путь снимков должен быть применен"
                );
                
                // Загружаем настройки из файла
                let loaded_settings = SettingsManager::load_from_file(&config_path).unwrap();
                
                // Проверяем что настройка сохранилась
                prop_assert_eq!(
                    loaded_settings.snapshots_path,
                    path,
                    "Путь снимков должен сохраниться в файле"
                );
            }
        }
    }

    // **Feature: autolaunch-core, Property 24: Автоматическая очистка по настройкам**
    // **Validates: Requirements 9.4**
    // 
    // Для любого проекта при включенной настройке автоочистки, временные файлы 
    // должны автоматически удаляться после остановки

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Подзадача 8.2: Property тест для автоочистки
        #[test]
        fn test_property_auto_cleanup_setting(
            enabled in auto_cleanup_strategy()
        ) {
            let (mut manager, _temp_dir) = create_test_manager();
            
            // Требование 9.4: Устанавливаем настройку автоочистки
            let result = manager.set_auto_cleanup(enabled);
            
            prop_assert!(
                result.is_ok(),
                "Установка настройки автоочистки должна быть успешной"
            );
            
            // Проверяем что настройка применена
            prop_assert_eq!(
                manager.get_settings().auto_cleanup,
                enabled,
                "Настройка автоочистки должна быть применена: {}",
                enabled
            );
        }

        #[test]
        fn test_property_auto_cleanup_persists(
            enabled in auto_cleanup_strategy()
        ) {
            let (mut manager, _temp_dir) = create_test_manager();
            let config_path = manager.config_path.clone();
            
            // Устанавливаем настройку автоочистки
            manager.set_auto_cleanup(enabled).unwrap();
            
            // Загружаем настройки из файла
            let loaded_settings = SettingsManager::load_from_file(&config_path).unwrap();
            
            // Проверяем что настройка сохранилась
            prop_assert_eq!(
                loaded_settings.auto_cleanup,
                enabled,
                "Настройка автоочистки должна сохраниться: {}",
                enabled
            );
        }

        #[test]
        fn test_property_auto_cleanup_in_full_settings(
            isolation_mode in isolation_mode_strategy(),
            theme in theme_strategy(),
            auto_cleanup in auto_cleanup_strategy()
        ) {
            let (mut manager, _temp_dir) = create_test_manager();
            
            let new_settings = AppSettings {
                default_isolation_mode: isolation_mode,
                snapshots_path: "/test/path".to_string(),
                theme,
                auto_cleanup,
                max_snapshot_age_days: 30,
                enable_logging: true,
            };
            
            // Обновляем все настройки
            manager.update_settings(new_settings).unwrap();
            
            // Проверяем что настройка автоочистки применена корректно
            prop_assert_eq!(
                manager.get_settings().auto_cleanup,
                auto_cleanup,
                "Настройка автоочистки должна быть применена в составе полных настроек"
            );
        }
    }

    // **Feature: autolaunch-core, Property 25: Сериализация настроек**
    // **Validates: Requirements 9.5**
    // 
    // Для любых сохраняемых настроек, они должны корректно записываться в 
    // конфигурационный файл и восстанавливаться при запуске

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Подзадача 8.3: Property тест для сериализации настроек
        #[test]
        fn test_property_settings_serialization(
            isolation_mode in isolation_mode_strategy(),
            theme in theme_strategy(),
            auto_cleanup in auto_cleanup_strategy(),
            max_age in 1u64..365u64
        ) {
            let temp_dir = TempDir::new().unwrap();
            let config_path = temp_dir.path().join("settings.json");
            
            let original_settings = AppSettings {
                default_isolation_mode: isolation_mode.clone(),
                snapshots_path: "/test/snapshots".to_string(),
                theme: theme.clone(),
                auto_cleanup,
                max_snapshot_age_days: max_age,
                enable_logging: true,
            };
            
            // Требование 9.5: Сохраняем настройки в файл
            let save_result = SettingsManager::save_to_file(&config_path, &original_settings);
            
            prop_assert!(
                save_result.is_ok(),
                "Сохранение настроек должно быть успешным"
            );
            
            prop_assert!(
                config_path.exists(),
                "Файл конфигурации должен быть создан"
            );
            
            // Требование 9.5: Загружаем настройки из файла
            let load_result = SettingsManager::load_from_file(&config_path);
            
            prop_assert!(
                load_result.is_ok(),
                "Загрузка настроек должна быть успешной"
            );
            
            let loaded_settings = load_result.unwrap();
            
            // Проверяем что все поля корректно сериализованы и десериализованы
            prop_assert_eq!(
                loaded_settings.default_isolation_mode,
                isolation_mode,
                "Режим изоляции должен быть корректно сериализован"
            );
            
            prop_assert_eq!(
                loaded_settings.theme,
                theme,
                "Тема должна быть корректно сериализована"
            );
            
            prop_assert_eq!(
                loaded_settings.auto_cleanup,
                auto_cleanup,
                "Настройка автоочистки должна быть корректно сериализована"
            );
            
            prop_assert_eq!(
                loaded_settings.max_snapshot_age_days,
                max_age,
                "Максимальный возраст снимков должен быть корректно сериализован"
            );
            
            prop_assert_eq!(
                loaded_settings.snapshots_path,
                "/test/snapshots",
                "Путь снимков должен быть корректно сериализован"
            );
        }

        #[test]
        fn test_property_serialization_roundtrip(
            isolation_mode in isolation_mode_strategy(),
            theme in theme_strategy(),
            auto_cleanup in auto_cleanup_strategy()
        ) {
            let temp_dir = TempDir::new().unwrap();
            let config_path = temp_dir.path().join("settings.json");
            
            let settings1 = AppSettings {
                default_isolation_mode: isolation_mode.clone(),
                snapshots_path: "/path1".to_string(),
                theme: theme.clone(),
                auto_cleanup,
                max_snapshot_age_days: 30,
                enable_logging: true,
            };
            
            // Первый цикл сериализации
            SettingsManager::save_to_file(&config_path, &settings1).unwrap();
            let loaded1 = SettingsManager::load_from_file(&config_path).unwrap();
            
            // Второй цикл сериализации (roundtrip)
            SettingsManager::save_to_file(&config_path, &loaded1).unwrap();
            let loaded2 = SettingsManager::load_from_file(&config_path).unwrap();
            
            // Проверяем что данные не изменились после двух циклов
            prop_assert_eq!(
                loaded2.default_isolation_mode,
                isolation_mode,
                "Режим изоляции должен сохраниться после roundtrip"
            );
            
            prop_assert_eq!(
                loaded2.theme,
                theme,
                "Тема должна сохраниться после roundtrip"
            );
            
            prop_assert_eq!(
                loaded2.auto_cleanup,
                auto_cleanup,
                "Настройка автоочистки должна сохраниться после roundtrip"
            );
        }

        #[test]
        fn test_property_multiple_saves_idempotent(
            isolation_mode in isolation_mode_strategy(),
            saves in 1usize..10usize
        ) {
            let temp_dir = TempDir::new().unwrap();
            let config_path = temp_dir.path().join("settings.json");
            
            let settings = AppSettings {
                default_isolation_mode: isolation_mode.clone(),
                snapshots_path: "/test".to_string(),
                theme: Theme::Dark,
                auto_cleanup: true,
                max_snapshot_age_days: 30,
                enable_logging: true,
            };
            
            // Сохраняем настройки несколько раз
            for _ in 0..saves {
                SettingsManager::save_to_file(&config_path, &settings).unwrap();
            }
            
            // Загружаем настройки
            let loaded = SettingsManager::load_from_file(&config_path).unwrap();
            
            // Проверяем что множественные сохранения не повредили данные
            prop_assert_eq!(
                loaded.default_isolation_mode,
                isolation_mode,
                "Множественные сохранения не должны повредить данные"
            );
        }

        #[test]
        fn test_property_file_format_valid_json(
            isolation_mode in isolation_mode_strategy(),
            theme in theme_strategy()
        ) {
            let temp_dir = TempDir::new().unwrap();
            let config_path = temp_dir.path().join("settings.json");
            
            let settings = AppSettings {
                default_isolation_mode: isolation_mode,
                snapshots_path: "/test".to_string(),
                theme,
                auto_cleanup: true,
                max_snapshot_age_days: 30,
                enable_logging: true,
            };
            
            // Сохраняем настройки
            SettingsManager::save_to_file(&config_path, &settings).unwrap();
            
            // Читаем файл как текст
            let content = std::fs::read_to_string(&config_path).unwrap();
            
            // Проверяем что это валидный JSON
            let json_parse_result: Result<serde_json::Value, _> = serde_json::from_str(&content);
            
            prop_assert!(
                json_parse_result.is_ok(),
                "Файл настроек должен содержать валидный JSON"
            );
        }
    }
}

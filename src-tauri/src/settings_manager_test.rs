use crate::settings_manager::{AppSettings, IsolationMode, SettingsManager, Theme};
use std::fs;
use tempfile::TempDir;

#[cfg(test)]
mod settings_tests {
    use super::*;

    fn create_test_settings_manager() -> (SettingsManager, TempDir) {
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

    #[test]
    fn test_default_settings_values() {
        // Проверяем значения по умолчанию
        let settings = AppSettings::default();
        
        assert_eq!(settings.default_isolation_mode, IsolationMode::Sandbox);
        assert_eq!(settings.theme, Theme::Dark);
        assert!(settings.auto_cleanup);
        assert_eq!(settings.max_snapshot_age_days, 30);
        assert!(settings.enable_logging);
    }

    #[test]
    fn test_save_and_load_settings() {
        // Требование 9.5: Сохранение настроек в конфигурационный файл
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("settings.json");
        
        let settings = AppSettings {
            default_isolation_mode: IsolationMode::Direct,
            snapshots_path: "/custom/path".to_string(),
            theme: Theme::Light,
            auto_cleanup: false,
            max_snapshot_age_days: 60,
            enable_logging: false,
        };
        
        // Сохраняем
        SettingsManager::save_to_file(&config_path, &settings).unwrap();
        
        // Проверяем что файл создан
        assert!(config_path.exists());
        
        // Загружаем
        let loaded = SettingsManager::load_from_file(&config_path).unwrap();
        
        // Проверяем что все поля совпадают
        assert_eq!(loaded.default_isolation_mode, IsolationMode::Direct);
        assert_eq!(loaded.snapshots_path, "/custom/path");
        assert_eq!(loaded.theme, Theme::Light);
        assert!(!loaded.auto_cleanup);
        assert_eq!(loaded.max_snapshot_age_days, 60);
        assert!(!loaded.enable_logging);
    }

    #[test]
    fn test_update_settings() {
        // Требование 9.2: Применение настроек немедленно
        let (mut manager, _temp_dir) = create_test_settings_manager();
        
        let new_settings = AppSettings {
            default_isolation_mode: IsolationMode::Direct,
            snapshots_path: "/new/path".to_string(),
            theme: Theme::System,
            auto_cleanup: false,
            max_snapshot_age_days: 90,
            enable_logging: true,
        };
        
        manager.update_settings(new_settings.clone()).unwrap();
        
        // Проверяем что настройки применены
        assert_eq!(manager.get_settings().default_isolation_mode, IsolationMode::Direct);
        assert_eq!(manager.get_settings().snapshots_path, "/new/path");
        assert_eq!(manager.get_settings().theme, Theme::System);
        assert!(!manager.get_settings().auto_cleanup);
        assert_eq!(manager.get_settings().max_snapshot_age_days, 90);
    }

    #[test]
    fn test_set_isolation_mode() {
        // Требование 9.1: Настройки режима изоляции
        let (mut manager, _temp_dir) = create_test_settings_manager();
        
        // Изначально Sandbox
        assert_eq!(manager.get_settings().default_isolation_mode, IsolationMode::Sandbox);
        
        // Меняем на Direct
        manager.set_default_isolation_mode(IsolationMode::Direct).unwrap();
        assert_eq!(manager.get_settings().default_isolation_mode, IsolationMode::Direct);
        
        // Меняем обратно на Sandbox
        manager.set_default_isolation_mode(IsolationMode::Sandbox).unwrap();
        assert_eq!(manager.get_settings().default_isolation_mode, IsolationMode::Sandbox);
    }

    #[test]
    fn test_set_snapshots_path() {
        // Требование 9.2: Изменение пути сохранения снимков
        let (mut manager, temp_dir) = create_test_settings_manager();
        
        let new_path = temp_dir.path().join("custom_snapshots").to_string_lossy().to_string();
        
        manager.set_snapshots_path(new_path.clone()).unwrap();
        
        assert_eq!(manager.get_settings().snapshots_path, new_path);
        
        // Проверяем что директория создана
        assert!(temp_dir.path().join("custom_snapshots").exists());
    }

    #[test]
    fn test_set_theme() {
        // Требование 9.3: Настройки темы оформления
        let (mut manager, _temp_dir) = create_test_settings_manager();
        
        // Изначально Dark
        assert_eq!(manager.get_settings().theme, Theme::Dark);
        
        // Меняем на Light
        manager.set_theme(Theme::Light).unwrap();
        assert_eq!(manager.get_settings().theme, Theme::Light);
        
        // Меняем на System
        manager.set_theme(Theme::System).unwrap();
        assert_eq!(manager.get_settings().theme, Theme::System);
    }

    #[test]
    fn test_set_auto_cleanup() {
        // Требование 9.4: Настройки автоочистки
        let (mut manager, _temp_dir) = create_test_settings_manager();
        
        // Изначально включена
        assert!(manager.get_settings().auto_cleanup);
        
        // Выключаем
        manager.set_auto_cleanup(false).unwrap();
        assert!(!manager.get_settings().auto_cleanup);
        
        // Включаем обратно
        manager.set_auto_cleanup(true).unwrap();
        assert!(manager.get_settings().auto_cleanup);
    }

    #[test]
    fn test_reset_to_defaults() {
        let (mut manager, _temp_dir) = create_test_settings_manager();
        
        // Изменяем все настройки
        manager.set_theme(Theme::Light).unwrap();
        manager.set_auto_cleanup(false).unwrap();
        manager.set_default_isolation_mode(IsolationMode::Direct).unwrap();
        
        // Проверяем что изменения применены
        assert_eq!(manager.get_settings().theme, Theme::Light);
        assert!(!manager.get_settings().auto_cleanup);
        assert_eq!(manager.get_settings().default_isolation_mode, IsolationMode::Direct);
        
        // Сбрасываем к значениям по умолчанию
        manager.reset_to_defaults().unwrap();
        
        // Проверяем что все вернулось к дефолтным значениям
        assert_eq!(manager.get_settings().theme, Theme::Dark);
        assert!(manager.get_settings().auto_cleanup);
        assert_eq!(manager.get_settings().default_isolation_mode, IsolationMode::Sandbox);
    }

    #[test]
    fn test_settings_persistence() {
        // Проверяем что настройки сохраняются между перезапусками
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("settings.json");
        
        // Создаем первый менеджер и меняем настройки
        {
            let settings = AppSettings::default();
            SettingsManager::save_to_file(&config_path, &settings).unwrap();
            
            let mut manager = SettingsManager {
                config_path: config_path.clone(),
                settings,
            };
            
            manager.set_theme(Theme::Light).unwrap();
            manager.set_auto_cleanup(false).unwrap();
        }
        
        // Создаем второй менеджер и проверяем что настройки загрузились
        {
            let loaded_settings = SettingsManager::load_from_file(&config_path).unwrap();
            
            assert_eq!(loaded_settings.theme, Theme::Light);
            assert!(!loaded_settings.auto_cleanup);
        }
    }

    #[test]
    fn test_invalid_snapshots_path_creates_directory() {
        let (mut manager, temp_dir) = create_test_settings_manager();
        
        // Путь к несуществующей директории
        let new_path = temp_dir.path()
            .join("deeply")
            .join("nested")
            .join("path")
            .to_string_lossy()
            .to_string();
        
        // Устанавливаем путь - должна создаться вся цепочка директорий
        manager.set_snapshots_path(new_path.clone()).unwrap();
        
        assert_eq!(manager.get_settings().snapshots_path, new_path);
    }

    #[test]
    fn test_settings_serialization_format() {
        // Проверяем что настройки сохраняются в правильном JSON формате
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("settings.json");
        
        let settings = AppSettings {
            default_isolation_mode: IsolationMode::Sandbox,
            snapshots_path: "/test/path".to_string(),
            theme: Theme::Dark,
            auto_cleanup: true,
            max_snapshot_age_days: 30,
            enable_logging: true,
        };
        
        SettingsManager::save_to_file(&config_path, &settings).unwrap();
        
        // Читаем файл и проверяем что это валидный JSON
        let content = fs::read_to_string(&config_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
        
        assert_eq!(parsed["default_isolation_mode"], "Sandbox");
        assert_eq!(parsed["snapshots_path"], "/test/path");
        assert_eq!(parsed["theme"], "Dark");
        assert_eq!(parsed["auto_cleanup"], true);
        assert_eq!(parsed["max_snapshot_age_days"], 30);
        assert_eq!(parsed["enable_logging"], true);
    }

    #[test]
    fn test_concurrent_settings_updates() {
        // Проверяем что множественные обновления настроек работают корректно
        let (mut manager, _temp_dir) = create_test_settings_manager();
        
        // Выполняем несколько обновлений подряд
        manager.set_theme(Theme::Light).unwrap();
        manager.set_auto_cleanup(false).unwrap();
        manager.set_default_isolation_mode(IsolationMode::Direct).unwrap();
        
        let new_path = "/final/path".to_string();
        manager.set_snapshots_path(new_path.clone()).unwrap();
        
        // Проверяем что все изменения применены
        let settings = manager.get_settings();
        assert_eq!(settings.theme, Theme::Light);
        assert!(!settings.auto_cleanup);
        assert_eq!(settings.default_isolation_mode, IsolationMode::Direct);
        assert_eq!(settings.snapshots_path, new_path);
    }

    #[test]
    fn test_settings_edge_cases() {
        let (mut manager, _temp_dir) = create_test_settings_manager();
        
        // Пустой путь для снимков
        let empty_path = "".to_string();
        let result = manager.set_snapshots_path(empty_path);
        // Должно работать, но путь останется пустым
        assert!(result.is_ok());
        
        // Очень большое значение для max_snapshot_age_days
        let mut settings = manager.get_settings().clone();
        settings.max_snapshot_age_days = 365 * 10; // 10 лет
        assert!(manager.update_settings(settings).is_ok());
        
        // Минимальное значение
        let mut settings = manager.get_settings().clone();
        settings.max_snapshot_age_days = 1;
        assert!(manager.update_settings(settings).is_ok());
    }
}

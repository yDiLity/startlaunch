use crate::error::{AutoLaunchError, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Настройки приложения (Требование 9)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// Режим изоляции по умолчанию (Требование 9.1)
    pub default_isolation_mode: IsolationMode,
    
    /// Путь для сохранения снимков (Требование 9.2)
    pub snapshots_path: String,
    
    /// Тема оформления (Требование 9.3)
    pub theme: Theme,
    
    /// Автоматическая очистка временных файлов (Требование 9.4)
    pub auto_cleanup: bool,
    
    /// Дополнительные настройки
    pub max_snapshot_age_days: u64,
    pub enable_logging: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IsolationMode {
    Sandbox,
    Direct,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Theme {
    Light,
    Dark,
    System,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            default_isolation_mode: IsolationMode::Sandbox,
            snapshots_path: Self::default_snapshots_path(),
            theme: Theme::Dark,
            auto_cleanup: true,
            max_snapshot_age_days: 30,
            enable_logging: true,
        }
    }
}

impl AppSettings {
    fn default_snapshots_path() -> String {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("autolaunch")
            .join("snapshots")
            .to_string_lossy()
            .to_string()
    }
}

/// Менеджер настроек приложения
pub struct SettingsManager {
    config_path: PathBuf,
    settings: AppSettings,
}

impl SettingsManager {
    /// Создает новый менеджер настроек
    pub fn new() -> Result<Self> {
        let config_path = Self::get_config_path()?;
        
        // Создаем директорию для конфигурации если не существует
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        // Загружаем настройки или создаем по умолчанию
        let settings = if config_path.exists() {
            Self::load_from_file(&config_path)?
        } else {
            let default_settings = AppSettings::default();
            Self::save_to_file(&config_path, &default_settings)?;
            default_settings
        };
        
        tracing::info!("Менеджер настроек инициализирован: {:?}", config_path);
        
        Ok(Self {
            config_path,
            settings,
        })
    }
    
    /// Получает путь к файлу конфигурации
    fn get_config_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| AutoLaunchError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Не удалось определить директорию конфигурации"
            )))?;
        
        Ok(config_dir.join("autolaunch").join("settings.json"))
    }
    
    /// Загружает настройки из файла
    fn load_from_file(path: &PathBuf) -> Result<AppSettings> {
        let content = fs::read_to_string(path)?;
        let settings: AppSettings = serde_json::from_str(&content)?;
        tracing::info!("Настройки загружены из файла: {:?}", path);
        Ok(settings)
    }
    
    /// Сохраняет настройки в файл (Требование 9.5)
    fn save_to_file(path: &PathBuf, settings: &AppSettings) -> Result<()> {
        let content = serde_json::to_string_pretty(settings)?;
        fs::write(path, content)?;
        tracing::info!("Настройки сохранены в файл: {:?}", path);
        Ok(())
    }
    
    /// Получает текущие настройки
    pub fn get_settings(&self) -> &AppSettings {
        &self.settings
    }
    
    /// Обновляет настройки (Требование 9.2: применяются немедленно)
    pub fn update_settings(&mut self, new_settings: AppSettings) -> Result<()> {
        tracing::info!("Обновление настроек приложения");
        
        // Валидация пути для снимков
        if !new_settings.snapshots_path.is_empty() {
            let snapshots_path = PathBuf::from(&new_settings.snapshots_path);
            if let Some(parent) = snapshots_path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
        }
        
        // Сохраняем настройки в файл
        Self::save_to_file(&self.config_path, &new_settings)?;
        
        // Применяем настройки
        self.settings = new_settings;
        
        tracing::info!("Настройки успешно обновлены и применены");
        Ok(())
    }
    
    /// Обновляет режим изоляции по умолчанию (Требование 9.1)
    pub fn set_default_isolation_mode(&mut self, mode: IsolationMode) -> Result<()> {
        tracing::info!("Изменение режима изоляции по умолчанию: {:?}", mode);
        self.settings.default_isolation_mode = mode;
        Self::save_to_file(&self.config_path, &self.settings)?;
        Ok(())
    }
    
    /// Обновляет путь сохранения снимков (Требование 9.2)
    pub fn set_snapshots_path(&mut self, path: String) -> Result<()> {
        tracing::info!("Изменение пути сохранения снимков: {}", path);
        
        // Создаем директорию если не существует
        let snapshots_path = PathBuf::from(&path);
        if let Some(parent) = snapshots_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }
        
        self.settings.snapshots_path = path;
        Self::save_to_file(&self.config_path, &self.settings)?;
        Ok(())
    }
    
    /// Обновляет тему оформления (Требование 9.3)
    pub fn set_theme(&mut self, theme: Theme) -> Result<()> {
        tracing::info!("Изменение темы оформления: {:?}", theme);
        self.settings.theme = theme;
        Self::save_to_file(&self.config_path, &self.settings)?;
        Ok(())
    }
    
    /// Обновляет настройку автоочистки (Требование 9.4)
    pub fn set_auto_cleanup(&mut self, enabled: bool) -> Result<()> {
        tracing::info!("Изменение настройки автоочистки: {}", enabled);
        self.settings.auto_cleanup = enabled;
        Self::save_to_file(&self.config_path, &self.settings)?;
        Ok(())
    }
    
    /// Сбрасывает настройки к значениям по умолчанию
    pub fn reset_to_defaults(&mut self) -> Result<()> {
        tracing::info!("Сброс настроек к значениям по умолчанию");
        self.settings = AppSettings::default();
        Self::save_to_file(&self.config_path, &self.settings)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
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
    
    #[test]
    fn test_default_settings() {
        let settings = AppSettings::default();
        assert_eq!(settings.default_isolation_mode, IsolationMode::Sandbox);
        assert_eq!(settings.theme, Theme::Dark);
        assert!(settings.auto_cleanup);
    }
    
    #[test]
    fn test_save_and_load_settings() {
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
        
        SettingsManager::save_to_file(&config_path, &settings).unwrap();
        let loaded = SettingsManager::load_from_file(&config_path).unwrap();
        
        assert_eq!(loaded.default_isolation_mode, IsolationMode::Direct);
        assert_eq!(loaded.snapshots_path, "/custom/path");
        assert_eq!(loaded.theme, Theme::Light);
        assert!(!loaded.auto_cleanup);
    }
    
    #[test]
    fn test_update_settings() {
        let (mut manager, _temp_dir) = create_test_manager();
        
        let new_settings = AppSettings {
            default_isolation_mode: IsolationMode::Direct,
            snapshots_path: "/new/path".to_string(),
            theme: Theme::System,
            auto_cleanup: false,
            max_snapshot_age_days: 90,
            enable_logging: true,
        };
        
        manager.update_settings(new_settings.clone()).unwrap();
        
        assert_eq!(manager.get_settings().default_isolation_mode, IsolationMode::Direct);
        assert_eq!(manager.get_settings().theme, Theme::System);
        assert!(!manager.get_settings().auto_cleanup);
    }
    
    #[test]
    fn test_set_isolation_mode() {
        let (mut manager, _temp_dir) = create_test_manager();
        
        manager.set_default_isolation_mode(IsolationMode::Direct).unwrap();
        assert_eq!(manager.get_settings().default_isolation_mode, IsolationMode::Direct);
    }
    
    #[test]
    fn test_set_theme() {
        let (mut manager, _temp_dir) = create_test_manager();
        
        manager.set_theme(Theme::Light).unwrap();
        assert_eq!(manager.get_settings().theme, Theme::Light);
    }
    
    #[test]
    fn test_set_auto_cleanup() {
        let (mut manager, _temp_dir) = create_test_manager();
        
        manager.set_auto_cleanup(false).unwrap();
        assert!(!manager.get_settings().auto_cleanup);
    }
    
    #[test]
    fn test_reset_to_defaults() {
        let (mut manager, _temp_dir) = create_test_manager();
        
        // Изменяем настройки
        manager.set_theme(Theme::Light).unwrap();
        manager.set_auto_cleanup(false).unwrap();
        
        // Сбрасываем к значениям по умолчанию
        manager.reset_to_defaults().unwrap();
        
        assert_eq!(manager.get_settings().theme, Theme::Dark);
        assert!(manager.get_settings().auto_cleanup);
    }
}

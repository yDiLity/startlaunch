use crate::error::{AutoLaunchError, Result};
use crate::models::{ProjectInfo, SecurityWarning, SecurityLevel};
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// SecurityScanner анализирует проекты и команды на предмет потенциальных угроз безопасности
pub struct SecurityScanner {
    trusted_repos: HashSet<String>,
    trusted_repos_file: PathBuf,
}

impl SecurityScanner {
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| AutoLaunchError::Security("Не удалось найти директорию конфигурации".to_string()))?
            .join("autolaunch");
        
        if !config_dir.exists() {
            fs::create_dir_all(&config_dir)?;
        }

        let trusted_repos_file = config_dir.join("trusted_repos.json");
        let trusted_repos = Self::load_trusted_repos(&trusted_repos_file)?;

        Ok(Self {
            trusted_repos,
            trusted_repos_file,
        })
    }

    /// Загружает список доверенных репозиториев из файла
    fn load_trusted_repos(file_path: &Path) -> Result<HashSet<String>> {
        if !file_path.exists() {
            return Ok(HashSet::new());
        }

        let content = fs::read_to_string(file_path)?;
        let repos: HashSet<String> = serde_json::from_str(&content)
            .unwrap_or_else(|_| HashSet::new());
        
        Ok(repos)
    }

    /// Сохраняет список доверенных репозиториев в файл
    fn save_trusted_repos(&self) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.trusted_repos)
            .map_err(|e| AutoLaunchError::Security(format!("Ошибка сериализации: {}", e)))?;
        
        fs::write(&self.trusted_repos_file, content)?;
        Ok(())
    }

    /// Сканирует проект на предмет потенциальных угроз безопасности
    pub fn scan_project(&self, project: &ProjectInfo) -> Vec<SecurityWarning> {
        let mut warnings = Vec::new();

        // Проверяем команду запуска
        if let Some(ref command) = project.entry_command {
            warnings.extend(self.scan_command(command));
        }

        // Добавляем предупреждения из анализа проекта
        warnings.extend(project.security_warnings.clone());

        warnings
    }

    /// Сканирует команду на предмет опасных операций
    pub fn scan_command(&self, command: &str) -> Vec<SecurityWarning> {
        let mut warnings = Vec::new();

        // Опасные паттерны команд с уровнями угрозы
        let critical_patterns = vec![
            (r"rm\s+-rf\s+/", "Удаление корневой директории", SecurityLevel::Critical),
            (r":\(\)\{\s*:\|:&\s*\};:", "Fork bomb", SecurityLevel::Critical),
            (r"dd\s+if=/dev/zero\s+of=/dev/", "Перезапись устройства", SecurityLevel::Critical),
        ];

        let high_patterns = vec![
            (r"rm\s+-rf", "Рекурсивное удаление файлов", SecurityLevel::High),
            (r"sudo\s+", "Выполнение с правами суперпользователя", SecurityLevel::High),
            (r"curl.*\|.*bash", "Выполнение скрипта из интернета", SecurityLevel::High),
            (r"wget.*\|.*bash", "Выполнение скрипта из интернета", SecurityLevel::High),
            (r"curl.*\|.*sh", "Выполнение скрипта из интернета", SecurityLevel::High),
            (r"wget.*\|.*sh", "Выполнение скрипта из интернета", SecurityLevel::High),
            (r"eval\s*\(", "Динамическое выполнение кода", SecurityLevel::High),
            (r"exec\s*\(", "Выполнение произвольного кода", SecurityLevel::High),
            (r"chmod\s+777", "Установка небезопасных прав доступа", SecurityLevel::High),
        ];

        let medium_patterns = vec![
            (r">/dev/null\s+2>&1", "Подавление вывода ошибок", SecurityLevel::Medium),
            (r"nohup\s+", "Фоновое выполнение процесса", SecurityLevel::Medium),
            (r"&\s*$", "Фоновое выполнение", SecurityLevel::Medium),
        ];

        // Проверяем критические паттерны
        for (pattern, description, level) in critical_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(command) {
                    warnings.push(SecurityWarning {
                        level,
                        message: format!("КРИТИЧЕСКАЯ УГРОЗА: {}", description),
                        suggestion: Some("Не выполняйте эту команду! Она может повредить вашу систему.".to_string()),
                    });
                }
            }
        }

        // Проверяем высокоопасные паттерны
        for (pattern, description, level) in high_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(command) {
                    warnings.push(SecurityWarning {
                        level,
                        message: format!("Обнаружена потенциально опасная операция: {}", description),
                        suggestion: Some("Внимательно проверьте команду перед выполнением".to_string()),
                    });
                }
            }
        }

        // Проверяем среднеопасные паттерны
        for (pattern, description, level) in medium_patterns {
            if let Ok(re) = Regex::new(pattern) {
                if re.is_match(command) {
                    warnings.push(SecurityWarning {
                        level,
                        message: format!("Обнаружена подозрительная операция: {}", description),
                        suggestion: Some("Убедитесь, что понимаете, что делает эта команда".to_string()),
                    });
                }
            }
        }

        warnings
    }

    /// Проверяет, является ли репозиторий доверенным
    pub fn is_trusted_repository(&self, repo_url: &str) -> bool {
        let normalized_url = Self::normalize_repo_url(repo_url);
        self.trusted_repos.contains(&normalized_url)
    }

    /// Добавляет репозиторий в список доверенных
    pub fn add_trusted_repository(&mut self, repo_url: &str) -> Result<()> {
        let normalized_url = Self::normalize_repo_url(repo_url);
        self.trusted_repos.insert(normalized_url);
        self.save_trusted_repos()?;
        Ok(())
    }

    /// Удаляет репозиторий из списка доверенных
    pub fn remove_trusted_repository(&mut self, repo_url: &str) -> Result<()> {
        let normalized_url = Self::normalize_repo_url(repo_url);
        self.trusted_repos.remove(&normalized_url);
        self.save_trusted_repos()?;
        Ok(())
    }

    /// Нормализует URL репозитория для единообразного сравнения
    fn normalize_repo_url(url: &str) -> String {
        url.trim()
            .trim_end_matches('/')
            .trim_end_matches(".git")
            .to_lowercase()
    }

    /// Возвращает список всех доверенных репозиториев
    pub fn get_trusted_repositories(&self) -> Vec<String> {
        self.trusted_repos.iter().cloned().collect()
    }
}


#[cfg(test)]
mod tests;

#[cfg(test)]
mod property_tests;

use crate::error::{AutoLaunchError, Result};
use regex::Regex;
use url::Url;

/// Структура для хранения распарсенной информации о GitHub репозитории
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitHubRepoInfo {
    pub owner: String,
    pub repo_name: String,
    pub normalized_url: String,
}

/// Парсер и валидатор GitHub URL
/// 
/// Поддерживает следующие форматы:
/// - owner/repo (например: facebook/react)
/// - https://github.com/owner/repo
/// - https://github.com/owner/repo.git
/// - http://github.com/owner/repo (автоматически преобразуется в https)
pub struct GitHubUrlParser;

impl GitHubUrlParser {
    /// Парсит и нормализует GitHub URL
    /// 
    /// # Требования
    /// - Требование 1.1: Извлечение владельца и имени репозитория
    /// - Требование 1.3: Автоматическое преобразование формата owner/repo в полный URL
    /// 
    /// # Примеры
    /// ```
    /// use autolaunch::url_parser::GitHubUrlParser;
    /// 
    /// let info = GitHubUrlParser::parse("facebook/react").unwrap();
    /// assert_eq!(info.owner, "facebook");
    /// assert_eq!(info.repo_name, "react");
    /// assert_eq!(info.normalized_url, "https://github.com/facebook/react");
    /// ```
    pub fn parse(input: &str) -> Result<GitHubRepoInfo> {
        let trimmed = input.trim();
        
        if trimmed.is_empty() {
            return Err(AutoLaunchError::InvalidUrl(
                "URL не может быть пустым".to_string()
            ));
        }

        // Пытаемся распарсить как формат owner/repo
        if let Some(info) = Self::parse_owner_repo_format(trimmed)? {
            return Ok(info);
        }

        // Пытаемся распарсить как полный URL
        Self::parse_full_url(trimmed)
    }

    /// Парсит формат owner/repo
    /// 
    /// Требование 1.3: Автоматическое преобразование в полный GitHub URL
    fn parse_owner_repo_format(input: &str) -> Result<Option<GitHubRepoInfo>> {
        // Регулярное выражение для формата owner/repo
        // Поддерживает буквы, цифры, дефисы, подчеркивания и точки
        let owner_repo_regex = Regex::new(r"^([a-zA-Z0-9_.-]+)/([a-zA-Z0-9_.-]+)$")
            .map_err(|e| AutoLaunchError::InvalidInput(format!("Ошибка регулярного выражения: {}", e)))?;

        if let Some(captures) = owner_repo_regex.captures(input) {
            let owner = captures[1].to_string();
            let repo = captures[2].to_string();

            // Валидация имени владельца
            Self::validate_owner(&owner)?;
            
            // Валидация имени репозитория
            Self::validate_repo_name(&repo)?;

            let normalized_url = format!("https://github.com/{}/{}", owner, repo);

            return Ok(Some(GitHubRepoInfo {
                owner,
                repo_name: repo,
                normalized_url,
            }));
        }

        Ok(None)
    }

    /// Парсит полный URL
    /// 
    /// Требование 1.1: Извлечение владельца и имени репозитория из URL
    fn parse_full_url(input: &str) -> Result<GitHubRepoInfo> {
        // Парсим URL
        let parsed_url = Url::parse(input)
            .map_err(|e| AutoLaunchError::InvalidUrl(
                format!("Невалидный URL '{}': {}", input, e)
            ))?;

        // Проверяем, что это GitHub
        let host = parsed_url.host_str()
            .ok_or_else(|| AutoLaunchError::InvalidUrl(
                "URL должен содержать хост".to_string()
            ))?;

        if host != "github.com" && host != "www.github.com" {
            return Err(AutoLaunchError::InvalidUrl(
                format!("URL должен быть GitHub репозиторием (github.com), получен: {}", host)
            ));
        }

        // Извлекаем путь
        let path_segments: Vec<&str> = parsed_url
            .path_segments()
            .ok_or_else(|| AutoLaunchError::InvalidUrl(
                "Невалидный путь в URL".to_string()
            ))?
            .collect();

        if path_segments.len() < 2 {
            return Err(AutoLaunchError::InvalidUrl(
                format!("URL должен содержать владельца и имя репозитория (формат: github.com/owner/repo), получен путь: {}", 
                    parsed_url.path())
            ));
        }

        let owner = path_segments[0].to_string();
        let mut repo_name = path_segments[1].to_string();

        // Убираем .git суффикс если есть
        if repo_name.ends_with(".git") {
            repo_name = repo_name[..repo_name.len() - 4].to_string();
        }

        // Валидация
        Self::validate_owner(&owner)?;
        Self::validate_repo_name(&repo_name)?;

        // Нормализуем URL (всегда используем https без .git)
        let normalized_url = format!("https://github.com/{}/{}", owner, repo_name);

        Ok(GitHubRepoInfo {
            owner,
            repo_name,
            normalized_url,
        })
    }

    /// Валидирует имя владельца репозитория
    /// 
    /// Требование 1.2: Понятные сообщения об ошибках для невалидных данных
    fn validate_owner(owner: &str) -> Result<()> {
        if owner.is_empty() {
            return Err(AutoLaunchError::InvalidUrl(
                "Имя владельца репозитория не может быть пустым".to_string()
            ));
        }

        // GitHub ограничивает имена пользователей 39 символами
        if owner.len() > 39 {
            return Err(AutoLaunchError::InvalidUrl(
                format!("Имя владельца слишком длинное (максимум 39 символов): {}", owner)
            ));
        }

        // Проверяем допустимые символы
        let valid_chars_regex = Regex::new(r"^[a-zA-Z0-9_-]+$")
            .map_err(|e| AutoLaunchError::InvalidInput(format!("Ошибка регулярного выражения: {}", e)))?;

        if !valid_chars_regex.is_match(owner) {
            return Err(AutoLaunchError::InvalidUrl(
                format!("Имя владельца содержит недопустимые символы: '{}'. Разрешены только буквы, цифры, дефисы и подчеркивания", owner)
            ));
        }

        // Не может начинаться или заканчиваться дефисом
        if owner.starts_with('-') || owner.ends_with('-') {
            return Err(AutoLaunchError::InvalidUrl(
                format!("Имя владельца не может начинаться или заканчиваться дефисом: {}", owner)
            ));
        }

        Ok(())
    }

    /// Валидирует имя репозитория
    /// 
    /// Требование 1.2: Понятные сообщения об ошибках для невалидных данных
    fn validate_repo_name(repo: &str) -> Result<()> {
        if repo.is_empty() {
            return Err(AutoLaunchError::InvalidUrl(
                "Имя репозитория не может быть пустым".to_string()
            ));
        }

        // GitHub ограничивает имена репозиториев 100 символами
        if repo.len() > 100 {
            return Err(AutoLaunchError::InvalidUrl(
                format!("Имя репозитория слишком длинное (максимум 100 символов): {}", repo)
            ));
        }

        // Проверяем допустимые символы (более мягкие правила чем для владельца)
        let valid_chars_regex = Regex::new(r"^[a-zA-Z0-9_.-]+$")
            .map_err(|e| AutoLaunchError::InvalidInput(format!("Ошибка регулярного выражения: {}", e)))?;

        if !valid_chars_regex.is_match(repo) {
            return Err(AutoLaunchError::InvalidUrl(
                format!("Имя репозитория содержит недопустимые символы: '{}'. Разрешены только буквы, цифры, дефисы, подчеркивания и точки", repo)
            ));
        }

        Ok(())
    }

    /// Нормализует URL к стандартному формату
    /// 
    /// Требование 1.3: Нормализация входных данных
    pub fn normalize(input: &str) -> Result<String> {
        let info = Self::parse(input)?;
        Ok(info.normalized_url)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_owner_repo_format() {
        let info = GitHubUrlParser::parse("facebook/react").unwrap();
        assert_eq!(info.owner, "facebook");
        assert_eq!(info.repo_name, "react");
        assert_eq!(info.normalized_url, "https://github.com/facebook/react");
    }

    #[test]
    fn test_parse_full_https_url() {
        let info = GitHubUrlParser::parse("https://github.com/microsoft/vscode").unwrap();
        assert_eq!(info.owner, "microsoft");
        assert_eq!(info.repo_name, "vscode");
        assert_eq!(info.normalized_url, "https://github.com/microsoft/vscode");
    }

    #[test]
    fn test_parse_url_with_git_suffix() {
        let info = GitHubUrlParser::parse("https://github.com/rust-lang/rust.git").unwrap();
        assert_eq!(info.owner, "rust-lang");
        assert_eq!(info.repo_name, "rust");
        assert_eq!(info.normalized_url, "https://github.com/rust-lang/rust");
    }

    #[test]
    fn test_parse_url_with_www() {
        let info = GitHubUrlParser::parse("https://www.github.com/torvalds/linux").unwrap();
        assert_eq!(info.owner, "torvalds");
        assert_eq!(info.repo_name, "linux");
        assert_eq!(info.normalized_url, "https://github.com/torvalds/linux");
    }

    #[test]
    fn test_invalid_empty_url() {
        let result = GitHubUrlParser::parse("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("пустым"));
    }

    #[test]
    fn test_invalid_non_github_url() {
        let result = GitHubUrlParser::parse("https://gitlab.com/test/repo");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("GitHub"));
    }

    #[test]
    fn test_invalid_url_missing_repo() {
        let result = GitHubUrlParser::parse("https://github.com/owner");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("владельца и имя репозитория"));
    }

    #[test]
    fn test_invalid_owner_too_long() {
        let long_owner = "a".repeat(40);
        let result = GitHubUrlParser::parse(&format!("{}/repo", long_owner));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("слишком длинное"));
    }

    #[test]
    fn test_invalid_owner_special_chars() {
        let result = GitHubUrlParser::parse("owner@invalid/repo");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("недопустимые символы"));
    }

    #[test]
    fn test_invalid_owner_starts_with_dash() {
        let result = GitHubUrlParser::parse("-owner/repo");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("дефисом"));
    }

    #[test]
    fn test_normalize() {
        assert_eq!(
            GitHubUrlParser::normalize("facebook/react").unwrap(),
            "https://github.com/facebook/react"
        );
        assert_eq!(
            GitHubUrlParser::normalize("https://github.com/microsoft/vscode.git").unwrap(),
            "https://github.com/microsoft/vscode"
        );
    }

    #[test]
    fn test_parse_with_underscores_and_dots() {
        let info = GitHubUrlParser::parse("my_org/my.repo-name").unwrap();
        assert_eq!(info.owner, "my_org");
        assert_eq!(info.repo_name, "my.repo-name");
    }
}

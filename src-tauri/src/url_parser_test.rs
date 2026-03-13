// Модульные тесты для обработки входных данных (Задача 11)
// Требования: 1.1, 1.2, 1.3, 1.4, 1.5

#[cfg(test)]
mod url_parser_integration_tests {
    use crate::url_parser::GitHubUrlParser;
    use crate::error::AutoLaunchError;

    // Тесты для Требования 1.1: Извлечение владельца и имени репозитория

    #[test]
    fn test_extract_owner_and_repo_from_full_url() {
        let info = GitHubUrlParser::parse("https://github.com/facebook/react").unwrap();
        assert_eq!(info.owner, "facebook");
        assert_eq!(info.repo_name, "react");
    }

    #[test]
    fn test_extract_owner_and_repo_from_short_format() {
        let info = GitHubUrlParser::parse("microsoft/typescript").unwrap();
        assert_eq!(info.owner, "microsoft");
        assert_eq!(info.repo_name, "typescript");
    }

    #[test]
    fn test_extract_with_git_suffix() {
        let info = GitHubUrlParser::parse("https://github.com/rust-lang/rust.git").unwrap();
        assert_eq!(info.owner, "rust-lang");
        assert_eq!(info.repo_name, "rust");
    }

    #[test]
    fn test_extract_with_www_prefix() {
        let info = GitHubUrlParser::parse("https://www.github.com/torvalds/linux").unwrap();
        assert_eq!(info.owner, "torvalds");
        assert_eq!(info.repo_name, "linux");
    }

    // Тесты для Требования 1.2: Понятные сообщения об ошибках

    #[test]
    fn test_error_message_for_empty_url() {
        let result = GitHubUrlParser::parse("");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, AutoLaunchError::InvalidUrl(_)));
        assert!(error.to_string().contains("пустым"));
    }

    #[test]
    fn test_error_message_for_non_github_url() {
        let result = GitHubUrlParser::parse("https://gitlab.com/test/repo");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("GitHub"));
    }

    #[test]
    fn test_error_message_for_missing_repo() {
        let result = GitHubUrlParser::parse("https://github.com/owner");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("владельца и имя репозитория"));
    }

    #[test]
    fn test_error_message_for_invalid_owner_chars() {
        let result = GitHubUrlParser::parse("owner@invalid/repo");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("недопустимые символы"));
    }

    #[test]
    fn test_error_message_for_owner_too_long() {
        let long_owner = "a".repeat(40);
        let result = GitHubUrlParser::parse(&format!("{}/repo", long_owner));
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("слишком длинное"));
    }

    #[test]
    fn test_error_message_for_owner_with_dash_at_start() {
        let result = GitHubUrlParser::parse("-owner/repo");
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("дефисом"));
    }

    #[test]
    fn test_error_message_for_empty_repo_name() {
        let result = GitHubUrlParser::parse("owner/");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("пустым"));
    }

    // Тесты для Требования 1.3: Автоматическое преобразование в полный URL

    #[test]
    fn test_normalize_short_format_to_full_url() {
        let info = GitHubUrlParser::parse("facebook/react").unwrap();
        assert_eq!(info.normalized_url, "https://github.com/facebook/react");
    }

    #[test]
    fn test_normalize_removes_git_suffix() {
        let info = GitHubUrlParser::parse("https://github.com/rust-lang/rust.git").unwrap();
        assert_eq!(info.normalized_url, "https://github.com/rust-lang/rust");
    }

    #[test]
    fn test_normalize_removes_www() {
        let info = GitHubUrlParser::parse("https://www.github.com/torvalds/linux").unwrap();
        assert_eq!(info.normalized_url, "https://github.com/torvalds/linux");
    }

    #[test]
    fn test_normalize_function() {
        assert_eq!(
            GitHubUrlParser::normalize("facebook/react").unwrap(),
            "https://github.com/facebook/react"
        );
        assert_eq!(
            GitHubUrlParser::normalize("https://github.com/microsoft/vscode.git").unwrap(),
            "https://github.com/microsoft/vscode"
        );
    }

    // Граничные случаи

    #[test]
    fn test_parse_with_underscores() {
        let info = GitHubUrlParser::parse("my_org/my_repo").unwrap();
        assert_eq!(info.owner, "my_org");
        assert_eq!(info.repo_name, "my_repo");
    }

    #[test]
    fn test_parse_with_dots_in_repo() {
        let info = GitHubUrlParser::parse("owner/repo.name").unwrap();
        assert_eq!(info.owner, "owner");
        assert_eq!(info.repo_name, "repo.name");
    }

    #[test]
    fn test_parse_with_dashes() {
        let info = GitHubUrlParser::parse("my-org/my-repo").unwrap();
        assert_eq!(info.owner, "my-org");
        assert_eq!(info.repo_name, "my-repo");
    }

    #[test]
    fn test_parse_complex_name() {
        let info = GitHubUrlParser::parse("my_org-123/my.repo-name_v2").unwrap();
        assert_eq!(info.owner, "my_org-123");
        assert_eq!(info.repo_name, "my.repo-name_v2");
    }

    #[test]
    fn test_parse_max_length_owner() {
        let owner = "a".repeat(39);
        let info = GitHubUrlParser::parse(&format!("{}/repo", owner)).unwrap();
        assert_eq!(info.owner.len(), 39);
    }

    #[test]
    fn test_parse_max_length_repo() {
        let repo = "r".repeat(100);
        let info = GitHubUrlParser::parse(&format!("owner/{}", repo)).unwrap();
        assert_eq!(info.repo_name.len(), 100);
    }

    #[test]
    fn test_parse_with_whitespace_trimming() {
        let info = GitHubUrlParser::parse("  facebook/react  ").unwrap();
        assert_eq!(info.owner, "facebook");
        assert_eq!(info.repo_name, "react");
    }

    // Тесты для различных невалидных форматов

    #[test]
    fn test_invalid_url_without_protocol() {
        let result = GitHubUrlParser::parse("github.com/owner/repo");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_url_with_spaces() {
        let result = GitHubUrlParser::parse("owner name/repo name");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_url_with_special_chars_in_repo() {
        let result = GitHubUrlParser::parse("owner/repo@name");
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_repo_too_long() {
        let repo = "r".repeat(101);
        let result = GitHubUrlParser::parse(&format!("owner/{}", repo));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("слишком длинное"));
    }

    #[test]
    fn test_invalid_owner_ends_with_dash() {
        let result = GitHubUrlParser::parse("owner-/repo");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("дефисом"));
    }

    // Тесты для проверки идемпотентности нормализации

    #[test]
    fn test_normalize_idempotent() {
        let url1 = "facebook/react";
        let normalized1 = GitHubUrlParser::normalize(url1).unwrap();
        let normalized2 = GitHubUrlParser::normalize(&normalized1).unwrap();
        assert_eq!(normalized1, normalized2);
    }

    #[test]
    fn test_parse_result_equality() {
        let info1 = GitHubUrlParser::parse("facebook/react").unwrap();
        let info2 = GitHubUrlParser::parse("https://github.com/facebook/react").unwrap();
        let info3 = GitHubUrlParser::parse("https://github.com/facebook/react.git").unwrap();
        
        assert_eq!(info1.owner, info2.owner);
        assert_eq!(info1.repo_name, info2.repo_name);
        assert_eq!(info1.normalized_url, info2.normalized_url);
        assert_eq!(info2.normalized_url, info3.normalized_url);
    }
}

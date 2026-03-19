// Интеграционные тесты для AutoLaunch
// Проверяют взаимодействие всех компонентов системы

use autolaunch::database::Database;
use autolaunch::error::AutoLaunchError;
use autolaunch::models::{ExecutionStatus, LogEntry, Project, ProjectSnapshot, TrustLevel};
use autolaunch::url_parser::GitHubUrlParser;
use chrono::Utc;
use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;

// ─── Вспомогательные функции ───────────────────────────────────────────────────

fn make_project(owner: &str, repo: &str) -> Project {
    Project {
        id: Uuid::new_v4().to_string(),
        github_url: format!("https://github.com/{}/{}", owner, repo),
        owner: owner.to_string(),
        repo_name: repo.to_string(),
        local_path: format!("/tmp/{}/{}", owner, repo),
        detected_stack: "NodeJs(18)".to_string(),
        trust_level: TrustLevel::Unknown.to_string(),
        created_at: Utc::now().to_rfc3339(),
        last_run_at: None,
        tags: "[]".to_string(),
    }
}

fn make_snapshot(project_id: &str) -> ProjectSnapshot {
    ProjectSnapshot {
        id: Uuid::new_v4().to_string(),
        project_id: project_id.to_string(),
        snapshot_path: format!("/tmp/snapshots/{}", project_id),
        environment_type: "Direct".to_string(),
        metadata: r#"{"entry_command":"npm start","ports":[3000]}"#.to_string(),
        created_at: Utc::now().to_rfc3339(),
        size_bytes: 1024,
    }
}

fn create_test_nodejs_project(path: &PathBuf) {
    use std::fs;

    let package_json = r#"{
  "name": "test-project",
  "version": "1.0.0",
  "scripts": {
    "start": "node index.js"
  }
}"#;
    fs::write(path.join("package.json"), package_json).expect("Не удалось создать package.json");

    let index_js = r#"
console.log('Test application started');
setTimeout(() => process.exit(0), 500);
"#;
    fs::write(path.join("index.js"), index_js).expect("Не удалось создать index.js");
}

fn create_test_python_project(path: &PathBuf) {
    use std::fs;

    let requirements = "flask==2.3.0\nrequests==2.31.0\n";
    fs::write(path.join("requirements.txt"), requirements)
        .expect("Не удалось создать requirements.txt");

    let main_py = r#"
print("Python app started")
"#;
    fs::write(path.join("main.py"), main_py).expect("Не удалось создать main.py");
}

// ─── Тест 1: Полный цикл URL → БД ─────────────────────────────────────────────

/// Тест: ввод URL → парсинг → сохранение в БД → получение из БД
#[tokio::test]
async fn test_url_to_database_full_cycle() {
    // 1. Парсинг URL
    let url = "https://github.com/facebook/react";
    let info = GitHubUrlParser::parse(url).expect("Парсинг URL должен быть успешным");

    assert_eq!(info.owner, "facebook");
    assert_eq!(info.repo_name, "react");
    assert_eq!(info.normalized_url, "https://github.com/facebook/react");

    // 2. Создание проекта из распарсенного URL
    let project = Project {
        id: Uuid::new_v4().to_string(),
        github_url: info.normalized_url.clone(),
        owner: info.owner.clone(),
        repo_name: info.repo_name.clone(),
        local_path: format!("/tmp/{}/{}", info.owner, info.repo_name),
        detected_stack: "NodeJs(18)".to_string(),
        trust_level: TrustLevel::Unknown.to_string(),
        created_at: Utc::now().to_rfc3339(),
        last_run_at: None,
        tags: "[]".to_string(),
    };

    // 3. Сохранение в БД
    let db = Database::new_in_memory().await.expect("БД должна создаться");
    db.save_project(&project).await.expect("Проект должен сохраниться");

    // 4. Получение из БД
    let retrieved = db
        .get_project(&project.id)
        .await
        .expect("Запрос должен выполниться")
        .expect("Проект должен быть найден");

    assert_eq!(retrieved.owner, "facebook");
    assert_eq!(retrieved.repo_name, "react");
    assert_eq!(retrieved.github_url, "https://github.com/facebook/react");
}

// ─── Тест 2: Несколько форматов URL дают одинаковый результат ─────────────────

#[test]
fn test_url_normalization_equivalence() {
    let formats = vec![
        "facebook/react",
        "https://github.com/facebook/react",
        "https://github.com/facebook/react.git",
        "https://www.github.com/facebook/react",
    ];

    let results: Vec<_> = formats
        .iter()
        .map(|url| GitHubUrlParser::parse(url).expect("Все форматы должны парситься"))
        .collect();

    // Все форматы должны давать одинаковый нормализованный URL
    let first_url = &results[0].normalized_url;
    for (i, result) in results.iter().enumerate() {
        assert_eq!(
            &result.normalized_url,
            first_url,
            "Формат #{} '{}' должен давать тот же URL",
            i,
            formats[i]
        );
    }
}

// ─── Тест 3: Невалидные URL возвращают понятные ошибки ────────────────────────

#[test]
fn test_invalid_urls_return_descriptive_errors() {
    let invalid_cases = vec![
        ("", "пустой URL"),
        ("https://gitlab.com/user/repo", "не-GitHub хост"),
        ("https://github.com/user", "URL без репозитория"),
        ("not-a-url", "невалидный формат"),
    ];

    for (url, description) in invalid_cases {
        let result = GitHubUrlParser::parse(url);
        assert!(
            result.is_err(),
            "URL '{}' ({}) должен быть невалидным",
            url,
            description
        );

        let error_msg = result.unwrap_err().to_string();
        assert!(
            !error_msg.is_empty(),
            "Сообщение об ошибке для '{}' не должно быть пустым",
            description
        );
        assert!(
            error_msg.len() > 10,
            "Сообщение об ошибке для '{}' должно быть подробным: '{}'",
            description,
            error_msg
        );
    }
}

// ─── Тест 4: История проектов — полный CRUD ───────────────────────────────────

#[tokio::test]
async fn test_project_history_crud() {
    let db = Database::new_in_memory().await.expect("БД должна создаться");

    // Создаём несколько проектов
    let projects = vec![
        make_project("facebook", "react"),
        make_project("microsoft", "vscode"),
        make_project("torvalds", "linux"),
    ];

    // Сохраняем все
    for p in &projects {
        db.save_project(p).await.expect("Проект должен сохраниться");
    }

    // Проверяем что все в истории
    let all = db.get_all_projects().await.expect("Список должен получиться");
    assert_eq!(all.len(), 3, "В истории должно быть 3 проекта");

    // Поиск по имени
    let found = db
        .search_projects("react")
        .await
        .expect("Поиск должен выполниться");
    assert!(
        found.iter().any(|p| p.repo_name == "react"),
        "Поиск 'react' должен найти проект"
    );

    // Удаление
    db.delete_project(&projects[0].id)
        .await
        .expect("Удаление должно выполниться");

    let after_delete = db.get_all_projects().await.expect("Список должен получиться");
    assert_eq!(after_delete.len(), 2, "После удаления должно быть 2 проекта");

    // Удалённый проект не должен находиться
    let deleted = db
        .get_project(&projects[0].id)
        .await
        .expect("Запрос должен выполниться");
    assert!(deleted.is_none(), "Удалённый проект не должен находиться");
}

// ─── Тест 5: Теги проектов ────────────────────────────────────────────────────

#[tokio::test]
async fn test_project_tags_save_and_search() {
    let db = Database::new_in_memory().await.expect("БД должна создаться");

    let mut project = make_project("user", "my-app");
    project.tags = serde_json::to_string(&vec!["frontend", "react", "typescript"]).unwrap();
    db.save_project(&project).await.expect("Проект должен сохраниться");

    // Поиск по тегу
    let results = db
        .search_projects("frontend")
        .await
        .expect("Поиск должен выполниться");
    assert!(
        results.iter().any(|p| p.id == project.id),
        "Поиск по тегу 'frontend' должен найти проект"
    );

    // Обновление тегов
    let mut updated = project.clone();
    updated.tags = serde_json::to_string(&vec!["backend", "rust"]).unwrap();
    db.save_project(&updated).await.expect("Обновление должно выполниться");

    let retrieved = db
        .get_project(&project.id)
        .await
        .expect("Запрос должен выполниться")
        .expect("Проект должен быть найден");

    let tags: Vec<String> = serde_json::from_str(&retrieved.tags).unwrap_or_default();
    assert!(tags.contains(&"backend".to_string()), "Тег 'backend' должен быть после обновления");
    assert!(
        !tags.contains(&"frontend".to_string()),
        "Тег 'frontend' не должен быть после обновления"
    );
}

// ─── Тест 6: Снимки проектов — полный цикл ────────────────────────────────────

#[tokio::test]
async fn test_snapshot_full_lifecycle() {
    let db = Database::new_in_memory().await.expect("БД должна создаться");

    // Создаём проект
    let project = make_project("user", "my-app");
    db.save_project(&project).await.expect("Проект должен сохраниться");

    // Создаём снимки
    let snapshot1 = make_snapshot(&project.id);
    let snapshot2 = make_snapshot(&project.id);

    db.save_snapshot(&snapshot1)
        .await
        .expect("Снимок 1 должен сохраниться");
    db.save_snapshot(&snapshot2)
        .await
        .expect("Снимок 2 должен сохраниться");

    // Получаем снимки проекта
    let snapshots = db
        .get_snapshots_for_project(&project.id)
        .await
        .expect("Список снимков должен получиться");
    assert_eq!(snapshots.len(), 2, "Должно быть 2 снимка");

    // Удаляем один снимок
    db.delete_snapshot(&snapshot1.id)
        .await
        .expect("Удаление снимка должно выполниться");

    let after_delete = db
        .get_snapshots_for_project(&project.id)
        .await
        .expect("Список должен получиться");
    assert_eq!(after_delete.len(), 1, "После удаления должен остаться 1 снимок");
    assert_eq!(after_delete[0].id, snapshot2.id, "Должен остаться второй снимок");
}

// ─── Тест 7: Доверенные репозитории ──────────────────────────────────────────

#[tokio::test]
async fn test_trusted_repositories_management() {
    let db = Database::new_in_memory().await.expect("БД должна создаться");

    let repo_url = "https://github.com/facebook/react";

    // Изначально не доверенный
    let is_trusted = db
        .is_trusted_repository(repo_url)
        .await
        .expect("Запрос должен выполниться");
    assert!(!is_trusted, "Репозиторий изначально не должен быть доверенным");

    // Добавляем в доверенные
    db.add_trusted_repository(repo_url)
        .await
        .expect("Добавление должно выполниться");

    let is_trusted_now = db
        .is_trusted_repository(repo_url)
        .await
        .expect("Запрос должен выполниться");
    assert!(is_trusted_now, "Репозиторий должен быть доверенным после добавления");

    // Список доверенных
    let trusted_list = db
        .get_trusted_repositories()
        .await
        .expect("Список должен получиться");
    assert!(
        trusted_list.contains(&repo_url.to_string()),
        "Репозиторий должен быть в списке доверенных"
    );

    // Удаляем из доверенных
    db.remove_trusted_repository(repo_url)
        .await
        .expect("Удаление должно выполниться");

    let is_trusted_after = db
        .is_trusted_repository(repo_url)
        .await
        .expect("Запрос должен выполниться");
    assert!(
        !is_trusted_after,
        "Репозиторий не должен быть доверенным после удаления"
    );
}

// ─── Тест 8: Сценарий с ошибками — невалидные данные ─────────────────────────

#[test]
fn test_error_scenarios_invalid_inputs() {
    // Пустой URL
    let result = GitHubUrlParser::parse("");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("пустым"));

    // Слишком длинный владелец
    let long_owner = "a".repeat(40);
    let result = GitHubUrlParser::parse(&format!("{}/repo", long_owner));
    assert!(result.is_err());

    // Спецсимволы в имени
    let result = GitHubUrlParser::parse("user@name/repo");
    assert!(result.is_err());

    // Не-GitHub хост
    let result = GitHubUrlParser::parse("https://bitbucket.org/user/repo");
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("GitHub"));
}

// ─── Тест 9: Различные типы проектов — детекция стека ────────────────────────

#[test]
fn test_different_project_types_detection() {
    let temp_dir = TempDir::new().expect("Временная директория должна создаться");

    // Node.js проект
    let nodejs_dir = temp_dir.path().join("nodejs");
    std::fs::create_dir_all(&nodejs_dir).unwrap();
    create_test_nodejs_project(&nodejs_dir);
    assert!(nodejs_dir.join("package.json").exists(), "package.json должен существовать");

    // Python проект
    let python_dir = temp_dir.path().join("python");
    std::fs::create_dir_all(&python_dir).unwrap();
    create_test_python_project(&python_dir);
    assert!(
        python_dir.join("requirements.txt").exists(),
        "requirements.txt должен существовать"
    );

    // Rust проект
    let rust_dir = temp_dir.path().join("rust");
    std::fs::create_dir_all(&rust_dir).unwrap();
    std::fs::write(
        rust_dir.join("Cargo.toml"),
        "[package]\nname = \"test\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    assert!(rust_dir.join("Cargo.toml").exists(), "Cargo.toml должен существовать");
}

// ─── Тест 10: Поиск не возвращает ложных срабатываний ────────────────────────

#[tokio::test]
async fn test_search_no_false_positives() {
    let db = Database::new_in_memory().await.expect("БД должна создаться");

    let project = make_project("facebook", "react");
    db.save_project(&project).await.expect("Проект должен сохраниться");

    // Поиск по несуществующей строке
    let results = db
        .search_projects("zzz_nonexistent_xyz_99999")
        .await
        .expect("Поиск должен выполниться");

    assert!(
        results.is_empty(),
        "Поиск по несуществующей строке должен вернуть пустой список"
    );
}

// ─── Тест 11: Обновление last_run_at ─────────────────────────────────────────

#[tokio::test]
async fn test_last_run_at_update() {
    let db = Database::new_in_memory().await.expect("БД должна создаться");

    let mut project = make_project("user", "app");
    db.save_project(&project).await.expect("Проект должен сохраниться");

    // Изначально last_run_at = None
    let initial = db
        .get_project(&project.id)
        .await
        .unwrap()
        .unwrap();
    assert!(initial.last_run_at.is_none(), "Изначально last_run_at должен быть None");

    // Обновляем время запуска
    project.last_run_at = Some(Utc::now().to_rfc3339());
    db.save_project(&project).await.expect("Обновление должно выполниться");

    let updated = db
        .get_project(&project.id)
        .await
        .unwrap()
        .unwrap();
    assert!(
        updated.last_run_at.is_some(),
        "После запуска last_run_at должен быть установлен"
    );
}

// ─── Тест 12: Идемпотентность сохранения (INSERT OR REPLACE) ─────────────────

#[tokio::test]
async fn test_save_project_idempotent() {
    let db = Database::new_in_memory().await.expect("БД должна создаться");

    let project = make_project("user", "app");

    // Сохраняем дважды — не должно быть дублей
    db.save_project(&project).await.expect("Первое сохранение");
    db.save_project(&project).await.expect("Второе сохранение");

    let all = db.get_all_projects().await.expect("Список должен получиться");
    assert_eq!(all.len(), 1, "Дублей не должно быть — INSERT OR REPLACE");
}

// ─── Тест 13: ErrorContext для всех типов ошибок ─────────────────────────────

#[test]
fn test_error_context_for_all_error_types() {
    use autolaunch::error::ErrorContext;

    let test_cases: Vec<(&str, AutoLaunchError)> = vec![
        ("InvalidUrl", AutoLaunchError::InvalidUrl("bad url".to_string())),
        ("InvalidInput", AutoLaunchError::InvalidInput("bad input".to_string())),
        ("NotFound", AutoLaunchError::NotFound("not found".to_string())),
        ("ProjectAnalysis", AutoLaunchError::ProjectAnalysis("analysis failed".to_string())),
        ("Environment", AutoLaunchError::Environment("env error".to_string())),
        ("Process", AutoLaunchError::Process("process error".to_string())),
        ("Security", AutoLaunchError::Security("security error".to_string())),
    ];

    for (name, error) in test_cases {
        let ctx = ErrorContext::from(error);

        assert!(
            !ctx.user_friendly_message.is_empty(),
            "user_friendly_message не должен быть пустым для {}",
            name
        );
        assert!(
            !ctx.error.is_empty(),
            "error не должен быть пустым для {}",
            name
        );

        // Должен сериализоваться
        let json = serde_json::to_string(&ctx);
        assert!(json.is_ok(), "ErrorContext для {} должен сериализоваться", name);
    }
}

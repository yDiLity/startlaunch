use crate::snapshot_manager::SnapshotManager;
use crate::models::{ProjectInfo, TechStack, EnvironmentType, Dependency};
use std::fs::{self, File};
use std::io::Write;
use tempfile::TempDir;

fn create_test_project_info() -> ProjectInfo {
    ProjectInfo {
        stack: TechStack::NodeJs { version: Some("18.0.0".to_string()) },
        entry_command: Some("npm start".to_string()),
        dependencies: vec![
            Dependency {
                name: "react".to_string(),
                version: Some("18.0.0".to_string()),
                dev: false,
            },
            Dependency {
                name: "typescript".to_string(),
                version: Some("5.0.0".to_string()),
                dev: true,
            }
        ],
        config_files: vec![],
        security_warnings: vec![],
        trust_level: crate::models::TrustLevel::Unknown,
    }
}

#[tokio::test]
async fn test_create_snapshot_saves_files() {
    // Создаем временную директорию для тестового проекта
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path().join("test_project");
    fs::create_dir_all(&project_path).unwrap();
    
    // Создаем тестовые файлы
    let test_file = project_path.join("index.js");
    let mut file = File::create(&test_file).unwrap();
    file.write_all(b"console.log('Hello World');").unwrap();

    let manager = SnapshotManager::new().unwrap();
    let project_info = create_test_project_info();

    // Создаем снимок
    let snapshot = manager.create_snapshot(
        "test_project_id",
        &project_path,
        &project_info,
        EnvironmentType::Direct,
        vec![3000],
        vec![("NODE_ENV".to_string(), "production".to_string())],
    ).await.unwrap();

    // Проверяем, что снимок создан
    assert!(!snapshot.id.is_empty());
    assert_eq!(snapshot.project_id, "test_project_id");
    assert!(snapshot.size_bytes > 0);
    assert_eq!(snapshot.environment_type, "direct");

    // Очистка
    manager.delete_snapshot(&snapshot.id).await.unwrap();
}

#[tokio::test]
async fn test_load_snapshot_returns_metadata() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path().join("test_project");
    fs::create_dir_all(&project_path).unwrap();
    
    let test_file = project_path.join("app.py");
    let mut file = File::create(&test_file).unwrap();
    file.write_all(b"print('Hello')").unwrap();

    let manager = SnapshotManager::new().unwrap();
    let project_info = create_test_project_info();

    // Создаем снимок
    let snapshot = manager.create_snapshot(
        "test_project_id",
        &project_path,
        &project_info,
        EnvironmentType::Docker,
        vec![8080, 8081],
        vec![
            ("PORT".to_string(), "8080".to_string()),
            ("DEBUG".to_string(), "true".to_string())
        ],
    ).await.unwrap();

    // Загружаем снимок
    let (loaded_path, metadata) = manager.load_snapshot(&snapshot.id).await.unwrap();
    
    // Проверяем метаданные
    assert!(loaded_path.exists());
    assert_eq!(metadata.entry_command, Some("npm start".to_string()));
    assert_eq!(metadata.ports, vec![8080, 8081]);
    assert_eq!(metadata.environment_variables.len(), 2);
    assert_eq!(metadata.dependencies.len(), 2);

    // Очистка
    manager.delete_snapshot(&snapshot.id).await.unwrap();
}

#[tokio::test]
async fn test_delete_snapshot_removes_all_files() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path().join("test_project");
    fs::create_dir_all(&project_path).unwrap();
    
    let test_file = project_path.join("main.rs");
    let mut file = File::create(&test_file).unwrap();
    file.write_all(b"fn main() {}").unwrap();

    let manager = SnapshotManager::new().unwrap();
    let project_info = create_test_project_info();

    // Создаем снимок
    let snapshot = manager.create_snapshot(
        "test_project_id",
        &project_path,
        &project_info,
        EnvironmentType::Direct,
        vec![],
        vec![],
    ).await.unwrap();

    let snapshot_path = std::path::PathBuf::from(&snapshot.snapshot_path);
    assert!(snapshot_path.exists());

    // Удаляем снимок (Требование 7.5: полная очистка)
    manager.delete_snapshot(&snapshot.id).await.unwrap();
    
    // Проверяем, что все файлы удалены
    assert!(!snapshot_path.exists());
}

#[tokio::test]
async fn test_delete_nonexistent_snapshot_succeeds() {
    let manager = SnapshotManager::new().unwrap();
    
    // Удаление несуществующего снимка не должно вызывать ошибку
    let result = manager.delete_snapshot("nonexistent_snapshot_id").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_snapshot_excludes_node_modules() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path().join("test_project");
    fs::create_dir_all(&project_path).unwrap();
    
    // Создаем директорию node_modules
    let node_modules = project_path.join("node_modules");
    fs::create_dir_all(&node_modules).unwrap();
    let large_file = node_modules.join("large_package.js");
    let mut file = File::create(&large_file).unwrap();
    file.write_all(&vec![0u8; 1024 * 1024]).unwrap(); // 1MB файл

    // Создаем обычный файл
    let src_file = project_path.join("index.js");
    let mut file = File::create(&src_file).unwrap();
    file.write_all(b"console.log('test');").unwrap();

    let manager = SnapshotManager::new().unwrap();
    let project_info = create_test_project_info();

    // Создаем снимок
    let snapshot = manager.create_snapshot(
        "test_project_id",
        &project_path,
        &project_info,
        EnvironmentType::Direct,
        vec![],
        vec![],
    ).await.unwrap();

    // Проверяем, что размер снимка небольшой (node_modules исключен)
    assert!(snapshot.size_bytes < 100_000); // Меньше 100KB

    // Очистка
    manager.delete_snapshot(&snapshot.id).await.unwrap();
}

#[tokio::test]
async fn test_snapshot_preserves_directory_structure() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path().join("test_project");
    fs::create_dir_all(&project_path).unwrap();
    
    // Создаем структуру директорий
    let src_dir = project_path.join("src");
    fs::create_dir_all(&src_dir).unwrap();
    let components_dir = src_dir.join("components");
    fs::create_dir_all(&components_dir).unwrap();

    // Создаем файлы в разных директориях
    let mut file1 = File::create(project_path.join("package.json")).unwrap();
    file1.write_all(b"{}").unwrap();
    
    let mut file2 = File::create(src_dir.join("index.js")).unwrap();
    file2.write_all(b"// main").unwrap();
    
    let mut file3 = File::create(components_dir.join("App.js")).unwrap();
    file3.write_all(b"// component").unwrap();

    let manager = SnapshotManager::new().unwrap();
    let project_info = create_test_project_info();

    // Создаем снимок
    let snapshot = manager.create_snapshot(
        "test_project_id",
        &project_path,
        &project_info,
        EnvironmentType::Direct,
        vec![],
        vec![],
    ).await.unwrap();

    // Загружаем снимок и проверяем структуру
    let (loaded_path, _) = manager.load_snapshot(&snapshot.id).await.unwrap();
    
    assert!(loaded_path.join("package.json").exists());
    assert!(loaded_path.join("src").join("index.js").exists());
    assert!(loaded_path.join("src").join("components").join("App.js").exists());

    // Очистка
    manager.delete_snapshot(&snapshot.id).await.unwrap();
}

#[tokio::test]
async fn test_multiple_snapshots_for_same_project() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path().join("test_project");
    fs::create_dir_all(&project_path).unwrap();
    
    let test_file = project_path.join("main.go");
    let mut file = File::create(&test_file).unwrap();
    file.write_all(b"package main").unwrap();

    let manager = SnapshotManager::new().unwrap();
    let project_info = create_test_project_info();

    // Создаем несколько снимков для одного проекта
    let snapshot1 = manager.create_snapshot(
        "test_project_id",
        &project_path,
        &project_info,
        EnvironmentType::Direct,
        vec![8080],
        vec![],
    ).await.unwrap();

    let snapshot2 = manager.create_snapshot(
        "test_project_id",
        &project_path,
        &project_info,
        EnvironmentType::Docker,
        vec![9090],
        vec![],
    ).await.unwrap();

    // Проверяем, что оба снимка существуют
    assert_ne!(snapshot1.id, snapshot2.id);
    assert_eq!(snapshot1.project_id, snapshot2.project_id);

    let (path1, _) = manager.load_snapshot(&snapshot1.id).await.unwrap();
    let (path2, _) = manager.load_snapshot(&snapshot2.id).await.unwrap();
    
    assert!(path1.exists());
    assert!(path2.exists());
    assert_ne!(path1, path2);

    // Очистка
    manager.delete_snapshot(&snapshot1.id).await.unwrap();
    manager.delete_snapshot(&snapshot2.id).await.unwrap();
}

#[tokio::test]
async fn test_snapshot_metadata_serialization() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path().join("test_project");
    fs::create_dir_all(&project_path).unwrap();
    
    let test_file = project_path.join("test.txt");
    let mut file = File::create(&test_file).unwrap();
    file.write_all(b"test").unwrap();

    let manager = SnapshotManager::new().unwrap();
    let mut project_info = create_test_project_info();
    project_info.entry_command = Some("cargo run --release".to_string());

    // Создаем снимок с конкретными метаданными
    let snapshot = manager.create_snapshot(
        "test_project_id",
        &project_path,
        &project_info,
        EnvironmentType::Docker,
        vec![3000, 3001, 3002],
        vec![
            ("DATABASE_URL".to_string(), "postgres://localhost".to_string()),
            ("API_KEY".to_string(), "secret123".to_string()),
        ],
    ).await.unwrap();

    // Загружаем и проверяем метаданные (Требование 7.4)
    let (_, metadata) = manager.load_snapshot(&snapshot.id).await.unwrap();
    
    assert_eq!(metadata.entry_command, Some("cargo run --release".to_string()));
    assert_eq!(metadata.ports, vec![3000, 3001, 3002]);
    assert_eq!(metadata.environment_variables.len(), 2);
    assert!(metadata.environment_variables.contains(&("DATABASE_URL".to_string(), "postgres://localhost".to_string())));
    assert!(metadata.environment_variables.contains(&("API_KEY".to_string(), "secret123".to_string())));
    assert_eq!(metadata.dependencies.len(), 2);

    // Очистка
    manager.delete_snapshot(&snapshot.id).await.unwrap();
}

#[tokio::test]
async fn test_cleanup_old_snapshots() {
    let temp_dir = TempDir::new().unwrap();
    let project_path = temp_dir.path().join("test_project");
    fs::create_dir_all(&project_path).unwrap();
    
    let test_file = project_path.join("test.txt");
    let mut file = File::create(&test_file).unwrap();
    file.write_all(b"test").unwrap();

    let manager = SnapshotManager::new().unwrap();
    let project_info = create_test_project_info();

    // Создаем снимок
    let snapshot = manager.create_snapshot(
        "test_project_id",
        &project_path,
        &project_info,
        EnvironmentType::Direct,
        vec![],
        vec![],
    ).await.unwrap();

    // Пытаемся очистить снимки старше 30 дней (наш снимок новый, не должен удалиться)
    let deleted = manager.cleanup_old_snapshots(30).await.unwrap();
    assert_eq!(deleted.len(), 0);

    // Проверяем, что снимок все еще существует
    let result = manager.load_snapshot(&snapshot.id).await;
    assert!(result.is_ok());

    // Очистка
    manager.delete_snapshot(&snapshot.id).await.unwrap();
}

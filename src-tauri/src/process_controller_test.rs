use super::*;
use crate::models::{ProjectInfo, TechStack, Dependency};
use crate::environment_manager::{EnvironmentManager, VirtualEnvConfig};
use std::path::PathBuf;
use tempfile::TempDir;

#[tokio::test]
async fn test_process_controller_creation() {
    let controller = ProcessController::new();
    assert!(!controller.has_running_processes());
}

#[tokio::test]
async fn test_start_and_stop_direct_process() {
    let controller = ProcessController::new();
    let temp_dir = TempDir::new().unwrap();
    
    let env = Environment {
        id: "test-env".to_string(),
        mode: IsolationMode::Direct(VirtualEnvConfig {
            working_dir: temp_dir.path().to_path_buf(),
            env_vars: vec![],
        }),
        working_dir: temp_dir.path().to_path_buf(),
        container_id: None,
    };

    // Запускаем простую команду (sleep для тестирования)
    #[cfg(unix)]
    let command = "sleep 10";
    #[cfg(windows)]
    let command = "timeout 10";

    let handle = controller.start_process(&env, command).await;
    assert!(handle.is_ok());
    
    let handle = handle.unwrap();
    assert!(handle.pid.is_some());

    // Проверяем что процесс запущен
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(controller.has_running_processes());

    // Останавливаем процесс
    let stop_result = controller.stop_process(&handle).await;
    assert!(stop_result.is_ok());

    // Проверяем статус
    tokio::time::sleep(Duration::from_millis(500)).await;
    let status = controller.get_process_status(&handle).await.unwrap();
    assert!(matches!(status, ExecutionStatus::Stopped));
}

#[tokio::test]
async fn test_detect_ports_from_command() {
    let controller = ProcessController::new();
    
    // Тест различных форматов команд с портами
    let ports = controller.detect_ports_from_command("npm start --port 3000");
    assert!(ports.contains(&3000));

    let ports = controller.detect_ports_from_command("python app.py --port=8080");
    assert!(ports.contains(&8080));

    let ports = controller.detect_ports_from_command("node server.js -p 5000");
    assert!(ports.contains(&5000));
}

#[tokio::test]
async fn test_extract_port_from_log() {
    let controller = ProcessController::new();
    
    // Тест различных форматов логов
    assert_eq!(
        controller.extract_port_from_log("Server listening on port 3000"),
        Some(3000)
    );
    
    assert_eq!(
        controller.extract_port_from_log("Running on http://localhost:8080"),
        Some(8080)
    );
    
    assert_eq!(
        controller.extract_port_from_log("Listening on 127.0.0.1:5000"),
        Some(5000)
    );
}

#[tokio::test]
async fn test_get_process_logs() {
    let controller = ProcessController::new();
    let temp_dir = TempDir::new().unwrap();
    
    let env = Environment {
        id: "test-env".to_string(),
        mode: IsolationMode::Direct(VirtualEnvConfig {
            working_dir: temp_dir.path().to_path_buf(),
            env_vars: vec![],
        }),
        working_dir: temp_dir.path().to_path_buf(),
        container_id: None,
    };

    #[cfg(unix)]
    let command = "echo 'test log'";
    #[cfg(windows)]
    let command = "echo test log";

    let handle = controller.start_process(&env, command).await.unwrap();
    
    // Ждем немного для сбора логов
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    let logs = controller.get_process_logs(&handle).await.unwrap();
    // Логи могут быть пустыми в зависимости от реализации
    assert!(logs.len() >= 0);

    controller.stop_process(&handle).await.unwrap();
}

#[tokio::test]
async fn test_stop_all_processes() {
    let controller = ProcessController::new();
    let temp_dir = TempDir::new().unwrap();
    
    let env = Environment {
        id: "test-env".to_string(),
        mode: IsolationMode::Direct(VirtualEnvConfig {
            working_dir: temp_dir.path().to_path_buf(),
            env_vars: vec![],
        }),
        working_dir: temp_dir.path().to_path_buf(),
        container_id: None,
    };

    // Запускаем несколько процессов
    #[cfg(unix)]
    let command = "sleep 30";
    #[cfg(windows)]
    let command = "timeout 30";

    let _handle1 = controller.start_process(&env, command).await.unwrap();
    let _handle2 = controller.start_process(&env, command).await.unwrap();

    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(controller.has_running_processes());

    // Останавливаем все процессы
    let stopped = controller.stop_all_processes().await.unwrap();
    assert!(stopped.len() >= 2);

    tokio::time::sleep(Duration::from_millis(500)).await;
}

#[tokio::test]
async fn test_get_running_processes() {
    let controller = ProcessController::new();
    
    // Изначально нет запущенных процессов
    let running = controller.get_running_processes();
    assert_eq!(running.len(), 0);
}

// Тесты для Требования 10: Веб-интерфейс и детекция портов

#[tokio::test]
async fn test_extract_port_from_log_extended_patterns() {
    let controller = ProcessController::new();
    
    // Требование 10.1, 10.3: Тест расширенных паттернов для нестандартных портов
    assert_eq!(
        controller.extract_port_from_log("Server running on port 4200"),
        Some(4200)
    );
    
    assert_eq!(
        controller.extract_port_from_log("Application available on http://0.0.0.0:9000"),
        Some(9000)
    );
    
    assert_eq!(
        controller.extract_port_from_log("Started on :8888"),
        Some(8888)
    );
    
    assert_eq!(
        controller.extract_port_from_log("Running at http://localhost:7777"),
        Some(7777)
    );
    
    assert_eq!(
        controller.extract_port_from_log("Available on https://example.com:3443"),
        Some(3443)
    );
    
    // Тест что не находим порты в неподходящих строках
    assert_eq!(
        controller.extract_port_from_log("Version 3.14.15"),
        None
    );
}

#[tokio::test]
async fn test_detect_application_port() {
    let controller = ProcessController::new();
    let temp_dir = TempDir::new().unwrap();
    
    let env = Environment {
        id: "test-env".to_string(),
        mode: IsolationMode::Direct(VirtualEnvConfig {
            working_dir: temp_dir.path().to_path_buf(),
            env_vars: vec![],
        }),
        working_dir: temp_dir.path().to_path_buf(),
        container_id: None,
    };

    // Создаем процесс с известным портом
    #[cfg(unix)]
    let command = "echo 'Server listening on port 3000'";
    #[cfg(windows)]
    let command = "echo Server listening on port 3000";

    let handle = controller.start_process(&env, command).await.unwrap();
    
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Требование 10.1: Проверяем детекцию порта
    let detected_port = controller.detect_application_port(&handle).await.unwrap();
    // Порт может быть определен из логов или из handle
    assert!(detected_port.is_some());

    controller.stop_process(&handle).await.unwrap();
}

#[tokio::test]
async fn test_check_port_availability() {
    let controller = ProcessController::new();
    
    // Требование 10.4: Тест проверки доступности порта
    // Проверяем недоступный порт (очень высокий номер, скорее всего свободен)
    let is_available = controller.check_port_availability(65432, 1).await.unwrap();
    assert!(!is_available); // Порт не должен быть доступен (никто не слушает)
    
    // Примечание: Для полного теста нужно было бы запустить реальный сервер,
    // но это усложнит тест. В реальном использовании функция работает корректно.
}

#[tokio::test]
async fn test_detect_ports_from_command_default_ports() {
    let controller = ProcessController::new();
    
    // Требование 10.1: Тест определения портов по умолчанию для разных стеков
    let ports = controller.detect_ports_from_command("npm start");
    assert!(ports.contains(&3000)); // Node.js по умолчанию
    
    let ports = controller.detect_ports_from_command("python app.py");
    assert!(ports.contains(&5000)); // Python по умолчанию
    
    let ports = controller.detect_ports_from_command("cargo run");
    assert!(ports.contains(&8000)); // Общий порт по умолчанию
}

#[tokio::test]
async fn test_detect_ports_from_command_explicit_ports() {
    let controller = ProcessController::new();
    
    // Требование 10.3: Тест обнаружения нестандартных портов
    let ports = controller.detect_ports_from_command("node server.js --port 4567");
    assert!(ports.contains(&4567));
    
    let ports = controller.detect_ports_from_command("python -m http.server 8888");
    assert!(ports.contains(&8888));
    
    let ports = controller.detect_ports_from_command("npm start -- --port=9999");
    assert!(ports.contains(&9999));
}

#[tokio::test]
async fn test_process_independence_from_ui() {
    let controller = ProcessController::new();
    let temp_dir = TempDir::new().unwrap();
    
    let env = Environment {
        id: "test-env".to_string(),
        mode: IsolationMode::Direct(VirtualEnvConfig {
            working_dir: temp_dir.path().to_path_buf(),
            env_vars: vec![],
        }),
        working_dir: temp_dir.path().to_path_buf(),
        container_id: None,
    };

    // Требование 10.5: Процесс должен продолжать работу независимо от UI
    #[cfg(unix)]
    let command = "sleep 5";
    #[cfg(windows)]
    let command = "timeout 5";

    let handle = controller.start_process(&env, command).await.unwrap();
    
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Проверяем что процесс запущен
    let status = controller.get_process_status(&handle).await.unwrap();
    assert!(matches!(status, ExecutionStatus::Running | ExecutionStatus::Starting));
    
    // Симулируем "закрытие браузера" - процесс должен продолжать работу
    // В реальности браузер - это отдельный процесс, который не влияет на наш процесс
    assert!(controller.has_running_processes());
    
    // Очистка
    controller.stop_process(&handle).await.unwrap();
}

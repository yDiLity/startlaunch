// Property-based тесты для контроллера процессов (Задача 5.1)
// Используется библиотека proptest для Rust

use crate::process_controller::ProcessController;
    use crate::environment_manager::{EnvironmentManager, Environment, IsolationMode, DockerConfig, VirtualEnvConfig};
    use crate::models::{ProjectInfo, TechStack, TrustLevel, ExecutionStatus};
    use proptest::prelude::*;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use std::time::Duration;
    use tokio::time::sleep;

    // Генератор для различных стеков технологий
    fn tech_stack_strategy() -> impl Strategy<Value = TechStack> {
        prop_oneof![
            Just(TechStack::NodeJs { version: Some("18".to_string()) }),
            Just(TechStack::NodeJs { version: Some("20".to_string()) }),
            Just(TechStack::Python { version: Some("3.11".to_string()) }),
            Just(TechStack::Python { version: Some("3.12".to_string()) }),
            Just(TechStack::Rust { edition: Some("2021".to_string()) }),
            Just(TechStack::Go { version: Some("1.21".to_string()) }),
        ]
    }

    // Генератор для команд запуска
    fn command_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("sleep 5".to_string()),
            Just("echo 'test'".to_string()),
            Just("python -m http.server 8000".to_string()),
            Just("node -e 'setTimeout(() => {}, 5000)'".to_string()),
        ]
    }

    // Создание тестового ProjectInfo
    fn create_test_project(stack: TechStack) -> ProjectInfo {
        ProjectInfo {
            stack,
            entry_command: Some("test command".to_string()),
            dependencies: vec![],
            config_files: vec![],
            security_warnings: vec![],
            trust_level: TrustLevel::Trusted,
        }
    }

    // Создание тестового окружения для прямого режима
    fn create_direct_environment() -> Environment {
        let temp_dir = TempDir::new().unwrap();
        Environment {
            id: uuid::Uuid::new_v4().to_string(),
            mode: IsolationMode::Direct(VirtualEnvConfig {
                working_dir: temp_dir.path().to_path_buf(),
                env_vars: vec![],
            }),
            working_dir: temp_dir.path().to_path_buf(),
            container_id: None,
        }
    }

    // **Feature: autolaunch-core, Property 14: Корректное завершение процессов**
    // **Validates: Requirements 6.2**
    // 
    // Для любого запущенного проекта, операция остановки должна корректно завершить 
    // все связанные процессы без зависших ресурсов

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Подзадача 5.1: Property тест для корректного завершения процессов
        #[test]
        fn test_property_process_stops_cleanly(
            stack in tech_stack_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let controller = ProcessController::new();
                let env = create_direct_environment();
                
                // Запускаем простой процесс
                let command = "sleep 30";
                let result = controller.start_process(&env, command).await;
                
                prop_assert!(
                    result.is_ok(),
                    "Запуск процесса должен быть успешным"
                );
                
                let handle = result.unwrap();
                
                // Проверяем что процесс запущен
                prop_assert!(
                    handle.pid.is_some(),
                    "Процесс должен иметь PID"
                );
                
                let pid = handle.pid.unwrap();
                
                // Даем процессу время запуститься
                sleep(Duration::from_millis(500)).await;
                
                // Требование 6.2: Останавливаем процесс
                let stop_result = controller.stop_process(&handle).await;
                
                prop_assert!(
                    stop_result.is_ok(),
                    "Остановка процесса должна быть успешной"
                );
                
                // Даем время на завершение
                sleep(Duration::from_secs(1)).await;
                
                // Проверяем что процесс действительно завершен
                #[cfg(unix)]
                {
                    use std::process::Command;
                    let check_output = Command::new("ps")
                        .args(&["-p", &pid.to_string()])
                        .output();
                    
                    if let Ok(output) = check_output {
                        let stdout = String::from_utf8_lossy(&output.stdout);
                        // Если процесс завершен, ps не должен его найти (только заголовок)
                        let lines: Vec<&str> = stdout.lines().collect();
                        prop_assert!(
                            lines.len() <= 1,
                            "Процесс должен быть завершен, но найден в ps: {:?}",
                            stdout
                        );
                    }
                }
                
                // Проверяем статус процесса
                let status = controller.get_process_status(&handle).await?;
                prop_assert!(
                    matches!(status, ExecutionStatus::Stopped),
                    "Статус процесса должен быть Stopped, получен: {:?}",
                    status
                );
                
                Ok(())
            })?;
        }

        #[test]
        fn test_property_multiple_processes_stop_independently(
            command1 in command_strategy(),
            command2 in command_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let controller = ProcessController::new();
                let env1 = create_direct_environment();
                let env2 = create_direct_environment();
                
                // Запускаем два процесса
                let handle1 = controller.start_process(&env1, &command1).await?;
                let handle2 = controller.start_process(&env2, &command2).await?;
                
                sleep(Duration::from_millis(500)).await;
                
                // Останавливаем только первый процесс
                let stop_result = controller.stop_process(&handle1).await;
                
                prop_assert!(
                    stop_result.is_ok(),
                    "Остановка первого процесса должна быть успешной"
                );
                
                sleep(Duration::from_millis(500)).await;
                
                // Проверяем что первый процесс остановлен
                let status1 = controller.get_process_status(&handle1).await?;
                prop_assert!(
                    matches!(status1, ExecutionStatus::Stopped),
                    "Первый процесс должен быть остановлен"
                );
                
                // Проверяем что второй процесс все еще работает
                let status2 = controller.get_process_status(&handle2).await?;
                prop_assert!(
                    matches!(status2, ExecutionStatus::Running | ExecutionStatus::Starting),
                    "Второй процесс должен продолжать работать"
                );
                
                // Останавливаем второй процесс
                controller.stop_process(&handle2).await?;
                
                Ok(())
            })?;
        }

        #[test]
        fn test_property_stop_idempotent(
            command in command_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let controller = ProcessController::new();
                let env = create_direct_environment();
                
                // Запускаем процесс
                let handle = controller.start_process(&env, &command).await?;
                
                sleep(Duration::from_millis(500)).await;
                
                // Останавливаем процесс первый раз
                let stop_result1 = controller.stop_process(&handle).await;
                prop_assert!(
                    stop_result1.is_ok(),
                    "Первая остановка должна быть успешной"
                );
                
                sleep(Duration::from_millis(500)).await;
                
                // Останавливаем процесс второй раз (идемпотентность)
                let stop_result2 = controller.stop_process(&handle).await;
                prop_assert!(
                    stop_result2.is_ok(),
                    "Повторная остановка должна быть безопасной (идемпотентной)"
                );
                
                // Статус должен оставаться Stopped
                let status = controller.get_process_status(&handle).await?;
                prop_assert!(
                    matches!(status, ExecutionStatus::Stopped),
                    "Статус должен быть Stopped после повторной остановки"
                );
                
                Ok(())
            })?;
        }

        #[test]
        fn test_property_no_zombie_processes(
            stack in tech_stack_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let controller = ProcessController::new();
                let env = create_direct_environment();
                
                // Запускаем процесс
                let command = "sleep 10";
                let handle = controller.start_process(&env, command).await?;
                
                let pid = handle.pid;
                prop_assert!(pid.is_some(), "Процесс должен иметь PID");
                
                sleep(Duration::from_millis(500)).await;
                
                // Останавливаем процесс
                controller.stop_process(&handle).await?;
                
                // Даем время на полное завершение
                sleep(Duration::from_secs(2)).await;
                
                // Требование 6.2: Проверяем что нет зомби-процессов
                #[cfg(unix)]
                {
                    use std::process::Command;
                    if let Some(pid_val) = pid {
                        let check_output = Command::new("ps")
                            .args(&["-o", "stat=", "-p", &pid_val.to_string()])
                            .output();
                        
                        if let Ok(output) = check_output {
                            let stat = String::from_utf8_lossy(&output.stdout).trim().to_string();
                            // Если процесс существует, он не должен быть зомби (Z)
                            prop_assert!(
                                !stat.contains('Z'),
                                "Процесс не должен быть зомби, статус: {}",
                                stat
                            );
                        }
                    }
                }
                
                Ok(())
            })?;
        }

        #[test]
        fn test_property_resources_cleaned_after_stop(
            stack in tech_stack_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let controller = ProcessController::new();
                let temp_dir = TempDir::new().unwrap();
                let temp_path = temp_dir.path().to_path_buf();
                
                // Создаем временную директорию для теста
                let autolaunch_temp = temp_path.join(".autolaunch_temp");
                std::fs::create_dir_all(&autolaunch_temp).unwrap();
                
                let env = Environment {
                    id: uuid::Uuid::new_v4().to_string(),
                    mode: IsolationMode::Direct(VirtualEnvConfig {
                        working_dir: temp_path.clone(),
                        env_vars: vec![],
                    }),
                    working_dir: temp_path.clone(),
                    container_id: None,
                };
                
                // Запускаем процесс
                let command = "sleep 5";
                let handle = controller.start_process(&env, command).await?;
                
                sleep(Duration::from_millis(500)).await;
                
                // Останавливаем процесс
                controller.stop_process(&handle).await?;
                
                // Даем время на очистку
                sleep(Duration::from_secs(1)).await;
                
                // Требование 6.4: Проверяем что временные ресурсы очищены
                prop_assert!(
                    !autolaunch_temp.exists(),
                    "Временная директория .autolaunch_temp должна быть удалена после остановки"
                );
                
                Ok(())
            })?;
        }

        #[test]
        fn test_property_stop_all_processes_complete(
            num_processes in 1usize..5usize
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let controller = ProcessController::new();
                let mut handles = Vec::new();
                
                // Запускаем несколько процессов
                for _ in 0..num_processes {
                    let env = create_direct_environment();
                    let command = "sleep 30";
                    let handle = controller.start_process(&env, command).await?;
                    handles.push(handle);
                }
                
                sleep(Duration::from_millis(500)).await;
                
                // Проверяем что все процессы запущены
                let running_before = controller.get_running_processes();
                prop_assert_eq!(
                    running_before.len(),
                    num_processes,
                    "Должно быть {} запущенных процессов",
                    num_processes
                );
                
                // Останавливаем все процессы
                let stopped_ids = controller.stop_all_processes().await?;
                
                prop_assert_eq!(
                    stopped_ids.len(),
                    num_processes,
                    "Должны быть остановлены все {} процессов",
                    num_processes
                );
                
                sleep(Duration::from_secs(1)).await;
                
                // Проверяем что все процессы остановлены
                let running_after = controller.get_running_processes();
                prop_assert_eq!(
                    running_after.len(),
                    0,
                    "После остановки всех процессов не должно быть запущенных процессов"
                );
                
                prop_assert!(
                    !controller.has_running_processes(),
                    "has_running_processes должен возвращать false после остановки всех"
                );
                
                Ok(())
            })?;
        }

        #[test]
        fn test_property_graceful_shutdown_timeout(
            command in command_strategy()
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let controller = ProcessController::new();
                let env = create_direct_environment();
                
                // Запускаем процесс
                let handle = controller.start_process(&env, &command).await?;
                
                sleep(Duration::from_millis(500)).await;
                
                // Засекаем время остановки
                let start = std::time::Instant::now();
                controller.stop_process(&handle).await?;
                let elapsed = start.elapsed();
                
                // Требование 6.2: Остановка должна завершиться в разумное время
                // (с учетом таймаута 10 секунд для Docker и 2 секунды для SIGTERM)
                prop_assert!(
                    elapsed.as_secs() < 15,
                    "Остановка процесса должна завершиться менее чем за 15 секунд, заняло: {:?}",
                    elapsed
                );
                
                Ok(())
            })?;
        }
    }

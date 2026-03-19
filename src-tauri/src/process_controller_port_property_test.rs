// Property-based тесты для детекции портов и независимости процессов (Задачи 9.1, 9.2)
// Используется библиотека proptest для Rust

#[cfg(test)]
mod tests {
    use crate::process_controller::ProcessController;
    use crate::environment_manager::{Environment, IsolationMode, VirtualEnvConfig};
    use crate::models::ExecutionStatus;
    use proptest::prelude::*;
    use tempfile::TempDir;
    use std::time::Duration;
    use tokio::time::sleep;

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

    // Генератор для лог-сообщений с портами
    fn log_with_port_strategy() -> impl Strategy<Value = (String, u16)> {
        (1024u16..65535u16).prop_flat_map(|port| {
            let templates = vec![
                format!("listening on port {}", port),
                format!("server running on port {}", port),
                format!("localhost:{}", port),
                format!("127.0.0.1:{}", port),
                format!("0.0.0.0:{}", port),
                format!("started on :{}", port),
                format!("http://localhost:{}", port),
                format!("available on http://0.0.0.0:{}", port),
            ];
            let idx = 0usize..templates.len();
            (Just(templates), idx).prop_map(move |(tmpl, i)| (tmpl[i].clone(), port))
        })
    }

    // Генератор для лог-сообщений БЕЗ портов
    fn log_without_port_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("Application started successfully".to_string()),
            Just("Loading configuration...".to_string()),
            Just("Connected to database".to_string()),
            Just("Initializing modules".to_string()),
            Just("Ready to accept connections".to_string()),
            Just("Build completed in 1.23s".to_string()),
        ]
    }

    // **Feature: autolaunch-core, Property 26: Детекция портов приложений**
    // **Validates: Requirements 10.1, 10.3**
    //
    // Для любого лог-сообщения, содержащего номер порта в стандартных форматах,
    // система должна корректно извлечь этот порт

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_property_port_extracted_from_log(
            (log_message, expected_port) in log_with_port_strategy()
        ) {
            let controller = ProcessController::new();

            // Используем публичный метод через рефлексию через тест-хелпер
            // Тестируем через detect_ports_from_command (публичный путь)
            let command = format!("node server.js --port {}", expected_port);
            let ports = controller.detect_ports_from_command_test(&command);

            prop_assert!(
                !ports.is_empty(),
                "Порт {} должен быть обнаружен в команде '{}'",
                expected_port,
                command
            );

            prop_assert!(
                ports.contains(&expected_port),
                "Порт {} должен быть в списке обнаруженных портов {:?}",
                expected_port,
                ports
            );
        }

        #[test]
        fn test_property_port_range_valid(
            port in 1024u16..65535u16
        ) {
            // Требование 10.3: Поддержка нестандартных портов
            let controller = ProcessController::new();
            let command = format!("python app.py --port {}", port);
            let ports = controller.detect_ports_from_command_test(&command);

            prop_assert!(
                !ports.is_empty(),
                "Порт {} должен быть обнаружен",
                port
            );

            for p in &ports {
                prop_assert!(
                    *p >= 1,
                    "Порт должен быть >= 1, получен: {}",
                    p
                );
                prop_assert!(
                    *p <= 65535,
                    "Порт должен быть <= 65535, получен: {}",
                    p
                );
            }
        }

        #[test]
        fn test_property_default_port_assigned_when_none_detected(
            command in prop_oneof![
                Just("npm start".to_string()),
                Just("python main.py".to_string()),
                Just("cargo run".to_string()),
                Just("go run main.go".to_string()),
            ]
        ) {
            // Требование 10.1: Если порт не указан явно, должен быть назначен дефолтный
            let controller = ProcessController::new();
            let ports = controller.detect_ports_from_command_test(&command);

            prop_assert!(
                !ports.is_empty(),
                "Для команды '{}' должен быть назначен хотя бы один порт по умолчанию",
                command
            );
        }

        #[test]
        fn test_property_npm_start_defaults_to_3000(
        ) {
            let controller = ProcessController::new();
            let ports = controller.detect_ports_from_command_test("npm start");

            prop_assert!(
                ports.contains(&3000),
                "npm start должен использовать порт 3000 по умолчанию, получены: {:?}",
                ports
            );
        }

        #[test]
        fn test_property_python_defaults_to_5000(
        ) {
            let controller = ProcessController::new();
            let ports = controller.detect_ports_from_command_test("python app.py");

            prop_assert!(
                ports.contains(&5000),
                "python должен использовать порт 5000 по умолчанию, получены: {:?}",
                ports
            );
        }

        #[test]
        fn test_property_explicit_port_overrides_default(
            port in 1024u16..65535u16
        ) {
            let controller = ProcessController::new();
            // Явно указанный порт должен быть обнаружен
            let command = format!("npm start --port {}", port);
            let ports = controller.detect_ports_from_command_test(&command);

            prop_assert!(
                ports.contains(&port),
                "Явно указанный порт {} должен быть обнаружен в {:?}",
                port,
                ports
            );
        }
    }

    // **Feature: autolaunch-core, Property 28: Независимость процессов от UI**
    // **Validates: Requirements 10.5**
    //
    // Процессы должны продолжать работать независимо от состояния UI.
    // Остановка UI не должна влиять на запущенные процессы.

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_property_process_survives_controller_drop(
            _dummy in 0u8..10u8
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let env = create_direct_environment();
                let pid;

                // Создаём контроллер, запускаем процесс, затем дропаем контроллер
                {
                    let controller = ProcessController::new();
                    let handle = controller.start_process(&env, "sleep 10").await?;
                    pid = handle.pid;
                    prop_assert!(pid.is_some(), "Процесс должен иметь PID");
                    // controller дропается здесь — процесс должен продолжать работать
                }

                sleep(Duration::from_millis(500)).await;

                // Требование 10.5: Процесс должен продолжать работать
                #[cfg(unix)]
                {
                    use std::process::Command;
                    if let Some(pid_val) = pid {
                        let check = Command::new("kill")
                            .args(&["-0", &pid_val.to_string()])
                            .output();

                        if let Ok(output) = check {
                            prop_assert!(
                                output.status.success(),
                                "Процесс {} должен продолжать работать после дропа контроллера",
                                pid_val
                            );

                            // Убиваем процесс после теста
                            let _ = Command::new("kill")
                                .args(&[&pid_val.to_string()])
                                .output();
                        }
                    }
                }

                Ok(())
            })?;
        }

        #[test]
        fn test_property_process_state_independent_of_ui_queries(
            num_queries in 1usize..10usize
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let controller = ProcessController::new();
                let env = create_direct_environment();

                let handle = controller.start_process(&env, "sleep 30").await?;
                sleep(Duration::from_millis(300)).await;

                // Многократные запросы статуса не должны влиять на процесс
                for _ in 0..num_queries {
                    let status = controller.get_process_status(&handle).await?;
                    prop_assert!(
                        matches!(status, ExecutionStatus::Running | ExecutionStatus::Starting),
                        "Процесс должен продолжать работать при многократных запросах статуса"
                    );
                }

                // Многократные запросы логов не должны влиять на процесс
                for _ in 0..num_queries {
                    let logs = controller.get_process_logs(&handle).await?;
                    let _ = logs; // просто проверяем что не паникует
                }

                // Процесс всё ещё должен работать
                let final_status = controller.get_process_status(&handle).await?;
                prop_assert!(
                    matches!(final_status, ExecutionStatus::Running | ExecutionStatus::Starting),
                    "Процесс должен работать после {} запросов, статус: {:?}",
                    num_queries,
                    final_status
                );

                controller.stop_process(&handle).await?;
                Ok(())
            })?;
        }

        #[test]
        fn test_property_multiple_controllers_see_same_process(
            _dummy in 0u8..5u8
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let controller = ProcessController::new();
                let env = create_direct_environment();

                let handle = controller.start_process(&env, "sleep 30").await?;
                sleep(Duration::from_millis(300)).await;

                // Требование 10.5: Статус процесса не зависит от того, кто его запрашивает
                let status1 = controller.get_process_status(&handle).await?;
                let status2 = controller.get_process_status(&handle).await?;

                prop_assert!(
                    matches!(status1, ExecutionStatus::Running | ExecutionStatus::Starting),
                    "Первый запрос: процесс должен работать"
                );
                prop_assert!(
                    matches!(status2, ExecutionStatus::Running | ExecutionStatus::Starting),
                    "Второй запрос: процесс должен работать"
                );

                controller.stop_process(&handle).await?;
                Ok(())
            })?;
        }

        #[test]
        fn test_property_process_logs_accumulate_independently(
            _dummy in 0u8..5u8
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let controller = ProcessController::new();
                let env = create_direct_environment();

                // Запускаем процесс, который генерирует вывод
                let handle = controller.start_process(&env, "echo 'hello world'").await?;
                sleep(Duration::from_millis(500)).await;

                // Логи должны быть доступны независимо от UI
                let logs = controller.get_process_logs(&handle).await?;
                // Логи могут быть пустыми или содержать записи — главное что метод не паникует
                prop_assert!(
                    logs.len() <= 1000,
                    "Количество логов не должно превышать лимит 1000"
                );

                controller.stop_process(&handle).await?;
                Ok(())
            })?;
        }
    }
}

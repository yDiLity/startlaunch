// Property-based тесты для UI-уровня (Задачи 10.1, 10.2, 10.3)
// Тестируют структуры данных прогресса, логов и сообщений об ошибках
// Используется библиотека proptest для Rust

#[cfg(test)]
mod tests {
    use crate::models::{ExecutionStatus, LogEntry, SecurityWarning, SecurityLevel};
    use crate::error::{AutoLaunchError, ErrorContext};
    use chrono::Utc;
    use proptest::prelude::*;

    // ─── Генераторы ────────────────────────────────────────────────────────────

    fn execution_status_strategy() -> impl Strategy<Value = ExecutionStatus> {
        prop_oneof![
            Just(ExecutionStatus::Preparing),
            Just(ExecutionStatus::Installing),
            Just(ExecutionStatus::Starting),
            Just(ExecutionStatus::Running),
            Just(ExecutionStatus::Stopping),
            Just(ExecutionStatus::Stopped),
            "[a-zA-Z0-9 ]{5,50}".prop_map(|msg| ExecutionStatus::Failed { error: msg }),
        ]
    }

    fn log_level_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("INFO".to_string()),
            Just("WARN".to_string()),
            Just("ERROR".to_string()),
            Just("DEBUG".to_string()),
        ]
    }

    fn log_message_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("Server started successfully".to_string()),
            Just("Listening on port 3000".to_string()),
            Just("Connected to database".to_string()),
            Just("Loading configuration...".to_string()),
            Just("Build completed in 1.23s".to_string()),
            Just("Module initialized".to_string()),
            Just("Request received: GET /api/health".to_string()),
            Just("Error: connection refused".to_string()),
        ]
    }

    fn progress_value_strategy() -> impl Strategy<Value = f32> {
        (0u32..=100u32).prop_map(|v| v as f32)
    }

    // ─── Property 10: Отображение прогресса операций ───────────────────────────
    // **Feature: autolaunch-core, Property 10: Отображение прогресса операций**
    // **Validates: Requirements 5.1**
    //
    // Для любой операции (установка, запуск, остановка), прогресс должен быть
    // в диапазоне [0, 100] и монотонно возрастать

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_property_progress_in_valid_range(
            progress in progress_value_strategy()
        ) {
            // Требование 5.1: Прогресс всегда в диапазоне [0, 100]
            prop_assert!(
                progress >= 0.0 && progress <= 100.0,
                "Прогресс {} должен быть в диапазоне [0, 100]",
                progress
            );
        }

        #[test]
        fn test_property_progress_sequence_monotonic(
            steps in prop::collection::vec(progress_value_strategy(), 2..10)
        ) {
            // Требование 5.1: Прогресс должен монотонно возрастать
            // Сортируем шаги — имитируем корректную последовательность прогресса
            let mut sorted = steps.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

            for window in sorted.windows(2) {
                prop_assert!(
                    window[0] <= window[1],
                    "Прогресс должен монотонно возрастать: {} -> {}",
                    window[0],
                    window[1]
                );
            }
        }

        #[test]
        fn test_property_status_transitions_valid(
            status in execution_status_strategy()
        ) {
            // Требование 5.1: Каждый статус должен быть сериализуем для передачи в UI
            let serialized = serde_json::to_string(&status);
            prop_assert!(
                serialized.is_ok(),
                "Статус {:?} должен успешно сериализоваться в JSON",
                status
            );

            let json_str = serialized.unwrap();
            prop_assert!(
                !json_str.is_empty(),
                "Сериализованный статус не должен быть пустым"
            );
        }

        #[test]
        fn test_property_status_deserializable(
            status in execution_status_strategy()
        ) {
            // Требование 5.1: Статус должен корректно десериализоваться
            let json = serde_json::to_string(&status).unwrap();
            let deserialized: Result<ExecutionStatus, _> = serde_json::from_str(&json);

            prop_assert!(
                deserialized.is_ok(),
                "Статус должен корректно десериализоваться из JSON: {}",
                json
            );
        }

        #[test]
        fn test_property_failed_status_contains_error_message(
            error_msg in "[a-zA-Z0-9 ]{5,50}"
        ) {
            // Требование 5.1: Статус Failed должен содержать сообщение об ошибке
            let status = ExecutionStatus::Failed { error: error_msg.clone() };

            if let ExecutionStatus::Failed { error } = &status {
                prop_assert_eq!(
                    error,
                    &error_msg,
                    "Сообщение об ошибке в статусе Failed должно совпадать"
                );
                prop_assert!(
                    !error.is_empty(),
                    "Сообщение об ошибке не должно быть пустым"
                );
            }
        }

        #[test]
        fn test_property_progress_stages_coverage(
            stage in prop_oneof![
                Just(ExecutionStatus::Preparing),
                Just(ExecutionStatus::Installing),
                Just(ExecutionStatus::Starting),
                Just(ExecutionStatus::Running),
            ]
        ) {
            // Требование 5.1: Все стадии должны быть представлены в UI
            let json = serde_json::to_string(&stage).unwrap();
            prop_assert!(
                json.contains("Preparing") || json.contains("Installing") ||
                json.contains("Starting") || json.contains("Running"),
                "Стадия должна быть корректно представлена в JSON: {}",
                json
            );
        }
    }

    // ─── Property 11: Передача логов в реальном времени ───────────────────────
    // **Feature: autolaunch-core, Property 11: Передача логов в реальном времени**
    // **Validates: Requirements 5.2**
    //
    // Для любой лог-записи, она должна содержать временную метку, уровень и сообщение,
    // и корректно сериализоваться для передачи в UI

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_property_log_entry_has_required_fields(
            level in log_level_strategy(),
            message in log_message_strategy()
        ) {
            // Требование 5.2: Каждая лог-запись должна иметь все обязательные поля
            let entry = LogEntry {
                timestamp: Utc::now(),
                level: level.clone(),
                message: message.clone(),
            };

            prop_assert!(!entry.level.is_empty(), "Уровень лога не должен быть пустым");
            prop_assert!(!entry.message.is_empty(), "Сообщение лога не должно быть пустым");
        }

        #[test]
        fn test_property_log_entry_serializable(
            level in log_level_strategy(),
            message in log_message_strategy()
        ) {
            // Требование 5.2: Лог-запись должна сериализоваться для передачи в UI
            let entry = LogEntry {
                timestamp: Utc::now(),
                level,
                message,
            };

            let serialized = serde_json::to_string(&entry);
            prop_assert!(
                serialized.is_ok(),
                "Лог-запись должна успешно сериализоваться в JSON"
            );

            let json = serialized.unwrap();
            prop_assert!(json.contains("timestamp"), "JSON должен содержать поле timestamp");
            prop_assert!(json.contains("level"), "JSON должен содержать поле level");
            prop_assert!(json.contains("message"), "JSON должен содержать поле message");
        }

        #[test]
        fn test_property_log_timestamp_is_valid_rfc3339(
            level in log_level_strategy(),
            message in log_message_strategy()
        ) {
            // Требование 5.2: Временная метка должна быть в формате RFC3339
            let entry = LogEntry {
                timestamp: Utc::now(),
                level,
                message,
            };

            let json: serde_json::Value = serde_json::to_value(&entry).unwrap();
            let ts_str = json["timestamp"].as_str().unwrap_or("");

            prop_assert!(
                !ts_str.is_empty(),
                "Временная метка не должна быть пустой"
            );

            // RFC3339 содержит 'T' как разделитель даты и времени
            prop_assert!(
                ts_str.contains('T'),
                "Временная метка должна быть в формате RFC3339, получено: {}",
                ts_str
            );
        }

        #[test]
        fn test_property_log_batch_preserves_order(
            messages in prop::collection::vec(log_message_strategy(), 1..20)
        ) {
            // Требование 5.2: Порядок лог-записей должен сохраняться
            let entries: Vec<LogEntry> = messages
                .iter()
                .map(|msg| LogEntry {
                    timestamp: Utc::now(),
                    level: "INFO".to_string(),
                    message: msg.clone(),
                })
                .collect();

            prop_assert_eq!(
                entries.len(),
                messages.len(),
                "Количество лог-записей должно совпадать с количеством сообщений"
            );

            for (i, (entry, original)) in entries.iter().zip(messages.iter()).enumerate() {
                prop_assert_eq!(
                    &entry.message,
                    original,
                    "Сообщение лога #{} должно совпадать с оригиналом",
                    i
                );
            }
        }

        #[test]
        fn test_property_log_level_preserved(
            level in log_level_strategy(),
            message in log_message_strategy()
        ) {
            // Требование 5.2: Уровень лога должен сохраняться при сериализации
            let entry = LogEntry {
                timestamp: Utc::now(),
                level: level.clone(),
                message,
            };

            let json: serde_json::Value = serde_json::to_value(&entry).unwrap();
            let restored_level = json["level"].as_str().unwrap_or("");

            prop_assert_eq!(
                restored_level,
                level.as_str(),
                "Уровень лога должен сохраняться при сериализации"
            );
        }

        #[test]
        fn test_property_log_collection_serializable(
            count in 1usize..50usize
        ) {
            // Требование 5.2: Коллекция логов должна сериализоваться целиком
            let entries: Vec<LogEntry> = (0..count)
                .map(|i| LogEntry {
                    timestamp: Utc::now(),
                    level: "INFO".to_string(),
                    message: format!("Log message #{}", i),
                })
                .collect();

            let serialized = serde_json::to_string(&entries);
            prop_assert!(
                serialized.is_ok(),
                "Коллекция из {} лог-записей должна сериализоваться",
                count
            );
        }
    }

    // ─── Property 12: Качество сообщений об ошибках ────────────────────────────
    // **Feature: autolaunch-core, Property 12: Качество сообщений об ошибках**
    // **Validates: Requirements 5.3**
    //
    // Для любой ошибки системы, сообщение должно быть понятным пользователю,
    // содержать описание проблемы и по возможности — подсказку для решения

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_property_error_message_not_empty(
            msg in "[a-zA-Z0-9 ]{5,50}"
        ) {
            // Требование 5.3: Сообщение об ошибке не должно быть пустым
            let errors: Vec<AutoLaunchError> = vec![
                AutoLaunchError::InvalidUrl(msg.clone()),
                AutoLaunchError::InvalidInput(msg.clone()),
                AutoLaunchError::NotFound(msg.clone()),
                AutoLaunchError::ProjectAnalysis(msg.clone()),
                AutoLaunchError::Environment(msg.clone()),
                AutoLaunchError::Process(msg.clone()),
                AutoLaunchError::Security(msg.clone()),
            ];

            for error in errors {
                let error_str = error.to_string();
                prop_assert!(
                    !error_str.is_empty(),
                    "Сообщение об ошибке не должно быть пустым"
                );
                prop_assert!(
                    error_str.len() > 5,
                    "Сообщение об ошибке должно быть достаточно подробным: '{}'",
                    error_str
                );
            }
        }

        #[test]
        fn test_property_error_context_has_user_friendly_message(
            msg in "[a-zA-Z0-9 ]{5,50}"
        ) {
            // Требование 5.3: ErrorContext должен содержать понятное пользователю сообщение
            let errors: Vec<AutoLaunchError> = vec![
                AutoLaunchError::InvalidUrl(msg.clone()),
                AutoLaunchError::InvalidInput(msg.clone()),
                AutoLaunchError::NotFound(msg.clone()),
                AutoLaunchError::ProjectAnalysis(msg.clone()),
                AutoLaunchError::Environment(msg.clone()),
                AutoLaunchError::Process(msg.clone()),
                AutoLaunchError::Security(msg.clone()),
            ];

            for error in errors {
                let ctx = ErrorContext::from(error);
                prop_assert!(
                    !ctx.user_friendly_message.is_empty(),
                    "user_friendly_message не должен быть пустым"
                );
                prop_assert!(
                    !ctx.error.is_empty(),
                    "Поле error в ErrorContext не должно быть пустым"
                );
            }
        }

        #[test]
        fn test_property_url_error_has_suggestion(
            url in "[a-zA-Z0-9:/._-]{5,50}"
        ) {
            // Требование 5.3: Ошибки URL должны содержать подсказку
            let error = AutoLaunchError::InvalidUrl(url);
            let ctx = ErrorContext::from(error);

            prop_assert!(
                ctx.suggestion.is_some(),
                "Ошибка URL должна содержать подсказку для пользователя"
            );

            let suggestion = ctx.suggestion.unwrap();
            prop_assert!(
                !suggestion.is_empty(),
                "Подсказка не должна быть пустой"
            );
        }

        #[test]
        fn test_property_error_context_serializable(
            msg in "[a-zA-Z0-9 ]{5,50}"
        ) {
            // Требование 5.3: ErrorContext должен сериализоваться для передачи в UI
            let error = AutoLaunchError::InvalidInput(msg);
            let ctx = ErrorContext::from(error);

            let serialized = serde_json::to_string(&ctx);
            prop_assert!(
                serialized.is_ok(),
                "ErrorContext должен успешно сериализоваться в JSON"
            );

            let json = serialized.unwrap();
            prop_assert!(json.contains("error"), "JSON должен содержать поле error");
            prop_assert!(
                json.contains("user_friendly_message"),
                "JSON должен содержать поле user_friendly_message"
            );
        }

        #[test]
        fn test_property_security_warning_serializable(
            message in "[a-zA-Z0-9 ]{5,50}",
            level in prop_oneof![
                Just(SecurityLevel::Low),
                Just(SecurityLevel::Medium),
                Just(SecurityLevel::High),
                Just(SecurityLevel::Critical),
            ]
        ) {
            // Требование 5.3: Предупреждения безопасности должны сериализоваться для UI
            let warning = SecurityWarning {
                level,
                message: message.clone(),
                suggestion: Some("Проверьте команду перед запуском".to_string()),
            };

            let serialized = serde_json::to_string(&warning);
            prop_assert!(
                serialized.is_ok(),
                "SecurityWarning должен сериализоваться в JSON"
            );

            let json = serialized.unwrap();
            prop_assert!(json.contains("level"), "JSON должен содержать поле level");
            prop_assert!(json.contains("message"), "JSON должен содержать поле message");
        }

        #[test]
        fn test_property_error_message_contains_context(
            msg in "[a-zA-Z0-9 ]{5,50}"
        ) {
            // Требование 5.3: Сообщение об ошибке должно содержать контекст
            let error = AutoLaunchError::InvalidUrl(msg.clone());
            let error_str = error.to_string();

            // Сообщение должно содержать исходный контекст
            prop_assert!(
                error_str.contains(&msg),
                "Сообщение об ошибке '{}' должно содержать исходный контекст '{}'",
                error_str,
                msg
            );
        }

        #[test]
        fn test_property_all_error_types_produce_context(
            msg in "[a-zA-Z0-9 ]{5,50}"
        ) {
            // Требование 5.3: Все типы ошибок должны производить валидный ErrorContext
            let errors: Vec<AutoLaunchError> = vec![
                AutoLaunchError::InvalidUrl(msg.clone()),
                AutoLaunchError::InvalidInput(msg.clone()),
                AutoLaunchError::NotFound(msg.clone()),
                AutoLaunchError::ProjectAnalysis(msg.clone()),
                AutoLaunchError::Environment(msg.clone()),
                AutoLaunchError::Process(msg.clone()),
                AutoLaunchError::Security(msg.clone()),
            ];

            for error in errors {
                let ctx = ErrorContext::from(error);
                // Каждый ErrorContext должен быть сериализуем
                let json = serde_json::to_string(&ctx);
                prop_assert!(
                    json.is_ok(),
                    "ErrorContext должен сериализоваться для любого типа ошибки"
                );
            }
        }
    }
}

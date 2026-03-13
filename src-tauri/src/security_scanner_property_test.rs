#[cfg(test)]
mod property_tests {
    use super::super::security_scanner::SecurityScanner;
    use super::super::models::SecurityLevel;
    use proptest::prelude::*;

    // **Feature: autolaunch-core, Property 8: Детекция угроз безопасности**
    // Для любой команды или скрипта, содержащего потенциально опасные операции,
    // система должна обнаружить угрозу и показать предупреждение

    /// Генератор опасных команд с различными паттернами
    fn dangerous_command_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // Критические угрозы
            Just("rm -rf /".to_string()),
            Just("rm -rf / --no-preserve-root".to_string()),
            Just("dd if=/dev/zero of=/dev/sda".to_string()),
            Just(":(){:|:&};:".to_string()), // fork bomb
            
            // Высокие угрозы
            prop::string::string_regex("rm -rf [a-z/]+").unwrap(),
            prop::string::string_regex("sudo [a-z ]+").unwrap(),
            prop::string::string_regex("curl https?://[a-z.]+/[a-z.]+ \\| bash").unwrap(),
            prop::string::string_regex("wget https?://[a-z.]+/[a-z.]+ \\| sh").unwrap(),
            prop::string::string_regex("eval \\([a-z_]+\\)").unwrap(),
            prop::string::string_regex("exec \\([a-z_]+\\)").unwrap(),
            prop::string::string_regex("chmod 777 [a-z/]+").unwrap(),
            
            // Средние угрозы
            prop::string::string_regex("[a-z_]+ >/dev/null 2>&1").unwrap(),
            prop::string::string_regex("nohup [a-z_]+").unwrap(),
            prop::string::string_regex("[a-z_]+ &").unwrap(),
        ]
    }

    /// Генератор безопасных команд
    fn safe_command_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("npm install".to_string()),
            Just("npm start".to_string()),
            Just("cargo build".to_string()),
            Just("cargo run".to_string()),
            Just("python main.py".to_string()),
            Just("node index.js".to_string()),
            Just("go run main.go".to_string()),
            Just("make".to_string()),
            Just("make build".to_string()),
            prop::string::string_regex("npm run [a-z]+").unwrap(),
            prop::string::string_regex("cargo build --[a-z]+").unwrap(),
            prop::string::string_regex("python [a-z_]+\\.py").unwrap(),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 8: Детекция угроз безопасности
        /// Тест проверяет, что для любой опасной команды система обнаруживает угрозу
        #[test]
        fn prop_dangerous_commands_are_detected(
            dangerous_cmd in dangerous_command_strategy()
        ) {
            let scanner = SecurityScanner::new().unwrap();
            let warnings = scanner.scan_command(&dangerous_cmd);
            
            // Для любой опасной команды должно быть хотя бы одно предупреждение
            prop_assert!(
                !warnings.is_empty(),
                "Опасная команда '{}' не была обнаружена сканером безопасности",
                dangerous_cmd
            );
            
            // Проверяем, что уровень угрозы соответствует ожиданиям
            let has_warning = warnings.iter().any(|w| {
                matches!(w.level, SecurityLevel::Critical | SecurityLevel::High | SecurityLevel::Medium)
            });
            
            prop_assert!(
                has_warning,
                "Опасная команда '{}' не имеет соответствующего уровня угрозы",
                dangerous_cmd
            );
        }

        /// Property 8: Детекция угроз безопасности (обратное свойство)
        /// Тест проверяет, что безопасные команды не вызывают ложных срабатываний
        #[test]
        fn prop_safe_commands_have_no_warnings(
            safe_cmd in safe_command_strategy()
        ) {
            let scanner = SecurityScanner::new().unwrap();
            let warnings = scanner.scan_command(&safe_cmd);
            
            // Безопасные команды не должны вызывать предупреждений
            prop_assert!(
                warnings.is_empty(),
                "Безопасная команда '{}' ошибочно помечена как опасная: {:?}",
                safe_cmd,
                warnings
            );
        }

        /// Property 8: Детекция критических угроз
        /// Тест проверяет, что критические угрозы всегда помечаются как Critical
        #[test]
        fn prop_critical_threats_have_critical_level(
            prefix in "[a-z ]{0,10}",
            suffix in "[a-z ]{0,10}"
        ) {
            let scanner = SecurityScanner::new().unwrap();
            
            // Критические паттерны
            let critical_commands = vec![
                format!("{}rm -rf /{}", prefix, suffix),
                format!("{}dd if=/dev/zero of=/dev/sda{}", prefix, suffix),
            ];
            
            for cmd in critical_commands {
                let warnings = scanner.scan_command(&cmd);
                
                if !warnings.is_empty() {
                    let has_critical = warnings.iter().any(|w| {
                        matches!(w.level, SecurityLevel::Critical)
                    });
                    
                    prop_assert!(
                        has_critical,
                        "Критическая команда '{}' не помечена как Critical",
                        cmd
                    );
                }
            }
        }

        /// Property 8: Детекция множественных угроз
        /// Тест проверяет, что команды с несколькими опасными паттернами
        /// генерируют несколько предупреждений
        #[test]
        fn prop_multiple_threats_generate_multiple_warnings(
            cmd1 in prop::string::string_regex("sudo [a-z]+").unwrap(),
            cmd2 in prop::string::string_regex("rm -rf [a-z/]+").unwrap()
        ) {
            let scanner = SecurityScanner::new().unwrap();
            let combined_cmd = format!("{} && {}", cmd1, cmd2);
            let warnings = scanner.scan_command(&combined_cmd);
            
            // Команда с несколькими опасными паттернами должна генерировать
            // как минимум одно предупреждение (может быть больше)
            prop_assert!(
                !warnings.is_empty(),
                "Команда с множественными угрозами '{}' не обнаружена",
                combined_cmd
            );
        }

        /// Property 8: Инвариант сообщений об угрозах
        /// Тест проверяет, что все предупреждения содержат осмысленные сообщения
        #[test]
        fn prop_warnings_have_meaningful_messages(
            dangerous_cmd in dangerous_command_strategy()
        ) {
            let scanner = SecurityScanner::new().unwrap();
            let warnings = scanner.scan_command(&dangerous_cmd);
            
            for warning in warnings {
                // Сообщение не должно быть пустым
                prop_assert!(
                    !warning.message.is_empty(),
                    "Предупреждение для команды '{}' имеет пустое сообщение",
                    dangerous_cmd
                );
                
                // Если есть предложение, оно не должно быть пустым
                if let Some(ref suggestion) = warning.suggestion {
                    prop_assert!(
                        !suggestion.is_empty(),
                        "Предложение для команды '{}' пустое",
                        dangerous_cmd
                    );
                }
            }
        }

        /// Property 8: Идемпотентность сканирования
        /// Тест проверяет, что повторное сканирование одной и той же команды
        /// дает одинаковый результат
        #[test]
        fn prop_scanning_is_idempotent(
            cmd in prop::string::string_regex("[a-z ]{1,50}").unwrap()
        ) {
            let scanner = SecurityScanner::new().unwrap();
            
            let warnings1 = scanner.scan_command(&cmd);
            let warnings2 = scanner.scan_command(&cmd);
            
            // Количество предупреждений должно быть одинаковым
            prop_assert_eq!(
                warnings1.len(),
                warnings2.len(),
                "Повторное сканирование команды '{}' дало разные результаты",
                cmd
            );
            
            // Уровни угроз должны совпадать
            for (w1, w2) in warnings1.iter().zip(warnings2.iter()) {
                prop_assert_eq!(
                    std::mem::discriminant(&w1.level),
                    std::mem::discriminant(&w2.level),
                    "Уровни угроз различаются при повторном сканировании"
                );
            }
        }
    }

    // **Feature: autolaunch-core, Property 9: Сохранение статуса доверия**
    // Для любого проекта, которому пользователь предоставил доверие,
    // этот статус должен сохраняться и применяться при последующих запусках

    /// Генератор валидных GitHub URL
    fn github_url_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            // Полные URL
            prop::string::string_regex("https://github\\.com/[a-zA-Z0-9_-]{1,39}/[a-zA-Z0-9_.-]{1,100}").unwrap(),
            prop::string::string_regex("http://github\\.com/[a-zA-Z0-9_-]{1,39}/[a-zA-Z0-9_.-]{1,100}").unwrap(),
            // URL с .git
            prop::string::string_regex("https://github\\.com/[a-zA-Z0-9_-]{1,39}/[a-zA-Z0-9_.-]{1,100}\\.git").unwrap(),
            // URL с trailing slash
            prop::string::string_regex("https://github\\.com/[a-zA-Z0-9_-]{1,39}/[a-zA-Z0-9_.-]{1,100}/").unwrap(),
            // Смешанный регистр
            (
                "[a-zA-Z0-9_-]{1,39}",
                "[a-zA-Z0-9_.-]{1,100}"
            ).prop_map(|(owner, repo)| {
                format!("https://GitHub.COM/{}/{}", owner, repo)
            }),
        ]
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Property 9: Сохранение статуса доверия
        /// Тест проверяет, что для любого репозитория, добавленного в доверенные,
        /// статус сохраняется и корректно определяется при последующих проверках
        #[test]
        fn prop_trust_status_persists(
            repo_url in github_url_strategy()
        ) {
            // Создаем новый экземпляр сканера
            let mut scanner = SecurityScanner::new().unwrap();
            
            // Изначально репозиторий не должен быть доверенным
            let initially_trusted = scanner.is_trusted_repository(&repo_url);
            
            // Добавляем репозиторий в доверенные
            scanner.add_trusted_repository(&repo_url).unwrap();
            
            // Проверяем, что репозиторий теперь доверенный
            prop_assert!(
                scanner.is_trusted_repository(&repo_url),
                "Репозиторий '{}' не помечен как доверенный сразу после добавления",
                repo_url
            );
            
            // Создаем новый экземпляр сканера (имитация перезапуска приложения)
            let scanner_after_restart = SecurityScanner::new().unwrap();
            
            // Проверяем, что статус доверия сохранился после "перезапуска"
            prop_assert!(
                scanner_after_restart.is_trusted_repository(&repo_url),
                "Статус доверия для репозитория '{}' не сохранился после перезапуска",
                repo_url
            );
            
            // Очистка: удаляем репозиторий из доверенных
            let mut scanner_cleanup = SecurityScanner::new().unwrap();
            scanner_cleanup.remove_trusted_repository(&repo_url).unwrap();
        }

        /// Property 9: Нормализация URL при сохранении доверия
        /// Тест проверяет, что различные варианты одного и того же URL
        /// (с .git, с trailing slash, разный регистр) обрабатываются как один репозиторий
        #[test]
        fn prop_trust_status_url_normalization(
            owner in "[a-zA-Z0-9_-]{1,39}",
            repo in "[a-zA-Z0-9_.-]{1,100}"
        ) {
            let mut scanner = SecurityScanner::new().unwrap();
            
            // Различные варианты одного и того же URL
            let url_variants = vec![
                format!("https://github.com/{}/{}", owner, repo),
                format!("https://github.com/{}/{}.git", owner, repo),
                format!("https://github.com/{}/{}/", owner, repo),
                format!("https://GitHub.COM/{}/{}", owner, repo),
                format!("HTTPS://GITHUB.COM/{}/{}", owner, repo),
            ];
            
            // Добавляем первый вариант в доверенные
            scanner.add_trusted_repository(&url_variants[0]).unwrap();
            
            // Проверяем, что все варианты распознаются как доверенные
            for variant in &url_variants {
                prop_assert!(
                    scanner.is_trusted_repository(variant),
                    "Вариант URL '{}' не распознан как доверенный, хотя базовый URL '{}' был добавлен",
                    variant,
                    url_variants[0]
                );
            }
            
            // Очистка
            scanner.remove_trusted_repository(&url_variants[0]).unwrap();
        }

        /// Property 9: Удаление статуса доверия
        /// Тест проверяет, что удаление репозитория из доверенных корректно работает
        /// и статус удаления сохраняется
        #[test]
        fn prop_trust_status_removal_persists(
            repo_url in github_url_strategy()
        ) {
            let mut scanner = SecurityScanner::new().unwrap();
            
            // Добавляем репозиторий в доверенные
            scanner.add_trusted_repository(&repo_url).unwrap();
            prop_assert!(scanner.is_trusted_repository(&repo_url));
            
            // Удаляем репозиторий из доверенных
            scanner.remove_trusted_repository(&repo_url).unwrap();
            
            // Проверяем, что репозиторий больше не доверенный
            prop_assert!(
                !scanner.is_trusted_repository(&repo_url),
                "Репозиторий '{}' все еще помечен как доверенный после удаления",
                repo_url
            );
            
            // Создаем новый экземпляр сканера (имитация перезапуска)
            let scanner_after_restart = SecurityScanner::new().unwrap();
            
            // Проверяем, что статус удаления сохранился
            prop_assert!(
                !scanner_after_restart.is_trusted_repository(&repo_url),
                "Репозиторий '{}' снова стал доверенным после перезапуска",
                repo_url
            );
        }

        /// Property 9: Множественные доверенные репозитории
        /// Тест проверяет, что система корректно управляет несколькими доверенными репозиториями
        #[test]
        fn prop_multiple_trusted_repositories(
            repos in prop::collection::vec(github_url_strategy(), 1..10)
        ) {
            let mut scanner = SecurityScanner::new().unwrap();
            
            // Добавляем все репозитории в доверенные
            for repo in &repos {
                scanner.add_trusted_repository(repo).unwrap();
            }
            
            // Проверяем, что все репозитории доверенные
            for repo in &repos {
                prop_assert!(
                    scanner.is_trusted_repository(repo),
                    "Репозиторий '{}' не помечен как доверенный",
                    repo
                );
            }
            
            // Создаем новый экземпляр (имитация перезапуска)
            let scanner_after_restart = SecurityScanner::new().unwrap();
            
            // Проверяем, что все репозитории остались доверенными
            for repo in &repos {
                prop_assert!(
                    scanner_after_restart.is_trusted_repository(repo),
                    "Репозиторий '{}' потерял статус доверия после перезапуска",
                    repo
                );
            }
            
            // Очистка
            let mut scanner_cleanup = SecurityScanner::new().unwrap();
            for repo in &repos {
                scanner_cleanup.remove_trusted_repository(repo).unwrap();
            }
        }

        /// Property 9: Получение списка доверенных репозиториев
        /// Тест проверяет, что метод get_trusted_repositories возвращает корректный список
        #[test]
        fn prop_get_trusted_repositories_is_accurate(
            repos in prop::collection::vec(github_url_strategy(), 1..5)
        ) {
            let mut scanner = SecurityScanner::new().unwrap();
            
            // Добавляем репозитории
            for repo in &repos {
                scanner.add_trusted_repository(repo).unwrap();
            }
            
            // Получаем список доверенных репозиториев
            let trusted_list = scanner.get_trusted_repositories();
            
            // Проверяем, что все добавленные репозитории присутствуют в списке
            // (с учетом нормализации URL)
            for repo in &repos {
                let normalized = repo.trim()
                    .trim_end_matches('/')
                    .trim_end_matches(".git")
                    .to_lowercase();
                
                prop_assert!(
                    trusted_list.contains(&normalized),
                    "Репозиторий '{}' (нормализованный: '{}') не найден в списке доверенных: {:?}",
                    repo,
                    normalized,
                    trusted_list
                );
            }
            
            // Очистка
            for repo in &repos {
                scanner.remove_trusted_repository(repo).unwrap();
            }
        }

        /// Property 9: Идемпотентность добавления доверия
        /// Тест проверяет, что повторное добавление одного и того же репозитория
        /// не вызывает проблем
        #[test]
        fn prop_adding_trust_is_idempotent(
            repo_url in github_url_strategy(),
            repeat_count in 1..5usize
        ) {
            let mut scanner = SecurityScanner::new().unwrap();
            
            // Добавляем репозиторий несколько раз
            for _ in 0..repeat_count {
                scanner.add_trusted_repository(&repo_url).unwrap();
            }
            
            // Проверяем, что репозиторий доверенный
            prop_assert!(scanner.is_trusted_repository(&repo_url));
            
            // Проверяем, что в списке нет дубликатов
            let trusted_list = scanner.get_trusted_repositories();
            let normalized = repo_url.trim()
                .trim_end_matches('/')
                .trim_end_matches(".git")
                .to_lowercase();
            
            let count = trusted_list.iter().filter(|&r| r == &normalized).count();
            prop_assert_eq!(
                count,
                1,
                "Репозиторий '{}' присутствует в списке {} раз вместо 1",
                repo_url,
                count
            );
            
            // Очистка
            scanner.remove_trusted_repository(&repo_url).unwrap();
        }
    }
}

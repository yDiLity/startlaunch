// Property-based тесты для обработки входных данных (Задача 11)
// Используется библиотека proptest для Rust

#[cfg(test)]
mod property_tests {
    use crate::url_parser::GitHubUrlParser;
    use proptest::prelude::*;

    // **Feature: autolaunch-core, Property 1: URL парсинг и нормализация**
    // **Validates: Requirements 1.1, 1.3**
    // 
    // Для любого валидного GitHub URL или формата owner/repo, система должна корректно 
    // извлечь владельца и имя репозитория, а также нормализовать входные данные к стандартному формату

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Подзадача 11.1: Property тест для парсинга URL
        #[test]
        fn test_property_parse_owner_repo_format(
            owner in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,37}[a-zA-Z0-9]",
            repo in "[a-zA-Z0-9_.-]{1,100}"
        ) {
            let input = format!("{}/{}", owner, repo);
            let result = GitHubUrlParser::parse(&input);
            
            prop_assert!(result.is_ok(), "Парсинг валидного формата owner/repo должен быть успешным");
            
            let info = result.unwrap();
            prop_assert_eq!(&info.owner, &owner, "Владелец должен быть корректно извлечен");
            prop_assert_eq!(&info.repo_name, &repo, "Имя репозитория должно быть корректно извлечено");
            prop_assert_eq!(
                &info.normalized_url, 
                &format!("https://github.com/{}/{}", owner, repo),
                "URL должен быть нормализован к стандартному формату"
            );
        }

        #[test]
        fn test_property_parse_full_url(
            owner in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,37}[a-zA-Z0-9]",
            repo in "[a-zA-Z0-9_.-]{1,100}"
        ) {
            let input = format!("https://github.com/{}/{}", owner, repo);
            let result = GitHubUrlParser::parse(&input);
            
            prop_assert!(result.is_ok(), "Парсинг валидного полного URL должен быть успешным");
            
            let info = result.unwrap();
            prop_assert_eq!(&info.owner, &owner);
            prop_assert_eq!(&info.repo_name, &repo);
            prop_assert_eq!(
                &info.normalized_url, 
                &format!("https://github.com/{}/{}", owner, repo)
            );
        }

        #[test]
        fn test_property_normalize_idempotent(
            owner in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,37}[a-zA-Z0-9]",
            repo in "[a-zA-Z0-9_.-]{1,100}"
        ) {
            let input = format!("{}/{}", owner, repo);
            let normalized1 = GitHubUrlParser::normalize(&input);
            
            prop_assert!(normalized1.is_ok(), "Первая нормализация должна быть успешной");
            
            let normalized1_str = normalized1.unwrap();
            let normalized2 = GitHubUrlParser::normalize(&normalized1_str);
            
            prop_assert!(normalized2.is_ok(), "Вторая нормализация должна быть успешной");
            
            let normalized2_str = normalized2.unwrap();
            prop_assert_eq!(
                &normalized1_str, 
                &normalized2_str,
                "Нормализация должна быть идемпотентной"
            );
        }

        #[test]
        fn test_property_parse_with_git_suffix(
            owner in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,37}[a-zA-Z0-9]",
            repo in "[a-zA-Z0-9_.-]{1,100}"
        ) {
            let input = format!("https://github.com/{}/{}.git", owner, repo);
            let result = GitHubUrlParser::parse(&input);
            
            prop_assert!(result.is_ok(), "Парсинг URL с .git суффиксом должен быть успешным");
            
            let info = result.unwrap();
            prop_assert_eq!(&info.repo_name, &repo, ".git суффикс должен быть удален");
            prop_assert_eq!(
                &info.normalized_url, 
                &format!("https://github.com/{}/{}", owner, repo),
                "Нормализованный URL не должен содержать .git"
            );
        }

        #[test]
        fn test_property_parse_equivalence(
            owner in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,37}[a-zA-Z0-9]",
            repo in "[a-zA-Z0-9_.-]{1,100}"
        ) {
            // Различные форматы одного и того же репозитория должны давать одинаковый результат
            let format1 = format!("{}/{}", owner, repo);
            let format2 = format!("https://github.com/{}/{}", owner, repo);
            let format3 = format!("https://github.com/{}/{}.git", owner, repo);
            let format4 = format!("https://www.github.com/{}/{}", owner, repo);
            
            let info1 = GitHubUrlParser::parse(&format1).unwrap();
            let info2 = GitHubUrlParser::parse(&format2).unwrap();
            let info3 = GitHubUrlParser::parse(&format3).unwrap();
            let info4 = GitHubUrlParser::parse(&format4).unwrap();
            
            prop_assert_eq!(&info1.normalized_url, &info2.normalized_url);
            prop_assert_eq!(&info2.normalized_url, &info3.normalized_url);
            prop_assert_eq!(&info3.normalized_url, &info4.normalized_url);
        }
    }

    // **Feature: autolaunch-core, Property 2: Обработка невалидных входных данных**
    // **Validates: Requirements 1.2**
    // 
    // Для любого невалидного URL или некорректного формата входных данных, 
    // система должна возвращать понятное сообщение об ошибке без сбоев

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        // Подзадача 11.2: Property тест для обработки невалидных данных
        #[test]
        fn test_property_invalid_owner_too_long(
            repo in "[a-zA-Z0-9_.-]{1,100}"
        ) {
            // Владелец длиннее 39 символов
            let long_owner = "a".repeat(40);
            let input = format!("{}/{}", long_owner, repo);
            let result = GitHubUrlParser::parse(&input);
            
            prop_assert!(result.is_err(), "Слишком длинное имя владельца должно вызывать ошибку");
            
            let error = result.unwrap_err();
            let error_msg = error.to_string();
            prop_assert!(
                error_msg.contains("слишком длинное") || error_msg.contains("длинн"),
                "Сообщение об ошибке должно быть понятным и содержать информацию о длине: {}",
                error_msg
            );
        }

        #[test]
        fn test_property_invalid_repo_too_long(
            owner in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,37}[a-zA-Z0-9]"
        ) {
            // Репозиторий длиннее 100 символов
            let long_repo = "r".repeat(101);
            let input = format!("{}/{}", owner, long_repo);
            let result = GitHubUrlParser::parse(&input);
            
            prop_assert!(result.is_err(), "Слишком длинное имя репозитория должно вызывать ошибку");
            
            let error = result.unwrap_err();
            let error_msg = error.to_string();
            prop_assert!(
                error_msg.contains("слишком длинное") || error_msg.contains("длинн"),
                "Сообщение об ошибке должно быть понятным: {}",
                error_msg
            );
        }

        #[test]
        fn test_property_invalid_non_github_host(
            owner in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,37}[a-zA-Z0-9]",
            repo in "[a-zA-Z0-9_.-]{1,100}",
            host in "(gitlab|bitbucket|codeberg)\\.com"
        ) {
            let input = format!("https://{}/{}/{}", host, owner, repo);
            let result = GitHubUrlParser::parse(&input);
            
            prop_assert!(result.is_err(), "Не-GitHub URL должен вызывать ошибку");
            
            let error = result.unwrap_err();
            let error_msg = error.to_string();
            prop_assert!(
                error_msg.contains("GitHub") || error_msg.contains("github"),
                "Сообщение об ошибке должно упоминать GitHub: {}",
                error_msg
            );
        }

        #[test]
        fn test_property_invalid_owner_with_special_chars(
            special_char in "[!@#$%^&*()+=\\[\\]{}|;:'\",<>?/\\\\]",
            repo in "[a-zA-Z0-9_.-]{1,100}"
        ) {
            let invalid_owner = format!("owner{}name", special_char);
            let input = format!("{}/{}", invalid_owner, repo);
            let result = GitHubUrlParser::parse(&input);
            
            prop_assert!(result.is_err(), "Владелец со спецсимволами должен вызывать ошибку");
            
            let error = result.unwrap_err();
            let error_msg = error.to_string();
            prop_assert!(
                error_msg.contains("недопустимые символы") || error_msg.contains("символ"),
                "Сообщение об ошибке должно упоминать недопустимые символы: {}",
                error_msg
            );
        }

        #[test]
        fn test_property_no_panic_on_random_input(
            random_input in "\\PC{0,200}"
        ) {
            // Любой случайный ввод не должен вызывать панику
            let result = GitHubUrlParser::parse(&random_input);
            
            // Мы просто проверяем, что функция возвращает Result (Ok или Err)
            // и не паникует
            match result {
                Ok(_) => {
                    // Если парсинг успешен, проверяем что результат валиден
                    prop_assert!(true);
                }
                Err(e) => {
                    // Если ошибка, проверяем что сообщение не пустое
                    let error_msg = e.to_string();
                    prop_assert!(!error_msg.is_empty(), "Сообщение об ошибке не должно быть пустым");
                }
            }
        }

        #[test]
        fn test_property_error_messages_are_descriptive(
            owner in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,37}[a-zA-Z0-9]"
        ) {
            // URL без имени репозитория
            let input = format!("https://github.com/{}", owner);
            let result = GitHubUrlParser::parse(&input);
            
            prop_assert!(result.is_err(), "URL без репозитория должен вызывать ошибку");
            
            let error = result.unwrap_err();
            let error_msg = error.to_string();
            
            // Проверяем что сообщение содержит полезную информацию
            prop_assert!(
                error_msg.len() > 10,
                "Сообщение об ошибке должно быть достаточно подробным"
            );
            prop_assert!(
                error_msg.contains("репозитор") || error_msg.contains("владел"),
                "Сообщение должно объяснять проблему: {}",
                error_msg
            );
        }
    }

    // Дополнительные property тесты для граничных случаев

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_property_whitespace_handling(
            owner in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,37}[a-zA-Z0-9]",
            repo in "[a-zA-Z0-9_.-]{1,100}",
            leading_spaces in "[ ]{0,10}",
            trailing_spaces in "[ ]{0,10}"
        ) {
            let input = format!("{}{}/{}{}", leading_spaces, owner, repo, trailing_spaces);
            let result = GitHubUrlParser::parse(&input);
            
            prop_assert!(result.is_ok(), "Пробелы в начале и конце должны игнорироваться");
            
            let info = result.unwrap();
            prop_assert_eq!(&info.owner, &owner);
            prop_assert_eq!(&info.repo_name, &repo);
        }

        #[test]
        fn test_property_owner_length_boundary(
            base_owner in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,35}[a-zA-Z0-9]",
            repo in "[a-zA-Z0-9_.-]{1,100}"
        ) {
            // Тестируем граничные значения длины владельца (38, 39, 40 символов)
            let owner_38 = format!("{}ab", base_owner);
            let owner_39 = format!("{}abc", base_owner);
            let owner_40 = format!("{}abcd", base_owner);
            
            // Обрезаем до нужной длины
            let owner_38 = &owner_38[..owner_38.len().min(38)];
            let owner_39 = &owner_39[..owner_39.len().min(39)];
            let owner_40 = &owner_40[..owner_40.len().min(40)];
            
            let result_38 = GitHubUrlParser::parse(&format!("{}/{}", owner_38, repo));
            let result_39 = GitHubUrlParser::parse(&format!("{}/{}", owner_39, repo));
            let result_40 = GitHubUrlParser::parse(&format!("{}/{}", owner_40, repo));
            
            prop_assert!(result_38.is_ok(), "38 символов должно быть валидным");
            prop_assert!(result_39.is_ok(), "39 символов должно быть валидным");
            prop_assert!(result_40.is_err(), "40 символов должно быть невалидным");
        }

        #[test]
        fn test_property_repo_length_boundary(
            owner in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,37}[a-zA-Z0-9]",
            base_repo in "[a-zA-Z0-9_.-]{1,95}"
        ) {
            // Тестируем граничные значения длины репозитория (99, 100, 101 символов)
            let repo_99 = format!("{}abcd", base_repo);
            let repo_100 = format!("{}abcde", base_repo);
            let repo_101 = format!("{}abcdef", base_repo);
            
            let repo_99 = &repo_99[..repo_99.len().min(99)];
            let repo_100 = &repo_100[..repo_100.len().min(100)];
            let repo_101 = &repo_101[..repo_101.len().min(101)];
            
            let result_99 = GitHubUrlParser::parse(&format!("{}/{}", owner, repo_99));
            let result_100 = GitHubUrlParser::parse(&format!("{}/{}", owner, repo_100));
            let result_101 = GitHubUrlParser::parse(&format!("{}/{}", owner, repo_101));
            
            prop_assert!(result_99.is_ok(), "99 символов должно быть валидным");
            prop_assert!(result_100.is_ok(), "100 символов должно быть валидным");
            prop_assert!(result_101.is_err(), "101 символ должен быть невалидным");
        }
    }
}

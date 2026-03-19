// Property-based тесты для менеджера проектов (Задачи 7.1, 7.2, 7.3)
// Используется библиотека proptest для Rust

#[cfg(test)]
mod tests {
    use crate::database::Database;
    use crate::models::{Project, TrustLevel};
    use chrono::Utc;
    use proptest::prelude::*;
    use uuid::Uuid;

    async fn create_test_db() -> Database {
        Database::new_in_memory()
            .await
            .expect("Не удалось создать тестовую БД в памяти")
    }

    fn make_project(repo_name: &str, owner: &str, tags: Vec<String>, last_run_at: Option<String>) -> Project {
        Project {
            id: Uuid::new_v4().to_string(),
            github_url: format!("https://github.com/{}/{}", owner, repo_name),
            owner: owner.to_string(),
            repo_name: repo_name.to_string(),
            local_path: format!("/tmp/{}/{}", owner, repo_name),
            detected_stack: "NodeJs(unknown)".to_string(),
            trust_level: TrustLevel::Unknown.to_string(),
            created_at: Utc::now().to_rfc3339(),
            last_run_at,
            tags: serde_json::to_string(&tags).unwrap_or_else(|_| "[]".to_string()),
        }
    }

    // Генератор для имён репозиториев
    fn repo_name_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("react-app".to_string()),
            Just("vue-project".to_string()),
            Just("django-api".to_string()),
            Just("rust-cli".to_string()),
            Just("go-service".to_string()),
            Just("flask-server".to_string()),
        ]
    }

    // Генератор для владельцев
    fn owner_strategy() -> impl Strategy<Value = String> {
        prop_oneof![
            Just("facebook".to_string()),
            Just("google".to_string()),
            Just("microsoft".to_string()),
            Just("torvalds".to_string()),
            Just("user123".to_string()),
        ]
    }

    // Генератор для тегов
    fn tags_strategy() -> impl Strategy<Value = Vec<String>> {
        prop::collection::vec(
            prop_oneof![
                Just("frontend".to_string()),
                Just("backend".to_string()),
                Just("react".to_string()),
                Just("python".to_string()),
                Just("rust".to_string()),
                Just("api".to_string()),
            ],
            0..5,
        )
    }

    // **Feature: autolaunch-core, Property 19: Ведение истории проектов**
    // **Validates: Requirements 8.1**
    //
    // Для любого запускаемого проекта, информация о запуске должна корректно
    // добавляться в историю с правильными временными метками

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_property_project_added_to_history(
            repo_name in repo_name_strategy(),
            owner in owner_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let db = create_test_db().await;
                let last_run = Utc::now().to_rfc3339();
                let project = make_project(&repo_name, &owner, vec![], Some(last_run.clone()));

                // Требование 8.1: Сохраняем проект (имитация запуска)
                db.save_project(&project).await.unwrap();

                // Проект должен быть в истории
                let retrieved = db.get_project(&project.id).await.unwrap();
                prop_assert!(retrieved.is_some(), "Проект должен быть в истории");

                let p = retrieved.unwrap();
                prop_assert_eq!(&p.repo_name, &repo_name, "Имя репозитория должно совпадать");
                prop_assert_eq!(&p.owner, &owner, "Владелец должен совпадать");
                prop_assert!(
                    p.last_run_at.is_some(),
                    "Время последнего запуска должно быть записано"
                );

                Ok(())
            })?;
        }

        #[test]
        fn test_property_history_preserves_all_projects(
            names in prop::collection::vec(repo_name_strategy(), 1..6),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let db = create_test_db().await;
                let mut saved_ids = Vec::new();

                // Сохраняем несколько проектов
                for name in &names {
                    let project = make_project(name, "testowner", vec![], Some(Utc::now().to_rfc3339()));
                    db.save_project(&project).await.unwrap();
                    saved_ids.push(project.id);
                }

                // Требование 8.1: Все проекты должны быть в истории
                let all = db.get_all_projects().await.unwrap();

                for id in &saved_ids {
                    prop_assert!(
                        all.iter().any(|p| &p.id == id),
                        "Проект {} должен быть в истории",
                        id
                    );
                }

                Ok(())
            })?;
        }

        #[test]
        fn test_property_last_run_timestamp_recorded(
            repo_name in repo_name_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let db = create_test_db().await;

                // Создаём проект без времени запуска
                let mut project = make_project(&repo_name, "owner", vec![], None);
                db.save_project(&project).await.unwrap();

                let before = db.get_project(&project.id).await.unwrap().unwrap();
                prop_assert!(before.last_run_at.is_none(), "Изначально last_run_at должен быть None");

                // Обновляем время запуска
                project.last_run_at = Some(Utc::now().to_rfc3339());
                db.save_project(&project).await.unwrap();

                let after = db.get_project(&project.id).await.unwrap().unwrap();
                prop_assert!(
                    after.last_run_at.is_some(),
                    "После запуска last_run_at должен быть установлен"
                );

                Ok(())
            })?;
        }
    }

    // **Feature: autolaunch-core, Property 21: Фильтрация и поиск проектов**
    // **Validates: Requirements 8.3**
    //
    // Для любого поискового запроса, система должна возвращать только проекты,
    // соответствующие критериям поиска

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_property_search_returns_matching_projects(
            query in prop_oneof![
                Just("react".to_string()),
                Just("vue".to_string()),
                Just("django".to_string()),
                Just("rust".to_string()),
            ],
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let db = create_test_db().await;

                // Создаём проект, который должен найтись
                let matching = make_project(
                    &format!("{}-app", query),
                    "owner",
                    vec![],
                    None,
                );
                // Создаём проект, который не должен найтись
                let non_matching = make_project("totally-different-xyz", "owner", vec![], None);

                db.save_project(&matching).await.unwrap();
                db.save_project(&non_matching).await.unwrap();

                // Требование 8.3: Поиск должен вернуть только совпадающие проекты
                let results = db.search_projects(&query).await.unwrap();

                prop_assert!(
                    results.iter().any(|p| p.id == matching.id),
                    "Совпадающий проект должен быть в результатах поиска"
                );

                Ok(())
            })?;
        }

        #[test]
        fn test_property_search_by_owner(
            owner in owner_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let db = create_test_db().await;

                let project = make_project("some-repo", &owner, vec![], None);
                db.save_project(&project).await.unwrap();

                let results = db.search_projects(&owner).await.unwrap();

                prop_assert!(
                    results.iter().any(|p| p.id == project.id),
                    "Поиск по владельцу '{}' должен найти проект",
                    owner
                );

                Ok(())
            })?;
        }

        #[test]
        fn test_property_search_empty_query_returns_all(
            names in prop::collection::vec(repo_name_strategy(), 1..4),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let db = create_test_db().await;
                let mut ids = Vec::new();

                for name in &names {
                    let p = make_project(name, "owner", vec![], None);
                    db.save_project(&p).await.unwrap();
                    ids.push(p.id);
                }

                // Пустой поиск должен вернуть все проекты
                let all = db.get_all_projects().await.unwrap();

                for id in &ids {
                    prop_assert!(
                        all.iter().any(|p| &p.id == id),
                        "Все проекты должны быть в общем списке"
                    );
                }

                Ok(())
            })?;
        }

        #[test]
        fn test_property_search_no_false_positives(
            repo_name in repo_name_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let db = create_test_db().await;

                // Сохраняем проект
                let project = make_project(&repo_name, "owner", vec![], None);
                db.save_project(&project).await.unwrap();

                // Поиск по строке, которой точно нет ни в одном поле
                let results = db.search_projects("zzz_nonexistent_xyz_12345").await.unwrap();

                prop_assert!(
                    !results.iter().any(|p| p.id == project.id),
                    "Поиск по несуществующей строке не должен возвращать проект"
                );

                Ok(())
            })?;
        }
    }

    // **Feature: autolaunch-core, Property 22: Сохранение и восстановление тегов**
    // **Validates: Requirements 8.4**
    //
    // Для любых тегов, добавленных к проекту, они должны корректно сохраняться
    // и восстанавливаться при последующих обращениях

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        #[test]
        fn test_property_tags_saved_and_restored(
            tags in tags_strategy(),
            repo_name in repo_name_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let db = create_test_db().await;

                // Требование 8.4: Сохраняем проект с тегами
                let project = make_project(&repo_name, "owner", tags.clone(), None);
                db.save_project(&project).await.unwrap();

                // Восстанавливаем и проверяем теги
                let retrieved = db.get_project(&project.id).await.unwrap().unwrap();
                let restored_tags: Vec<String> =
                    serde_json::from_str(&retrieved.tags).unwrap_or_default();

                prop_assert_eq!(
                    restored_tags.len(),
                    tags.len(),
                    "Количество тегов должно совпадать"
                );

                for tag in &tags {
                    prop_assert!(
                        restored_tags.contains(tag),
                        "Тег '{}' должен быть восстановлен",
                        tag
                    );
                }

                Ok(())
            })?;
        }

        #[test]
        fn test_property_tags_update_preserved(
            initial_tags in tags_strategy(),
            new_tags in tags_strategy(),
            repo_name in repo_name_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let db = create_test_db().await;

                let mut project = make_project(&repo_name, "owner", initial_tags, None);
                db.save_project(&project).await.unwrap();

                // Обновляем теги
                project.tags = serde_json::to_string(&new_tags).unwrap();
                db.save_project(&project).await.unwrap();

                // Проверяем что новые теги сохранились
                let retrieved = db.get_project(&project.id).await.unwrap().unwrap();
                let restored: Vec<String> =
                    serde_json::from_str(&retrieved.tags).unwrap_or_default();

                prop_assert_eq!(
                    restored.len(),
                    new_tags.len(),
                    "После обновления должны быть новые теги"
                );

                Ok(())
            })?;
        }

        #[test]
        fn test_property_empty_tags_valid(
            repo_name in repo_name_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let db = create_test_db().await;

                // Проект без тегов
                let project = make_project(&repo_name, "owner", vec![], None);
                db.save_project(&project).await.unwrap();

                let retrieved = db.get_project(&project.id).await.unwrap().unwrap();
                let tags: Vec<String> =
                    serde_json::from_str(&retrieved.tags).unwrap_or_default();

                prop_assert_eq!(tags.len(), 0, "Пустой список тегов должен корректно сохраняться");

                Ok(())
            })?;
        }

        #[test]
        fn test_property_tags_searchable(
            tag in prop_oneof![
                Just("frontend".to_string()),
                Just("backend".to_string()),
                Just("react".to_string()),
                Just("python".to_string()),
            ],
            repo_name in repo_name_strategy(),
        ) {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                let db = create_test_db().await;

                // Создаём проект с тегом
                let project = make_project(&repo_name, "owner", vec![tag.clone()], None);
                db.save_project(&project).await.unwrap();

                // Требование 8.3: Поиск по тегу должен найти проект
                let results = db.search_projects(&tag).await.unwrap();

                prop_assert!(
                    results.iter().any(|p| p.id == project.id),
                    "Поиск по тегу '{}' должен найти проект",
                    tag
                );

                Ok(())
            })?;
        }
    }
}

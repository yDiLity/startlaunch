// Property-based тесты для сохранения данных проекта в БД (Задача 11, подзадача 11.3)

#[cfg(test)]
mod database_property_tests {
    use crate::database::Database;
    use crate::models::{Project, TrustLevel};
    use proptest::prelude::*;
    use uuid::Uuid;
    use chrono::Utc;

    // **Feature: autolaunch-core, Property 3: Сохранение данных проекта**
    // **Validates: Requirements 1.5**
    // 
    // Для любого успешно проанализированного проекта, информация о проекте должна быть 
    // сохранена в локальной базе данных с корректными метаданными

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(50))]

        // Подзадача 11.3: Property тест для сохранения данных
        #[test]
        fn test_property_save_and_retrieve_project(
            owner in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,37}[a-zA-Z0-9]",
            repo_name in "[a-zA-Z0-9_.-]{1,100}",
            stack in "(NodeJs|Python|Rust|Go|Docker|Unknown)"
        ) {
            // Создаем временную базу данных для теста
            let runtime = tokio::runtime::Runtime::new().unwrap();
            
            runtime.block_on(async {
                // Используем in-memory SQLite для тестов
                let db = create_test_database().await;
                
                let github_url = format!("https://github.com/{}/{}", owner, repo_name);
                let local_path = format!("/tmp/autolaunch/{}_{}", owner, repo_name);
                
                let project = Project {
                    id: Uuid::new_v4().to_string(),
                    github_url: github_url.clone(),
                    owner: owner.clone(),
                    repo_name: repo_name.clone(),
                    local_path: local_path.clone(),
                    detected_stack: stack.to_string(),
                    trust_level: TrustLevel::Unknown.to_string(),
                    created_at: Utc::now().to_rfc3339(),
                    last_run_at: None,
                    tags: "[]".to_string(),
                };
                
                // Сохраняем проект
                let save_result = db.save_project(&project).await;
                prop_assert!(save_result.is_ok(), "Сохранение проекта должно быть успешным");
                
                // Извлекаем проект
                let retrieved = db.get_project(&project.id).await;
                prop_assert!(retrieved.is_ok(), "Извлечение проекта должно быть успешным");
                
                let retrieved_project = retrieved.unwrap();
                prop_assert!(retrieved_project.is_some(), "Проект должен быть найден в БД");
                
                let retrieved_project = retrieved_project.unwrap();
                
                // Проверяем корректность сохраненных данных
                prop_assert_eq!(&retrieved_project.id, &project.id, "ID должен совпадать");
                prop_assert_eq!(&retrieved_project.github_url, &github_url, "GitHub URL должен совпадать");
                prop_assert_eq!(&retrieved_project.owner, &owner, "Владелец должен совпадать");
                prop_assert_eq!(&retrieved_project.repo_name, &repo_name, "Имя репозитория должно совпадать");
                prop_assert_eq!(&retrieved_project.local_path, &local_path, "Локальный путь должен совпадать");
                prop_assert_eq!(&retrieved_project.detected_stack, &stack, "Стек технологий должен совпадать");
                
                Ok(())
            })?;
        }

        #[test]
        fn test_property_update_project(
            owner in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,37}[a-zA-Z0-9]",
            repo_name in "[a-zA-Z0-9_.-]{1,100}",
            initial_stack in "(NodeJs|Python|Rust)",
            updated_stack in "(Go|Docker|Unknown)"
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            
            runtime.block_on(async {
                let db = create_test_database().await;
                
                let project_id = Uuid::new_v4().to_string();
                let github_url = format!("https://github.com/{}/{}", owner, repo_name);
                
                // Создаем начальный проект
                let mut project = Project {
                    id: project_id.clone(),
                    github_url: github_url.clone(),
                    owner: owner.clone(),
                    repo_name: repo_name.clone(),
                    local_path: format!("/tmp/autolaunch/{}_{}", owner, repo_name),
                    detected_stack: initial_stack.to_string(),
                    trust_level: TrustLevel::Unknown.to_string(),
                    created_at: Utc::now().to_rfc3339(),
                    last_run_at: None,
                    tags: "[]".to_string(),
                };
                
                // Сохраняем
                db.save_project(&project).await.unwrap();
                
                // Обновляем стек
                project.detected_stack = updated_stack.to_string();
                project.last_run_at = Some(Utc::now().to_rfc3339());
                
                // Сохраняем обновленную версию
                let update_result = db.save_project(&project).await;
                prop_assert!(update_result.is_ok(), "Обновление проекта должно быть успешным");
                
                // Извлекаем и проверяем
                let retrieved = db.get_project(&project_id).await.unwrap().unwrap();
                
                prop_assert_eq!(&retrieved.detected_stack, &updated_stack, "Стек должен быть обновлен");
                prop_assert!(retrieved.last_run_at.is_some(), "Время последнего запуска должно быть установлено");
                
                Ok(())
            })?;
        }

        #[test]
        fn test_property_save_multiple_projects(
            projects_count in 1usize..10,
            owner_prefix in "[a-zA-Z0-9]{3,10}"
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            
            runtime.block_on(async {
                let db = create_test_database().await;
                
                let mut project_ids = Vec::new();
                
                // Сохраняем несколько проектов
                for i in 0..projects_count {
                    let project = Project {
                        id: Uuid::new_v4().to_string(),
                        github_url: format!("https://github.com/{}/repo{}", owner_prefix, i),
                        owner: owner_prefix.clone(),
                        repo_name: format!("repo{}", i),
                        local_path: format!("/tmp/autolaunch/{}_repo{}", owner_prefix, i),
                        detected_stack: "NodeJs".to_string(),
                        trust_level: TrustLevel::Unknown.to_string(),
                        created_at: Utc::now().to_rfc3339(),
                        last_run_at: None,
                        tags: "[]".to_string(),
                    };
                    
                    project_ids.push(project.id.clone());
                    db.save_project(&project).await.unwrap();
                }
                
                // Проверяем что все проекты сохранены
                let all_projects = db.get_all_projects().await.unwrap();
                
                prop_assert!(
                    all_projects.len() >= projects_count,
                    "Все проекты должны быть сохранены в БД"
                );
                
                // Проверяем что каждый проект можно извлечь по ID
                for project_id in project_ids {
                    let retrieved = db.get_project(&project_id).await.unwrap();
                    prop_assert!(retrieved.is_some(), "Каждый проект должен быть найден по ID");
                }
                
                Ok(())
            })?;
        }

        #[test]
        fn test_property_delete_project(
            owner in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,37}[a-zA-Z0-9]",
            repo_name in "[a-zA-Z0-9_.-]{1,100}"
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            
            runtime.block_on(async {
                let db = create_test_database().await;
                
                let project = Project {
                    id: Uuid::new_v4().to_string(),
                    github_url: format!("https://github.com/{}/{}", owner, repo_name),
                    owner: owner.clone(),
                    repo_name: repo_name.clone(),
                    local_path: format!("/tmp/autolaunch/{}_{}", owner, repo_name),
                    detected_stack: "NodeJs".to_string(),
                    trust_level: TrustLevel::Unknown.to_string(),
                    created_at: Utc::now().to_rfc3339(),
                    last_run_at: None,
                    tags: "[]".to_string(),
                };
                
                // Сохраняем
                db.save_project(&project).await.unwrap();
                
                // Проверяем что проект существует
                let exists = db.get_project(&project.id).await.unwrap();
                prop_assert!(exists.is_some(), "Проект должен существовать после сохранения");
                
                // Удаляем
                let delete_result = db.delete_project(&project.id).await;
                prop_assert!(delete_result.is_ok(), "Удаление должно быть успешным");
                
                // Проверяем что проект удален
                let not_exists = db.get_project(&project.id).await.unwrap();
                prop_assert!(not_exists.is_none(), "Проект не должен существовать после удаления");
                
                Ok(())
            })?;
        }

        #[test]
        fn test_property_search_projects(
            owner in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,37}[a-zA-Z0-9]",
            repo_name in "[a-zA-Z0-9_.-]{1,100}",
            search_term in "[a-zA-Z0-9]{3,10}"
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            
            runtime.block_on(async {
                let db = create_test_database().await;
                
                // Создаем проект с поисковым термином в имени
                let repo_with_term = format!("{}{}", search_term, repo_name);
                
                let project = Project {
                    id: Uuid::new_v4().to_string(),
                    github_url: format!("https://github.com/{}/{}", owner, repo_with_term),
                    owner: owner.clone(),
                    repo_name: repo_with_term.clone(),
                    local_path: format!("/tmp/autolaunch/{}_{}", owner, repo_with_term),
                    detected_stack: "NodeJs".to_string(),
                    trust_level: TrustLevel::Unknown.to_string(),
                    created_at: Utc::now().to_rfc3339(),
                    last_run_at: None,
                    tags: "[]".to_string(),
                };
                
                db.save_project(&project).await.unwrap();
                
                // Ищем по термину
                let search_results = db.search_projects(&search_term).await.unwrap();
                
                prop_assert!(
                    !search_results.is_empty(),
                    "Поиск должен найти проект с поисковым термином"
                );
                
                // Проверяем что найденный проект содержит поисковый термин
                let found = search_results.iter().any(|p| 
                    p.repo_name.contains(&search_term) || 
                    p.owner.contains(&search_term)
                );
                
                prop_assert!(found, "Найденный проект должен содержать поисковый термин");
                
                Ok(())
            })?;
        }

        #[test]
        fn test_property_tags_persistence(
            owner in "[a-zA-Z0-9][a-zA-Z0-9_-]{0,37}[a-zA-Z0-9]",
            repo_name in "[a-zA-Z0-9_.-]{1,100}",
            tag1 in "[a-z]{3,10}",
            tag2 in "[a-z]{3,10}"
        ) {
            let runtime = tokio::runtime::Runtime::new().unwrap();
            
            runtime.block_on(async {
                let db = create_test_database().await;
                
                let tags = vec![tag1.clone(), tag2.clone()];
                let tags_json = serde_json::to_string(&tags).unwrap();
                
                let project = Project {
                    id: Uuid::new_v4().to_string(),
                    github_url: format!("https://github.com/{}/{}", owner, repo_name),
                    owner: owner.clone(),
                    repo_name: repo_name.clone(),
                    local_path: format!("/tmp/autolaunch/{}_{}", owner, repo_name),
                    detected_stack: "NodeJs".to_string(),
                    trust_level: TrustLevel::Unknown.to_string(),
                    created_at: Utc::now().to_rfc3339(),
                    last_run_at: None,
                    tags: tags_json.clone(),
                };
                
                db.save_project(&project).await.unwrap();
                
                // Извлекаем и проверяем теги
                let retrieved = db.get_project(&project.id).await.unwrap().unwrap();
                
                prop_assert_eq!(&retrieved.tags, &tags_json, "Теги должны быть сохранены корректно");
                
                // Парсим теги обратно
                let retrieved_tags: Vec<String> = serde_json::from_str(&retrieved.tags).unwrap();
                prop_assert_eq!(retrieved_tags.len(), 2, "Должно быть 2 тега");
                prop_assert!(retrieved_tags.contains(&tag1), "Первый тег должен присутствовать");
                prop_assert!(retrieved_tags.contains(&tag2), "Второй тег должен присутствовать");
                
                Ok(())
            })?;
        }
    }

    // Вспомогательная функция для создания тестовой базы данных
    async fn create_test_database() -> Database {
        use crate::database::Database;
        
        // Создаем in-memory базу данных для тестов
        Database::new_in_memory()
            .await
            .expect("Не удалось создать тестовую БД")
    }
}

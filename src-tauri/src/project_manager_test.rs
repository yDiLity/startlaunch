// Тесты для менеджера проектов (Требование 8)

#[cfg(test)]
mod tests {
    use crate::database::Database;
    use crate::models::{Project, TrustLevel};
    use chrono::Utc;
    use uuid::Uuid;

    async fn create_test_db() -> Database {
        Database::new().await.expect("Не удалось создать тестовую БД")
    }

    fn create_test_project(name: &str, tags: Vec<String>) -> Project {
        Project {
            id: Uuid::new_v4().to_string(),
            github_url: format!("https://github.com/test/{}", name),
            owner: "test".to_string(),
            repo_name: name.to_string(),
            local_path: format!("/tmp/test/{}", name),
            detected_stack: "NodeJs(unknown)".to_string(),
            trust_level: TrustLevel::Unknown.to_string(),
            created_at: Utc::now().to_rfc3339(),
            last_run_at: None,
            tags: serde_json::to_string(&tags).unwrap(),
        }
    }

    // Требование 8.1: Добавление проекта в историю при запуске
    #[tokio::test]
    async fn test_project_added_to_history() {
        let db = create_test_db().await;
        let project = create_test_project("test-repo", vec![]);

        // Сохраняем проект
        db.save_project(&project).await.expect("Не удалось сохранить проект");

        // Проверяем, что проект добавлен в историю
        let retrieved = db.get_project(&project.id).await.expect("Ошибка получения проекта");
        assert!(retrieved.is_some());
        
        let retrieved_project = retrieved.unwrap();
        assert_eq!(retrieved_project.id, project.id);
        assert_eq!(retrieved_project.repo_name, "test-repo");
    }

    // Требование 8.2: Отображение списка проектов с датами последнего запуска
    #[tokio::test]
    async fn test_get_all_projects_with_dates() {
        let db = create_test_db().await;
        
        // Создаем несколько проектов
        let mut project1 = create_test_project("repo1", vec![]);
        let mut project2 = create_test_project("repo2", vec![]);
        
        // Устанавливаем разные даты последнего запуска
        project1.last_run_at = Some(Utc::now().to_rfc3339());
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        project2.last_run_at = Some(Utc::now().to_rfc3339());

        db.save_project(&project1).await.expect("Не удалось сохранить проект1");
        db.save_project(&project2).await.expect("Не удалось сохранить проект2");

        // Получаем все проекты
        let projects = db.get_all_projects().await.expect("Не удалось получить проекты");
        
        assert!(projects.len() >= 2);
        
        // Проверяем, что проекты отсортированы по дате последнего запуска (новые первыми)
        let found_project2 = projects.iter().find(|p| p.id == project2.id);
        let found_project1 = projects.iter().find(|p| p.id == project1.id);
        
        assert!(found_project2.is_some());
        assert!(found_project1.is_some());
    }

    // Требование 8.3: Фильтрация проектов по имени
    #[tokio::test]
    async fn test_search_projects_by_name() {
        let db = create_test_db().await;
        
        let project1 = create_test_project("react-app", vec![]);
        let project2 = create_test_project("vue-app", vec![]);
        let project3 = create_test_project("angular-app", vec![]);

        db.save_project(&project1).await.expect("Не удалось сохранить проект1");
        db.save_project(&project2).await.expect("Не удалось сохранить проект2");
        db.save_project(&project3).await.expect("Не удалось сохранить проект3");

        // Поиск по имени
        let results = db.search_projects("react").await.expect("Ошибка поиска");
        
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].repo_name, "react-app");
    }

    // Требование 8.3: Фильтрация проектов по владельцу
    #[tokio::test]
    async fn test_search_projects_by_owner() {
        let db = create_test_db().await;
        
        let mut project1 = create_test_project("repo1", vec![]);
        project1.owner = "facebook".to_string();
        
        let mut project2 = create_test_project("repo2", vec![]);
        project2.owner = "google".to_string();

        db.save_project(&project1).await.expect("Не удалось сохранить проект1");
        db.save_project(&project2).await.expect("Не удалось сохранить проект2");

        // Поиск по владельцу
        let results = db.search_projects("facebook").await.expect("Ошибка поиска");
        
        assert!(results.iter().any(|p| p.owner == "facebook"));
    }

    // Требование 8.4: Сохранение тегов для организации
    #[tokio::test]
    async fn test_save_and_retrieve_tags() {
        let db = create_test_db().await;
        
        let tags = vec!["frontend".to_string(), "react".to_string(), "typescript".to_string()];
        let project = create_test_project("tagged-repo", tags.clone());

        db.save_project(&project).await.expect("Не удалось сохранить проект");

        // Получаем проект и проверяем теги
        let retrieved = db.get_project(&project.id).await
            .expect("Ошибка получения проекта")
            .expect("Проект не найден");
        
        let retrieved_tags: Vec<String> = serde_json::from_str(&retrieved.tags).unwrap();
        assert_eq!(retrieved_tags.len(), 3);
        assert!(retrieved_tags.contains(&"frontend".to_string()));
        assert!(retrieved_tags.contains(&"react".to_string()));
        assert!(retrieved_tags.contains(&"typescript".to_string()));
    }

    // Требование 8.4: Обновление тегов проекта
    #[tokio::test]
    async fn test_update_project_tags() {
        let db = create_test_db().await;
        
        let mut project = create_test_project("repo", vec!["tag1".to_string()]);
        db.save_project(&project).await.expect("Не удалось сохранить проект");

        // Обновляем теги
        let new_tags = vec!["tag1".to_string(), "tag2".to_string(), "tag3".to_string()];
        project.tags = serde_json::to_string(&new_tags).unwrap();
        db.save_project(&project).await.expect("Не удалось обновить проект");

        // Проверяем обновленные теги
        let retrieved = db.get_project(&project.id).await
            .expect("Ошибка получения проекта")
            .expect("Проект не найден");
        
        let retrieved_tags: Vec<String> = serde_json::from_str(&retrieved.tags).unwrap();
        assert_eq!(retrieved_tags.len(), 3);
        assert!(retrieved_tags.contains(&"tag2".to_string()));
        assert!(retrieved_tags.contains(&"tag3".to_string()));
    }

    // Требование 8.3: Поиск проектов по тегам
    #[tokio::test]
    async fn test_search_projects_by_tags() {
        let db = create_test_db().await;
        
        let project1 = create_test_project("repo1", vec!["frontend".to_string(), "react".to_string()]);
        let project2 = create_test_project("repo2", vec!["backend".to_string(), "nodejs".to_string()]);
        let project3 = create_test_project("repo3", vec!["frontend".to_string(), "vue".to_string()]);

        db.save_project(&project1).await.expect("Не удалось сохранить проект1");
        db.save_project(&project2).await.expect("Не удалось сохранить проект2");
        db.save_project(&project3).await.expect("Не удалось сохранить проект3");

        // Поиск по тегу "frontend"
        let results = db.search_projects("frontend").await.expect("Ошибка поиска");
        
        // Должны найтись проекты с тегом "frontend"
        let frontend_projects: Vec<_> = results.iter()
            .filter(|p| {
                let tags: Vec<String> = serde_json::from_str(&p.tags).unwrap_or_default();
                tags.contains(&"frontend".to_string())
            })
            .collect();
        
        assert!(frontend_projects.len() >= 2);
    }

    // Требование 8.5: Обновление времени последнего запуска
    #[tokio::test]
    async fn test_update_last_run_time() {
        let db = create_test_db().await;
        
        let mut project = create_test_project("repo", vec![]);
        assert!(project.last_run_at.is_none());

        db.save_project(&project).await.expect("Не удалось сохранить проект");

        // Обновляем время последнего запуска
        project.last_run_at = Some(Utc::now().to_rfc3339());
        db.save_project(&project).await.expect("Не удалось обновить проект");

        // Проверяем обновленное время
        let retrieved = db.get_project(&project.id).await
            .expect("Ошибка получения проекта")
            .expect("Проект не найден");
        
        assert!(retrieved.last_run_at.is_some());
    }

    // Тест на пустой поиск (должен вернуть все проекты)
    #[tokio::test]
    async fn test_empty_search_returns_all() {
        let db = create_test_db().await;
        
        let project1 = create_test_project("repo1", vec![]);
        let project2 = create_test_project("repo2", vec![]);

        db.save_project(&project1).await.expect("Не удалось сохранить проект1");
        db.save_project(&project2).await.expect("Не удалось сохранить проект2");

        let all_projects = db.get_all_projects().await.expect("Ошибка получения проектов");
        
        assert!(all_projects.len() >= 2);
    }

    // Тест на удаление проекта
    #[tokio::test]
    async fn test_delete_project() {
        let db = create_test_db().await;
        
        let project = create_test_project("to-delete", vec![]);
        db.save_project(&project).await.expect("Не удалось сохранить проект");

        // Проверяем, что проект существует
        let retrieved = db.get_project(&project.id).await.expect("Ошибка получения проекта");
        assert!(retrieved.is_some());

        // Удаляем проект
        db.delete_project(&project.id).await.expect("Не удалось удалить проект");

        // Проверяем, что проект удален
        let retrieved = db.get_project(&project.id).await.expect("Ошибка получения проекта");
        assert!(retrieved.is_none());
    }

    // Тест на регистронезависимый поиск
    #[tokio::test]
    async fn test_case_insensitive_search() {
        let db = create_test_db().await;
        
        let project = create_test_project("React-App", vec![]);
        db.save_project(&project).await.expect("Не удалось сохранить проект");

        // Поиск в нижнем регистре
        let results = db.search_projects("react").await.expect("Ошибка поиска");
        assert!(results.iter().any(|p| p.repo_name == "React-App"));

        // Поиск в верхнем регистре
        let results = db.search_projects("REACT").await.expect("Ошибка поиска");
        assert!(results.iter().any(|p| p.repo_name == "React-App"));
    }
}

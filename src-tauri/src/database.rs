use crate::error::{AutoLaunchError, Result};
use crate::models::{Project, ProjectSnapshot};
use sqlx::{sqlite::SqlitePool, Row};

pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn new() -> Result<Self> {
        tracing::info!("Инициализация базы данных");
        
        // Создаем директорию для базы данных если её нет
        let db_dir = dirs::config_dir()
            .ok_or_else(|| AutoLaunchError::Database(sqlx::Error::Configuration("Не удалось найти директорию конфигурации".into())))?
            .join("autolaunch");
        
        if !db_dir.exists() {
            tracing::info!("Создание директории для базы данных: {:?}", db_dir);
            std::fs::create_dir_all(&db_dir)?;
        }

        let db_path = db_dir.join("autolaunch.db");
        let database_url = format!("sqlite:{}", db_path.display());
        
        tracing::info!("Подключение к базе данных: {}", database_url);
        let pool = SqlitePool::connect(&database_url).await?;
        
        let db = Database { pool };
        db.run_migrations().await?;
        
        tracing::info!("База данных успешно инициализирована");
        Ok(db)
    }

    #[cfg(test)]
    pub async fn new_in_memory() -> Result<Self> {
        use sqlx::sqlite::SqlitePoolOptions;
        
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await?;
        
        let db = Database { pool };
        db.run_migrations().await?;
        
        Ok(db)
    }

    pub(crate) async fn run_migrations(&self) -> Result<()> {
        tracing::info!("Запуск миграций базы данных");
        
        // Создание таблицы проектов
        tracing::debug!("Создание таблицы projects");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS projects (
                id TEXT PRIMARY KEY,
                github_url TEXT NOT NULL,
                owner TEXT NOT NULL,
                repo_name TEXT NOT NULL,
                local_path TEXT NOT NULL,
                detected_stack TEXT NOT NULL,
                trust_level TEXT NOT NULL,
                created_at TEXT NOT NULL,
                last_run_at TEXT,
                tags TEXT DEFAULT '[]'
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Создание таблицы снимков
        tracing::debug!("Создание таблицы snapshots");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS snapshots (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                snapshot_path TEXT NOT NULL,
                environment_type TEXT NOT NULL,
                metadata TEXT NOT NULL,
                created_at TEXT NOT NULL,
                size_bytes INTEGER NOT NULL,
                FOREIGN KEY (project_id) REFERENCES projects(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Создание таблицы выполнений
        tracing::debug!("Создание таблицы executions");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS executions (
                id TEXT PRIMARY KEY,
                project_id TEXT NOT NULL,
                status TEXT NOT NULL,
                sandbox_mode BOOLEAN NOT NULL,
                container_id TEXT,
                pid INTEGER,
                ports TEXT DEFAULT '[]',
                started_at TEXT NOT NULL,
                finished_at TEXT,
                FOREIGN KEY (project_id) REFERENCES projects(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Создание таблицы логов
        tracing::debug!("Создание таблицы logs");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS logs (
                id TEXT PRIMARY KEY,
                execution_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                level TEXT NOT NULL,
                message TEXT NOT NULL,
                FOREIGN KEY (execution_id) REFERENCES executions(id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Создание таблицы доверенных репозиториев
        tracing::debug!("Создание таблицы trusted_repositories");
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS trusted_repositories (
                id TEXT PRIMARY KEY,
                repo_url TEXT NOT NULL UNIQUE,
                added_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        tracing::info!("Миграции базы данных успешно выполнены");
        Ok(())
    }

    pub async fn save_project(&self, project: &Project) -> Result<()> {
        tracing::info!("Сохранение проекта: {} ({})", project.repo_name, project.id);
        
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO projects 
            (id, github_url, owner, repo_name, local_path, detected_stack, trust_level, created_at, last_run_at, tags)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&project.id)
        .bind(&project.github_url)
        .bind(&project.owner)
        .bind(&project.repo_name)
        .bind(&project.local_path)
        .bind(&project.detected_stack)
        .bind(&project.trust_level)
        .bind(&project.created_at)
        .bind(&project.last_run_at)
        .bind(&project.tags)
        .execute(&self.pool)
        .await?;

        tracing::debug!("Проект {} успешно сохранен", project.id);
        Ok(())
    }

    pub async fn get_project(&self, id: &str) -> Result<Option<Project>> {
        let project = sqlx::query_as::<_, Project>(
            "SELECT * FROM projects WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(project)
    }

    pub async fn get_all_projects(&self) -> Result<Vec<Project>> {
        let projects = sqlx::query_as::<_, Project>(
            "SELECT * FROM projects ORDER BY last_run_at DESC, created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(projects)
    }

    pub async fn search_projects(&self, query: &str) -> Result<Vec<Project>> {
        let search_pattern = format!("%{}%", query);
        let projects = sqlx::query_as::<_, Project>(
            r#"
            SELECT * FROM projects 
            WHERE repo_name LIKE ? OR owner LIKE ? OR tags LIKE ?
            ORDER BY last_run_at DESC, created_at DESC
            "#
        )
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .fetch_all(&self.pool)
        .await?;

        Ok(projects)
    }

    pub async fn delete_project(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM projects WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // Методы для работы с доверенными репозиториями
    
    pub async fn add_trusted_repository(&self, repo_url: &str) -> Result<()> {
        let id = uuid::Uuid::new_v4().to_string();
        let added_at = chrono::Utc::now().to_rfc3339();
        
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO trusted_repositories (id, repo_url, added_at)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(repo_url)
        .bind(&added_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn remove_trusted_repository(&self, repo_url: &str) -> Result<()> {
        sqlx::query("DELETE FROM trusted_repositories WHERE repo_url = ?")
            .bind(repo_url)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn is_trusted_repository(&self, repo_url: &str) -> Result<bool> {
        let result = sqlx::query("SELECT COUNT(*) as count FROM trusted_repositories WHERE repo_url = ?")
            .bind(repo_url)
            .fetch_one(&self.pool)
            .await?;

        let count: i64 = result.get("count");
        Ok(count > 0)
    }

    pub async fn get_trusted_repositories(&self) -> Result<Vec<String>> {
        let rows = sqlx::query("SELECT repo_url FROM trusted_repositories ORDER BY added_at DESC")
            .fetch_all(&self.pool)
            .await?;

        let repos = rows.iter()
            .map(|row| row.get("repo_url"))
            .collect();

        Ok(repos)
    }

    // Методы для работы со снимками проектов (Требование 7)

    pub async fn save_snapshot(&self, snapshot: &ProjectSnapshot) -> Result<()> {
        tracing::info!("Сохранение снимка: {} для проекта {}", snapshot.id, snapshot.project_id);
        
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO snapshots 
            (id, project_id, snapshot_path, environment_type, metadata, created_at, size_bytes)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&snapshot.id)
        .bind(&snapshot.project_id)
        .bind(&snapshot.snapshot_path)
        .bind(&snapshot.environment_type)
        .bind(&snapshot.metadata)
        .bind(&snapshot.created_at)
        .bind(snapshot.size_bytes)
        .execute(&self.pool)
        .await?;

        tracing::debug!("Снимок {} успешно сохранен", snapshot.id);
        Ok(())
    }

    pub async fn get_snapshot(&self, id: &str) -> Result<Option<ProjectSnapshot>> {
        let snapshot = sqlx::query_as::<_, ProjectSnapshot>(
            "SELECT * FROM snapshots WHERE id = ?"
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(snapshot)
    }

    pub async fn get_snapshots_for_project(&self, project_id: &str) -> Result<Vec<ProjectSnapshot>> {
        let snapshots = sqlx::query_as::<_, ProjectSnapshot>(
            "SELECT * FROM snapshots WHERE project_id = ? ORDER BY created_at DESC"
        )
        .bind(project_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(snapshots)
    }

    pub async fn get_all_snapshots(&self) -> Result<Vec<ProjectSnapshot>> {
        let snapshots = sqlx::query_as::<_, ProjectSnapshot>(
            "SELECT * FROM snapshots ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(snapshots)
    }

    pub async fn delete_snapshot(&self, id: &str) -> Result<()> {
        tracing::info!("Удаление снимка из БД: {}", id);
        
        sqlx::query("DELETE FROM snapshots WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        tracing::debug!("Снимок {} успешно удален из БД", id);
        Ok(())
    }
}

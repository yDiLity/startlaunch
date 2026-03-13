# База данных AutoLaunch

## Обзор

AutoLaunch использует SQLite для хранения метаданных проектов, снимков, выполнений и логов. База данных автоматически создается при первом запуске приложения.

## Расположение

База данных хранится в директории конфигурации пользователя:

- **Windows**: `C:\Users\<username>\AppData\Roaming\autolaunch\autolaunch.db`
- **macOS**: `~/Library/Application Support/autolaunch/autolaunch.db`
- **Linux**: `~/.config/autolaunch/autolaunch.db`

## Схема базы данных

### Таблица: projects

Хранит информацию о проанализированных проектах.

```sql
CREATE TABLE projects (
    id TEXT PRIMARY KEY,              -- UUID проекта
    github_url TEXT NOT NULL,         -- Полный URL репозитория
    owner TEXT NOT NULL,              -- Владелец репозитория
    repo_name TEXT NOT NULL,          -- Имя репозитория
    local_path TEXT NOT NULL,         -- Путь к локальной копии
    detected_stack TEXT NOT NULL,     -- Обнаруженный стек технологий
    trust_level TEXT NOT NULL,        -- Уровень доверия (Unknown/Trusted/Untrusted)
    created_at TEXT NOT NULL,         -- Дата создания (ISO 8601)
    last_run_at TEXT,                 -- Дата последнего запуска (ISO 8601)
    tags TEXT DEFAULT '[]'            -- Теги проекта (JSON array)
);
```

**Пример записи:**
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "github_url": "https://github.com/facebook/react",
  "owner": "facebook",
  "repo_name": "react",
  "local_path": "/tmp/autolaunch/facebook_react",
  "detected_stack": "NodeJs(18)",
  "trust_level": "Unknown",
  "created_at": "2024-01-15T10:30:00Z",
  "last_run_at": "2024-01-15T10:35:00Z",
  "tags": "[\"frontend\", \"library\"]"
}
```

### Таблица: snapshots

Хранит информацию о сохраненных снимках проектов.

```sql
CREATE TABLE snapshots (
    id TEXT PRIMARY KEY,              -- UUID снимка
    project_id TEXT NOT NULL,         -- Ссылка на проект
    snapshot_path TEXT NOT NULL,      -- Путь к снимку
    environment_type TEXT NOT NULL,   -- Тип окружения (Docker/Direct)
    metadata TEXT NOT NULL,           -- Метаданные снимка (JSON)
    created_at TEXT NOT NULL,         -- Дата создания (ISO 8601)
    size_bytes INTEGER NOT NULL,      -- Размер снимка в байтах
    FOREIGN KEY (project_id) REFERENCES projects(id)
);
```

**Пример записи:**
```json
{
  "id": "660e8400-e29b-41d4-a716-446655440001",
  "project_id": "550e8400-e29b-41d4-a716-446655440000",
  "snapshot_path": "/home/user/.local/share/autolaunch/snapshots/react-snapshot-1",
  "environment_type": "Docker",
  "metadata": "{\"ports\":[3000],\"env_vars\":{\"NODE_ENV\":\"development\"}}",
  "created_at": "2024-01-15T10:40:00Z",
  "size_bytes": 524288000
}
```

### Таблица: executions

Хранит информацию о запусках проектов.

```sql
CREATE TABLE executions (
    id TEXT PRIMARY KEY,              -- UUID выполнения
    project_id TEXT NOT NULL,         -- Ссылка на проект
    status TEXT NOT NULL,             -- Статус (Preparing/Installing/Starting/Running/Stopped/Failed)
    sandbox_mode BOOLEAN NOT NULL,    -- Режим изоляции (true=Docker, false=Direct)
    container_id TEXT,                -- ID Docker контейнера (если используется)
    pid INTEGER,                      -- ID процесса (если используется прямой режим)
    ports TEXT DEFAULT '[]',          -- Используемые порты (JSON array)
    started_at TEXT NOT NULL,         -- Время начала (ISO 8601)
    finished_at TEXT,                 -- Время завершения (ISO 8601)
    FOREIGN KEY (project_id) REFERENCES projects(id)
);
```

**Пример записи:**
```json
{
  "id": "770e8400-e29b-41d4-a716-446655440002",
  "project_id": "550e8400-e29b-41d4-a716-446655440000",
  "status": "Running",
  "sandbox_mode": true,
  "container_id": "abc123def456",
  "pid": null,
  "ports": "[3000, 8080]",
  "started_at": "2024-01-15T10:35:00Z",
  "finished_at": null
}
```

### Таблица: logs

Хранит логи выполнения проектов.

```sql
CREATE TABLE logs (
    id TEXT PRIMARY KEY,              -- UUID лога
    execution_id TEXT NOT NULL,       -- Ссылка на выполнение
    timestamp TEXT NOT NULL,          -- Время лога (ISO 8601)
    level TEXT NOT NULL,              -- Уровень (INFO/WARN/ERROR/DEBUG)
    message TEXT NOT NULL,            -- Сообщение лога
    FOREIGN KEY (execution_id) REFERENCES executions(id)
);
```

**Пример записи:**
```json
{
  "id": "880e8400-e29b-41d4-a716-446655440003",
  "execution_id": "770e8400-e29b-41d4-a716-446655440002",
  "timestamp": "2024-01-15T10:35:05Z",
  "level": "INFO",
  "message": "Server started on port 3000"
}
```

## Миграции

Миграции выполняются автоматически при запуске приложения через метод `Database::run_migrations()`.

Текущая версия схемы: **v1.0**

### Процесс миграции

1. При запуске приложения вызывается `Database::new()`
2. Создается подключение к SQLite
3. Выполняется `run_migrations()`, который создает таблицы если их нет
4. Используется `CREATE TABLE IF NOT EXISTS` для безопасного создания

### Будущие миграции

Для добавления новых миграций:

1. Создать новый SQL скрипт в `run_migrations()`
2. Использовать версионирование через отдельную таблицу `schema_version`
3. Применять миграции последовательно

## Операции с базой данных

### Сохранение проекта

```rust
let project = Project {
    id: Uuid::new_v4().to_string(),
    github_url: "https://github.com/owner/repo".to_string(),
    owner: "owner".to_string(),
    repo_name: "repo".to_string(),
    local_path: "/tmp/project".to_string(),
    detected_stack: "NodeJs(18)".to_string(),
    trust_level: "Unknown".to_string(),
    created_at: Utc::now().to_rfc3339(),
    last_run_at: None,
    tags: "[]".to_string(),
};

db.save_project(&project).await?;
```

### Получение проекта

```rust
// По ID
let project = db.get_project("550e8400-e29b-41d4-a716-446655440000").await?;

// Все проекты
let projects = db.get_all_projects().await?;

// Поиск
let results = db.search_projects("react").await?;
```

### Удаление проекта

```rust
db.delete_project("550e8400-e29b-41d4-a716-446655440000").await?;
```

## Индексы

Для оптимизации производительности рекомендуется добавить индексы:

```sql
-- Индекс для поиска по имени репозитория
CREATE INDEX idx_projects_repo_name ON projects(repo_name);

-- Индекс для сортировки по дате последнего запуска
CREATE INDEX idx_projects_last_run ON projects(last_run_at DESC);

-- Индекс для связи снимков с проектами
CREATE INDEX idx_snapshots_project_id ON snapshots(project_id);

-- Индекс для связи выполнений с проектами
CREATE INDEX idx_executions_project_id ON executions(project_id);

-- Индекс для связи логов с выполнениями
CREATE INDEX idx_logs_execution_id ON logs(execution_id);
```

## Резервное копирование

Для создания резервной копии базы данных:

```bash
# Linux/macOS
cp ~/.config/autolaunch/autolaunch.db ~/.config/autolaunch/autolaunch.db.backup

# Windows
copy "%APPDATA%\autolaunch\autolaunch.db" "%APPDATA%\autolaunch\autolaunch.db.backup"
```

## Очистка базы данных

Для полной очистки:

```bash
# Linux/macOS
rm ~/.config/autolaunch/autolaunch.db

# Windows
del "%APPDATA%\autolaunch\autolaunch.db"
```

База данных будет автоматически пересоздана при следующем запуске приложения.

## Ограничения

- **Размер базы данных**: SQLite поддерживает базы до 281 TB
- **Одновременные записи**: SQLite использует блокировки на уровне файла
- **Транзакции**: Все операции выполняются в транзакциях через sqlx

## Мониторинг

Для просмотра содержимого базы данных можно использовать:

- [DB Browser for SQLite](https://sqlitebrowser.org/)
- [SQLite CLI](https://sqlite.org/cli.html)
- [DBeaver](https://dbeaver.io/)

```bash
# Через SQLite CLI
sqlite3 ~/.config/autolaunch/autolaunch.db

# Просмотр таблиц
.tables

# Просмотр схемы
.schema projects

# Запрос данных
SELECT * FROM projects;
```

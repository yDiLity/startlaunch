# Менеджер проектов - Документация

## Обзор

Менеджер проектов предоставляет функциональность для управления историей запущенных проектов, включая поиск, фильтрацию и организацию с помощью тегов.

## Реализованные требования

### Требование 8.1: Добавление в историю
При запуске проекта система автоматически добавляет его в историю с временной меткой.

**Реализация:**
- Функция `save_project` в `database.rs` сохраняет проект в БД
- Поле `created_at` фиксирует время создания записи
- Поле `last_run_at` обновляется при каждом запуске

### Требование 8.2: Отображение списка проектов
Менеджер проектов показывает все проекты с датами последнего запуска.

**Реализация:**
- Команда `get_project_history` возвращает все проекты
- Проекты отсортированы по дате последнего запуска (новые первыми)
- UI компонент `ProjectManager.tsx` отображает список с датами

### Требование 8.3: Поиск и фильтрация
Система поддерживает поиск проектов по имени, владельцу и тегам.

**Реализация:**
- Команда `search_projects_by_query` выполняет поиск по базе данных
- Поиск работает по полям: `repo_name`, `owner`, `tags`
- Поиск регистронезависимый (LIKE с %)
- UI предоставляет поле поиска с мгновенной фильтрацией

### Требование 8.4: Система тегов
Пользователи могут добавлять теги к проектам для организации.

**Реализация:**
- Поле `tags` в модели `Project` хранит JSON массив тегов
- Команда `update_project_tags` обновляет теги проекта
- Команда `get_project_tags` получает теги конкретного проекта
- Команда `get_all_tags` возвращает все уникальные теги
- Команда `filter_projects_by_tags` фильтрует проекты по тегам
- UI позволяет добавлять/удалять теги в режиме редактирования

### Требование 8.5: Быстрый перезапуск
При нажатии на проект в истории система предлагает быстрый перезапуск.

**Реализация:**
- Кнопка "Запустить" в UI для каждого проекта
- Команда `update_project_last_run` обновляет время последнего запуска
- Интеграция с `start_project` для запуска проекта

## API команды

### Backend команды (Rust)

#### `get_project_history`
Получает список всех проектов, отсортированных по дате последнего запуска.

```rust
#[tauri::command]
pub async fn get_project_history(
    state: State<'_, AppState>,
) -> std::result::Result<Vec<Project>, ErrorContext>
```

**Возвращает:** Массив проектов с полной информацией

#### `search_projects_by_query`
Выполняет поиск проектов по запросу.

```rust
#[tauri::command]
pub async fn search_projects_by_query(
    query: String,
    state: State<'_, AppState>,
) -> std::result::Result<Vec<Project>, ErrorContext>
```

**Параметры:**
- `query` - поисковый запрос (имя, владелец, теги)

**Возвращает:** Массив найденных проектов

#### `update_project_tags`
Обновляет теги проекта.

```rust
#[tauri::command]
pub async fn update_project_tags(
    project_id: String,
    tags: Vec<String>,
    state: State<'_, AppState>,
) -> std::result::Result<(), ErrorContext>
```

**Параметры:**
- `project_id` - ID проекта
- `tags` - новый массив тегов

#### `get_project_tags`
Получает теги конкретного проекта.

```rust
#[tauri::command]
pub async fn get_project_tags(
    project_id: String,
    state: State<'_, AppState>,
) -> std::result::Result<Vec<String>, ErrorContext>
```

**Параметры:**
- `project_id` - ID проекта

**Возвращает:** Массив тегов

#### `get_all_tags`
Получает все уникальные теги из всех проектов.

```rust
#[tauri::command]
pub async fn get_all_tags(
    state: State<'_, AppState>,
) -> std::result::Result<Vec<String>, ErrorContext>
```

**Возвращает:** Отсортированный массив уникальных тегов

#### `filter_projects_by_tags`
Фильтрует проекты по указанным тегам.

```rust
#[tauri::command]
pub async fn filter_projects_by_tags(
    tags: Vec<String>,
    state: State<'_, AppState>,
) -> std::result::Result<Vec<Project>, ErrorContext>
```

**Параметры:**
- `tags` - массив тегов для фильтрации

**Возвращает:** Массив проектов, содержащих хотя бы один из указанных тегов

#### `delete_project`
Удаляет проект из истории.

```rust
#[tauri::command]
pub async fn delete_project(
    project_id: String,
    state: State<'_, AppState>,
) -> std::result::Result<(), ErrorContext>
```

**Параметры:**
- `project_id` - ID проекта для удаления

**Примечание:** Также удаляет локальные файлы и снимки проекта

#### `update_project_last_run`
Обновляет время последнего запуска проекта.

```rust
#[tauri::command]
pub async fn update_project_last_run(
    project_id: String,
    state: State<'_, AppState>,
) -> std::result::Result<(), ErrorContext>
```

**Параметры:**
- `project_id` - ID проекта

## UI компонент

### ProjectManager.tsx

React компонент для управления проектами.

**Основные возможности:**
- Отображение списка проектов с карточками
- Поиск в реальном времени
- Фильтрация по тегам
- Редактирование тегов проекта
- Запуск проекта
- Удаление проекта

**Props:**
```typescript
interface ProjectManagerProps {
  onClose: () => void;
  onLaunchProject: (projectId: string) => void;
}
```

**Использование:**
```tsx
<ProjectManager
  onClose={() => setShowManager(false)}
  onLaunchProject={(id) => handleLaunch(id)}
/>
```

## Структура данных

### Project
```rust
pub struct Project {
    pub id: String,              // UUID проекта
    pub github_url: String,      // URL репозитория
    pub owner: String,           // Владелец репозитория
    pub repo_name: String,       // Имя репозитория
    pub local_path: String,      // Локальный путь
    pub detected_stack: String,  // Обнаруженный стек
    pub trust_level: String,     // Уровень доверия
    pub created_at: String,      // Дата создания (RFC3339)
    pub last_run_at: Option<String>, // Дата последнего запуска (RFC3339)
    pub tags: String,            // JSON массив тегов
}
```

## База данных

### Таблица projects
```sql
CREATE TABLE projects (
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
);
```

**Индексы:**
- PRIMARY KEY на `id`
- Поиск оптимизирован через LIKE запросы

## Тестирование

### Модульные тесты
Файл: `src-tauri/src/project_manager_test.rs`

**Покрытие:**
- ✅ Добавление проекта в историю
- ✅ Получение списка проектов с датами
- ✅ Поиск по имени репозитория
- ✅ Поиск по владельцу
- ✅ Сохранение и получение тегов
- ✅ Обновление тегов
- ✅ Поиск по тегам
- ✅ Обновление времени последнего запуска
- ✅ Удаление проекта
- ✅ Регистронезависимый поиск

**Запуск тестов:**
```bash
cd src-tauri
cargo test project_manager_test
```

## Примеры использования

### Получение истории проектов
```typescript
const projects = await invoke<Project[]>("get_project_history");
console.log(`Найдено ${projects.length} проектов`);
```

### Поиск проектов
```typescript
const results = await invoke<Project[]>("search_projects_by_query", {
  query: "react"
});
```

### Добавление тегов
```typescript
await invoke("update_project_tags", {
  project_id: "project-uuid",
  tags: ["frontend", "react", "typescript"]
});
```

### Фильтрация по тегам
```typescript
const frontendProjects = await invoke<Project[]>("filter_projects_by_tags", {
  tags: ["frontend"]
});
```

### Запуск проекта из истории
```typescript
// Обновляем время последнего запуска
await invoke("update_project_last_run", { project_id: projectId });

// Запускаем проект
await invoke("start_project", { project_id: projectId });
```

## Производительность

### Оптимизации
- Использование индексов в SQLite для быстрого поиска
- Сортировка на уровне БД (ORDER BY)
- Кэширование списка тегов в UI
- Дебаунсинг поискового запроса в UI

### Рекомендации
- Для больших объемов данных (>1000 проектов) рекомендуется добавить пагинацию
- Регулярная очистка старых неиспользуемых проектов
- Использование индексов для полей `repo_name`, `owner` при росте БД

## Обработка ошибок

Все команды возвращают `ErrorContext` при ошибках:

```typescript
try {
  await invoke("update_project_tags", { project_id, tags });
} catch (err: any) {
  console.error(err.user_friendly_message);
  // Показать пользователю понятное сообщение об ошибке
}
```

## Будущие улучшения

- [ ] Пагинация для больших списков проектов
- [ ] Сортировка по различным полям (имя, дата, стек)
- [ ] Экспорт/импорт истории проектов
- [ ] Группировка проектов по владельцу или стеку
- [ ] Избранные проекты (закрепление)
- [ ] Статистика использования проектов

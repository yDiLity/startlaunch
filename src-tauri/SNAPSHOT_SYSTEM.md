# Система снимков проектов

## Обзор

Система снимков проектов реализует функциональность сохранения и быстрого восстановления состояния проектов (Требование 7). Снимки позволяют пользователям сохранять настроенные проекты со всеми зависимостями и конфигурацией для быстрого перезапуска без повторной установки.

## Архитектура

### Компоненты

1. **SnapshotManager** (`src/snapshot_manager.rs`)
   - Управление жизненным циклом снимков
   - Сериализация и десериализация состояния проектов
   - Копирование файлов проекта с исключениями
   - Очистка старых снимков

2. **Database** (`src/database.rs`)
   - Хранение метаданных снимков в SQLite
   - Связь снимков с проектами
   - Запросы для получения снимков

3. **Commands** (`src/commands.rs`)
   - Tauri команды для работы со снимками из фронтенда
   - Интеграция с другими компонентами системы

### Модели данных

```rust
pub struct ProjectSnapshot {
    pub id: String,              // UUID снимка
    pub project_id: String,      // ID связанного проекта
    pub snapshot_path: String,   // Путь к директории снимка
    pub environment_type: String, // "docker" или "direct"
    pub metadata: String,        // JSON с метаданными
    pub created_at: String,      // Дата создания (RFC3339)
    pub size_bytes: i64,         // Размер снимка в байтах
}

pub struct SnapshotMetadata {
    pub entry_command: Option<String>,           // Команда запуска
    pub ports: Vec<u16>,                         // Используемые порты
    pub environment_variables: Vec<(String, String)>, // Переменные окружения
    pub dependencies: Vec<Dependency>,           // Зависимости проекта
    pub tech_stack: String,                      // Стек технологий
}
```

## Функциональность

### 1. Создание снимка (Требование 7.2)

**Команда:** `save_project_snapshot(project_id: String)`

**Процесс:**
1. Получение информации о проекте из БД
2. Анализ текущего состояния проекта
3. Копирование файлов проекта (с исключениями)
4. Сохранение метаданных (команды, порты, переменные окружения)
5. Вычисление размера снимка
6. Сохранение записи в БД

**Исключаемые директории:**
- `.git` - история Git
- `node_modules` - зависимости Node.js
- `target` - артефакты сборки Rust
- `__pycache__` - кэш Python
- `.venv`, `venv` - виртуальные окружения Python
- `dist`, `build` - директории сборки
- `.cache` - кэш-файлы

**Пример использования:**
```javascript
const snapshotId = await invoke('save_project_snapshot', { 
    projectId: 'project-uuid-here' 
});
console.log('Снимок создан:', snapshotId);
```

### 2. Загрузка снимка (Требование 7.3)

**Команда:** `load_project_snapshot(snapshot_id: String)`

**Процесс:**
1. Получение информации о снимке из БД
2. Загрузка метаданных из файла
3. Создание окружения из снимка
4. Запуск проекта с сохраненными параметрами
5. Обновление времени последнего запуска

**Требование производительности:** Запуск из снимка должен занимать менее 10 секунд.

**Пример использования:**
```javascript
const message = await invoke('load_project_snapshot', { 
    snapshotId: 'snapshot-uuid-here' 
});
console.log(message); // "Проект запущен из снимка! ID процесса: ..."
```

### 3. Удаление снимка (Требование 7.5)

**Команда:** `delete_project_snapshot(snapshot_id: String)`

**Процесс:**
1. Удаление всех файлов снимка из файловой системы
2. Удаление записи из БД
3. Полная очистка без остатков

**Пример использования:**
```javascript
await invoke('delete_project_snapshot', { 
    snapshotId: 'snapshot-uuid-here' 
});
console.log('Снимок удален');
```

### 4. Получение списка снимков

**Команда:** `get_project_snapshots(project_id: String)`

Возвращает все снимки для конкретного проекта, отсортированные по дате создания (новые первыми).

**Команда:** `get_all_snapshots()`

Возвращает все снимки в системе.

**Пример использования:**
```javascript
const snapshots = await invoke('get_project_snapshots', { 
    projectId: 'project-uuid-here' 
});
snapshots.forEach(snapshot => {
    console.log(`Снимок ${snapshot.id}: ${snapshot.size_bytes} байт`);
});
```

### 5. Очистка старых снимков

**Команда:** `cleanup_old_snapshots(max_age_days: u64)`

Удаляет снимки старше указанного количества дней.

**Пример использования:**
```javascript
const deleted = await invoke('cleanup_old_snapshots', { 
    maxAgeDays: 30 
});
console.log(`Удалено ${deleted.length} старых снимков`);
```

## Хранение данных

### Файловая система

Снимки хранятся в директории данных приложения:
- **Windows:** `C:\Users\<User>\AppData\Roaming\autolaunch\snapshots\`
- **macOS:** `~/Library/Application Support/autolaunch/snapshots/`
- **Linux:** `~/.local/share/autolaunch/snapshots/`

Структура директории снимка:
```
<snapshot-uuid>/
├── snapshot_metadata.json  # Метаданные снимка
├── package.json           # Файлы проекта
├── src/
│   ├── index.js
│   └── ...
└── ...
```

### База данных

Таблица `snapshots` в SQLite:
```sql
CREATE TABLE snapshots (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    snapshot_path TEXT NOT NULL,
    environment_type TEXT NOT NULL,
    metadata TEXT NOT NULL,
    created_at TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id)
);
```

## Тестирование

### Модульные тесты

Файл: `src/snapshot_manager_test.rs`

**Покрытие:**
- Создание снимка с сохранением файлов
- Загрузка снимка и проверка метаданных
- Удаление снимка с полной очисткой
- Исключение node_modules и других директорий
- Сохранение структуры директорий
- Множественные снимки для одного проекта
- Сериализация метаданных
- Очистка старых снимков

**Запуск тестов:**
```bash
cd src-tauri
cargo test snapshot_manager
```

### Интеграционные тесты

Тесты проверяют взаимодействие между компонентами:
- Создание снимка через команду Tauri
- Сохранение в БД и файловую систему
- Загрузка и запуск проекта из снимка
- Удаление с очисткой всех данных

## Требования и соответствие

### Требование 7.1: Предложение сохранить снимок
✅ Реализовано через команду `save_project_snapshot`

### Требование 7.2: Сохранение зависимостей и конфигурации
✅ Реализовано через `SnapshotMetadata` и копирование файлов

### Требование 7.3: Быстрый запуск (< 10 секунд)
✅ Реализовано через `load_project_snapshot` с предустановленным окружением

### Требование 7.4: Сохранение метаданных
✅ Реализовано через `SnapshotMetadata` (команды, порты, переменные окружения)

### Требование 7.5: Полная очистка при удалении
✅ Реализовано через `delete_snapshot` с удалением всех файлов и записей БД

## Ограничения и известные проблемы

1. **Размер снимков:** Большие проекты могут создавать снимки значительного размера. Рекомендуется периодическая очистка старых снимков.

2. **Зависимости:** Снимки не включают установленные зависимости (node_modules, target и т.д.). При первом запуске из снимка может потребоваться их переустановка.

3. **Переносимость:** Снимки привязаны к конкретной машине и могут не работать на других системах из-за различий в путях и окружении.

## Будущие улучшения

1. **Сжатие снимков:** Использование архивации для уменьшения размера
2. **Инкрементальные снимки:** Сохранение только изменений между снимками
3. **Экспорт/импорт:** Возможность переноса снимков между машинами
4. **Автоматические снимки:** Создание снимков по расписанию или при определенных событиях
5. **Теги и описания:** Добавление пользовательских меток к снимкам для лучшей организации

## API Reference

### Rust API

```rust
impl SnapshotManager {
    pub fn new() -> Result<Self>;
    
    pub async fn create_snapshot(
        &self,
        project_id: &str,
        project_path: &Path,
        project_info: &ProjectInfo,
        environment_type: EnvironmentType,
        ports: Vec<u16>,
        environment_variables: Vec<(String, String)>,
    ) -> Result<ProjectSnapshot>;
    
    pub async fn load_snapshot(&self, snapshot_id: &str) 
        -> Result<(PathBuf, SnapshotMetadata)>;
    
    pub async fn delete_snapshot(&self, snapshot_id: &str) -> Result<()>;
    
    pub async fn list_snapshots(&self, project_id: &str) -> Result<Vec<String>>;
    
    pub async fn cleanup_old_snapshots(&self, max_age_days: u64) 
        -> Result<Vec<String>>;
}
```

### Tauri Commands

```rust
#[tauri::command]
pub async fn save_project_snapshot(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<String, ErrorContext>;

#[tauri::command]
pub async fn load_project_snapshot(
    snapshot_id: String,
    state: State<'_, AppState>,
) -> Result<String, ErrorContext>;

#[tauri::command]
pub async fn delete_project_snapshot(
    snapshot_id: String,
    state: State<'_, AppState>,
) -> Result<(), ErrorContext>;

#[tauri::command]
pub async fn get_project_snapshots(
    project_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<ProjectSnapshot>, ErrorContext>;

#[tauri::command]
pub async fn get_all_snapshots(
    state: State<'_, AppState>,
) -> Result<Vec<ProjectSnapshot>, ErrorContext>;

#[tauri::command]
pub async fn cleanup_old_snapshots(
    max_age_days: u64,
    state: State<'_, AppState>,
) -> Result<Vec<String>, ErrorContext>;
```

## Заключение

Система снимков проектов полностью реализует Требование 7 из спецификации AutoLaunch. Она предоставляет пользователям удобный способ сохранения и быстрого восстановления состояния проектов, значительно ускоряя повторные запуски и улучшая пользовательский опыт.

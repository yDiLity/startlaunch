# System Patterns

## Высокоуровневая архитектура

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Web Frontend  │◄──►│   Tauri Bridge   │◄──►│  Rust Backend   │
│  React + TS     │    │     (IPC)        │    │   (Core Logic)  │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                                         │
                                          ┌──────────────┼──────────────┐
                                          ▼              ▼              ▼
                                    ┌──────────┐  ┌──────────┐  ┌──────────┐
                                    │  SQLite  │  │  Docker  │  │   FS     │
                                    │   (DB)   │  │   API    │  │ (Files)  │
                                    └──────────┘  └──────────┘  └──────────┘
```

## Модули Rust Backend

| Модуль                    | Файл                          | Назначение                              |
|---------------------------|-------------------------------|-----------------------------------------|
| commands                  | commands.rs                   | Tauri IPC команды (точки входа из UI)   |
| url_parser                | url_parser.rs                 | Парсинг и валидация GitHub URL          |
| project_analyzer          | project_analyzer.rs           | Детекция стека, извлечение команд       |
| environment_manager       | environment_manager.rs        | Docker/venv изоляция                    |
| process_controller        | process_controller.rs         | Lifecycle процессов (start/stop/restart)|
| security_scanner          | security_scanner.rs           | Статический анализ угроз                |
| snapshot_manager          | snapshot_manager.rs           | Сохранение/восстановление снимков       |
| settings_manager          | settings_manager.rs           | Конфигурация приложения                 |
| database                  | database.rs                   | SQLite через sqlx                       |
| models                    | models.rs                     | Общие структуры данных                  |
| error                     | error.rs                      | Иерархия ошибок (thiserror)             |

## Ключевые паттерны

### Trait-based абстракции
Каждый компонент определяет трейт + конкретную реализацию:
```rust
pub trait ProjectAnalyzer { ... }
pub trait EnvironmentManager { ... }
pub trait ProcessController { ... }
pub trait SecurityScanner { ... }
```

### Иерархия ошибок
```rust
pub enum AutoLaunchError {
    ProjectAnalysis(AnalysisError),
    Environment(EnvironmentError),
    Process(ProcessError),
    Security(SecurityError),
    Io(std::io::Error),
}
```

### IPC Flow (Frontend → Backend)
1. React вызывает `invoke('command_name', { args })` через `@tauri-apps/api`
2. Rust обрабатывает в `commands.rs`
3. Результат возвращается как `Result<T, String>`

### База данных
- SQLite через `sqlx` с async runtime
- Таблицы: `projects`, `snapshots`, `executions`, `logs`
- Хранение: `~/.autolaunch/autolaunch.db`

### Property-Based Testing
- Библиотека: `proptest`
- Минимум 100 итераций на тест
- Файлы: `*_property_test.rs` рядом с исходниками
- Комментарий: `**Feature: autolaunch-core, Property N: ...**`

## Хранение данных

```
~/.autolaunch/
├── autolaunch.db       # SQLite база
├── config.toml         # Настройки приложения
└── cache/              # Кэш зависимостей

~/Documents/AutoLaunch/Projects/  # Снимки проектов (настраиваемо)
```

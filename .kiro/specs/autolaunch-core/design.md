# Документ проектирования AutoLaunch

## Обзор

AutoLaunch представляет собой кроссплатформенное десктопное приложение, построенное на архитектуре Tauri (Rust backend + веб-фронтенд). Приложение автоматизирует процесс анализа, настройки и запуска проектов с GitHub, обеспечивая безопасность через изоляцию и удобство использования через интуитивный интерфейс.

Основной поток работы: пользователь вводит URL репозитория → система анализирует проект → создает изолированное окружение → устанавливает зависимости → запускает проект → предоставляет доступ через браузер или нативный интерфейс.

## Архитектура

### Высокоуровневая архитектура

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Web Frontend  │◄──►│   Tauri Bridge   │◄──►│  Rust Backend   │
│   (React/Vue)   │    │     (IPC)        │    │   (Core Logic)  │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                                         │
                                                         ▼
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Docker API    │◄──►│  Process Manager │◄──►│  File System    │
│   (Containers)  │    │   (Lifecycle)    │    │   (Projects)    │
└─────────────────┘    └──────────────────┘    └─────────────────┘
```

### Компонентная архитектура

**Frontend Layer (Web Technologies)**
- UI Components: Интерфейс пользователя с React/Vue
- State Management: Управление состоянием приложения
- IPC Client: Коммуникация с Rust backend через Tauri

**Backend Layer (Rust)**
- Project Analyzer: Анализ структуры и типа проекта
- Environment Manager: Управление изолированными окружениями
- Process Controller: Управление жизненным циклом процессов
- Security Scanner: Анализ безопасности кода
- Storage Manager: Управление локальными данными и снимками

**System Integration Layer**
- Docker Integration: Работа с Docker API для контейнеризации
- Git Integration: Клонирование и работа с репозиториями
- File System: Управление локальными файлами и кэшем
- Database: SQLite для хранения метаданных проектов

## Компоненты и интерфейсы

### 1. Project Analyzer

**Назначение:** Анализ структуры проекта и определение стека технологий

**Интерфейс:**
```rust
pub trait ProjectAnalyzer {
    fn analyze_project(&self, path: &Path) -> Result<ProjectInfo, AnalysisError>;
    fn detect_stack(&self, files: &[PathBuf]) -> TechStack;
    fn find_entry_point(&self, project_info: &ProjectInfo) -> Option<String>;
    fn parse_dependencies(&self, project_info: &ProjectInfo) -> Vec<Dependency>;
}

pub struct ProjectInfo {
    pub stack: TechStack,
    pub entry_command: Option<String>,
    pub dependencies: Vec<Dependency>,
    pub config_files: Vec<ConfigFile>,
    pub security_warnings: Vec<SecurityWarning>,
}
```

### 2. Environment Manager

**Назначение:** Создание и управление изолированными окружениями

**Интерфейс:**
```rust
pub trait EnvironmentManager {
    fn create_environment(&self, project: &ProjectInfo, mode: IsolationMode) -> Result<Environment, EnvError>;
    fn install_dependencies(&self, env: &Environment, deps: &[Dependency]) -> Result<(), EnvError>;
    fn cleanup_environment(&self, env: &Environment) -> Result<(), EnvError>;
}

pub enum IsolationMode {
    Sandbox(DockerConfig),
    Direct(VirtualEnvConfig),
}

pub struct Environment {
    pub id: String,
    pub mode: IsolationMode,
    pub working_dir: PathBuf,
    pub container_id: Option<String>,
}
```

### 3. Process Controller

**Назначение:** Управление жизненным циклом запущенных процессов

**Интерфейс:**
```rust
pub trait ProcessController {
    fn start_process(&self, env: &Environment, command: &str) -> Result<ProcessHandle, ProcessError>;
    fn stop_process(&self, handle: &ProcessHandle) -> Result<(), ProcessError>;
    fn restart_process(&self, handle: &ProcessHandle) -> Result<(), ProcessError>;
    fn get_process_status(&self, handle: &ProcessHandle) -> ProcessStatus;
    fn get_process_logs(&self, handle: &ProcessHandle) -> Vec<LogEntry>;
}

pub struct ProcessHandle {
    pub id: String,
    pub pid: Option<u32>,
    pub container_id: Option<String>,
    pub ports: Vec<u16>,
}
```

### 4. Security Scanner

**Назначение:** Анализ потенциальных угроз безопасности

**Интерфейс:**
```rust
pub trait SecurityScanner {
    fn scan_project(&self, project: &ProjectInfo) -> Vec<SecurityWarning>;
    fn scan_command(&self, command: &str) -> Vec<SecurityWarning>;
    fn is_trusted_repository(&self, repo_url: &str) -> bool;
    fn add_trusted_repository(&self, repo_url: &str) -> Result<(), SecurityError>;
}

pub struct SecurityWarning {
    pub level: SecurityLevel,
    pub message: String,
    pub suggestion: Option<String>,
}
```

## Модели данных

### Основные сущности

```rust
// Проект
#[derive(Debug, Serialize, Deserialize)]
pub struct Project {
    pub id: Uuid,
    pub github_url: String,
    pub owner: String,
    pub repo_name: String,
    pub local_path: PathBuf,
    pub detected_stack: TechStack,
    pub trust_level: TrustLevel,
    pub created_at: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub tags: Vec<String>,
}

// Снимок проекта
#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectSnapshot {
    pub id: Uuid,
    pub project_id: Uuid,
    pub snapshot_path: PathBuf,
    pub environment_type: EnvironmentType,
    pub metadata: SnapshotMetadata,
    pub created_at: DateTime<Utc>,
    pub size_bytes: u64,
}

// Выполнение
#[derive(Debug, Serialize, Deserialize)]
pub struct Execution {
    pub id: Uuid,
    pub project_id: Uuid,
    pub status: ExecutionStatus,
    pub sandbox_mode: bool,
    pub container_id: Option<String>,
    pub pid: Option<u32>,
    pub ports: Vec<u16>,
    pub logs: Vec<LogEntry>,
    pub started_at: DateTime<Utc>,
    pub finished_at: Option<DateTime<Utc>>,
}

// Стек технологий
#[derive(Debug, Serialize, Deserialize)]
pub enum TechStack {
    NodeJs { version: Option<String> },
    Python { version: Option<String> },
    Rust { edition: Option<String> },
    Go { version: Option<String> },
    Java { version: Option<String> },
    Docker { compose: bool },
    Static { framework: Option<String> },
    Unknown,
}

// Статус выполнения
#[derive(Debug, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Preparing,
    Installing,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed { error: String },
}
```

### База данных (SQLite)

```sql
-- Таблица проектов
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
    tags TEXT -- JSON array
);

-- Таблица снимков
CREATE TABLE snapshots (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    snapshot_path TEXT NOT NULL,
    environment_type TEXT NOT NULL,
    metadata TEXT NOT NULL, -- JSON
    created_at TEXT NOT NULL,
    size_bytes INTEGER NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

-- Таблица выполнений
CREATE TABLE executions (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    status TEXT NOT NULL,
    sandbox_mode BOOLEAN NOT NULL,
    container_id TEXT,
    pid INTEGER,
    ports TEXT, -- JSON array
    started_at TEXT NOT NULL,
    finished_at TEXT,
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

-- Таблица логов
CREATE TABLE logs (
    id TEXT PRIMARY KEY,
    execution_id TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    level TEXT NOT NULL,
    message TEXT NOT NULL,
    FOREIGN KEY (execution_id) REFERENCES executions(id)
);
```

## Correctness Properties

*Свойство - это характеристика или поведение, которое должно выполняться во всех валидных выполнениях системы - по сути, формальное утверждение о том, что система должна делать. Свойства служат мостом между человекочитаемыми спецификациями и машинно-проверяемыми гарантиями корректности.*

Теперь проведу анализ критериев приемки для определения тестируемых свойств:
### Prope
rty Reflection

После анализа всех критериев приемки выявлены следующие избыточности:

**Объединяемые свойства:**
- Свойства 2.2-2.5 (детекция конкретных стеков) можно объединить в одно комплексное свойство детекции стека
- Свойства 3.2-3.3 (генерация конкретных команд) можно объединить в одно свойство генерации команд
- Свойства 7.2 и 7.4 (сохранение снимков) можно объединить в одно свойство сериализации снимков

**Исключаемые избыточные свойства:**
- Свойство детекции конкретных стеков поглощается общим свойством анализа проекта
- Отдельные свойства для каждого типа файлов конфигурации объединяются в универсальное свойство

### Correctness Properties

**Property 1: URL парсинг и нормализация**
*Для любого* валидного GitHub URL или формата owner/repo, система должна корректно извлечь владельца и имя репозитория, а также нормализовать входные данные к стандартному формату
**Validates: Requirements 1.1, 1.3**

**Property 2: Обработка невалидных входных данных**
*Для любого* невалидного URL или некорректного формата входных данных, система должна возвращать понятное сообщение об ошибке без сбоев
**Validates: Requirements 1.2**

**Property 3: Сохранение данных проекта**
*Для любого* успешно проанализированного проекта, информация о проекте должна быть сохранена в локальной базе данных с корректными метаданными
**Validates: Requirements 1.5**

**Property 4: Детекция стека технологий**
*Для любой* структуры файлов проекта, система должна корректно определить основной стек технологий на основе присутствующих конфигурационных файлов
**Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5**

**Property 5: Извлечение команд запуска**
*Для любого* конфигурационного файла (package.json, pyproject.toml и т.д.), система должна корректно извлечь команду запуска из соответствующих полей
**Validates: Requirements 3.1, 3.2, 3.3**

**Property 6: Режим безопасности по умолчанию**
*Для любого* неизвестного или недоверенного проекта, система должна автоматически выбрать режим песочницы для изоляции
**Validates: Requirements 4.1**

**Property 7: Конфигурация безопасности контейнеров**
*Для любого* создаваемого Docker контейнера, система должна применить ограничения безопасности (no-root, read-only FS, ограничение привилегий)
**Validates: Requirements 4.2**

**Property 8: Детекция угроз безопасности**
*Для любой* команды или скрипта, содержащего потенциально опасные операции, система должна обнаружить угрозу и показать предупреждение
**Validates: Requirements 4.3**

**Property 9: Сохранение статуса доверия**
*Для любого* проекта, которому пользователь предоставил доверие, этот статус должен сохраняться и применяться при последующих запусках
**Validates: Requirements 4.4**

**Property 10: Отображение прогресса операций**
*Для любой* выполняемой операции, система должна корректно обновлять прогресс-бар и отображать текущий этап
**Validates: Requirements 5.1**

**Property 11: Передача логов в реальном времени**
*Для любой* операции установки зависимостей, логи должны передаваться в UI в реальном времени без задержек
**Validates: Requirements 5.2**

**Property 12: Качество сообщений об ошибках**
*Для любой* возникающей ошибки, система должна генерировать понятное сообщение с описанием проблемы и предложениями решения
**Validates: Requirements 5.3**

**Property 13: Отображение статуса успеха**
*Для любой* успешно завершенной операции, система должна отображать статус "Готово" с соответствующим визуальным индикатором
**Validates: Requirements 5.5**

**Property 14: Корректное завершение процессов**
*Для любого* запущенного проекта, операция остановки должна корректно завершить все связанные процессы без зависших ресурсов
**Validates: Requirements 6.2**

**Property 15: Эквивалентность перезапуска**
*Для любого* запущенного проекта, операция перезапуска должна быть эквивалентна последовательности остановка + запуск
**Validates: Requirements 6.3**

**Property 16: Автоматическая очистка ресурсов**
*Для любого* остановленного проекта, все временные ресурсы (файлы, контейнеры) должны быть автоматически очищены
**Validates: Requirements 6.4**

**Property 17: Сериализация снимков проекта**
*Для любого* сохраняемого снимка проекта, все зависимости, конфигурация и метаданные должны быть корректно сериализованы и восстановимы
**Validates: Requirements 7.2, 7.4**

**Property 18: Полная очистка при удалении снимков**
*Для любого* удаляемого снимка проекта, все связанные файлы и директории должны быть полностью удалены без остатков
**Validates: Requirements 7.5**

**Property 19: Ведение истории проектов**
*Для любого* запускаемого проекта, информация о запуске должна корректно добавляться в историю с правильными временными метками
**Validates: Requirements 8.1**

**Property 20: Отображение списка проектов**
*Для любого* запроса списка проектов, система должна отображать все проекты с корректными датами последнего запуска и метаданными
**Validates: Requirements 8.2**

**Property 21: Фильтрация и поиск проектов**
*Для любого* поискового запроса, система должна возвращать только проекты, соответствующие критериям поиска
**Validates: Requirements 8.3**

**Property 22: Сохранение и восстановление тегов**
*Для любых* тегов, добавленных к проекту, они должны корректно сохраняться и восстанавливаться при последующих обращениях
**Validates: Requirements 8.4**

**Property 23: Применение настроек**
*Для любых* изменений в настройках приложения, новые значения должны немедленно применяться ко всем последующим операциям
**Validates: Requirements 9.2**

**Property 24: Автоматическая очистка по настройкам**
*Для любого* проекта при включенной настройке автоочистки, временные файлы должны автоматически удаляться после остановки
**Validates: Requirements 9.4**

**Property 25: Сериализация настроек**
*Для любых* сохраняемых настроек, они должны корректно записываться в конфигурационный файл и восстанавливаться при запуске
**Validates: Requirements 9.5**

**Property 26: Детекция портов приложений**
*Для любого* запускаемого веб-приложения, система должна корректно извлекать номер порта из логов или конфигурационных файлов
**Validates: Requirements 10.1, 10.3**

**Property 27: Обработка ошибок запуска**
*Для любого* приложения, которое не запускается на ожидаемом порту, система должна отображать информативное сообщение с инструкциями
**Validates: Requirements 10.4**

**Property 28: Независимость процессов от UI**
*Для любого* запущенного приложения, закрытие браузера или UI элементов не должно влиять на работу фонового процесса
**Validates: Requirements 10.5**

## Обработка ошибок

### Стратегия обработки ошибок

**Категории ошибок:**

1. **Пользовательские ошибки** (User Errors)
   - Невалидные URL репозиториев
   - Отсутствующие или поврежденные конфигурационные файлы
   - Недостаточные права доступа к файлам

2. **Системные ошибки** (System Errors)
   - Недоступность Docker
   - Ошибки сети при клонировании репозиториев
   - Нехватка дискового пространства

3. **Ошибки выполнения** (Runtime Errors)
   - Сбои при установке зависимостей
   - Ошибки компиляции проектов
   - Конфликты портов

**Принципы обработки:**

```rust
// Иерархия ошибок
#[derive(Debug, thiserror::Error)]
pub enum AutoLaunchError {
    #[error("Ошибка анализа проекта: {0}")]
    ProjectAnalysis(#[from] AnalysisError),
    
    #[error("Ошибка окружения: {0}")]
    Environment(#[from] EnvironmentError),
    
    #[error("Ошибка процесса: {0}")]
    Process(#[from] ProcessError),
    
    #[error("Ошибка безопасности: {0}")]
    Security(#[from] SecurityError),
    
    #[error("Ошибка ввода-вывода: {0}")]
    Io(#[from] std::io::Error),
}

// Контекст ошибки с предложениями решения
pub struct ErrorContext {
    pub error: AutoLaunchError,
    pub suggestion: Option<String>,
    pub recovery_actions: Vec<RecoveryAction>,
    pub user_friendly_message: String,
}
```

### Стратегии восстановления

1. **Автоматическое восстановление**
   - Повторные попытки сетевых операций
   - Fallback на альтернативные методы изоляции
   - Автоматическая очистка поврежденных ресурсов

2. **Пользовательское восстановление**
   - Предложение альтернативных команд запуска
   - Ручной ввод недостающих параметров
   - Выбор альтернативного режима изоляции

## Стратегия тестирования

### Двойной подход к тестированию

Система тестирования включает как модульные тесты, так и property-based тесты для обеспечения комплексного покрытия:

**Модульные тесты:**
- Проверяют конкретные примеры и граничные случаи
- Тестируют интеграционные точки между компонентами
- Фокусируются на специфических сценариях использования

**Property-based тесты:**
- Проверяют универсальные свойства на множестве входных данных
- Используют библиотеку `proptest` для Rust
- Каждый тест выполняется минимум 100 итераций для обеспечения надежности

### Конфигурация Property-Based Testing

**Используемая библиотека:** `proptest` для Rust

**Требования к тестам:**
- Минимум 100 итераций на каждый property-based тест
- Каждый тест помечается комментарием с ссылкой на соответствующее свойство корректности
- Формат комментария: `**Feature: autolaunch-core, Property {number}: {property_text}**`

**Пример конфигурации:**
```rust
use proptest::prelude::*;

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    // **Feature: autolaunch-core, Property 1: URL парсинг и нормализация**
    #[test]
    fn test_url_parsing_normalization(
        owner in "[a-zA-Z0-9_-]{1,39}",
        repo in "[a-zA-Z0-9_.-]{1,100}"
    ) {
        // Тест реализации
    }
}
```

### Стратегия модульного тестирования

**Покрытие модульными тестами:**
- Специфические примеры детекции стеков технологий
- Граничные случаи парсинга конфигурационных файлов
- Интеграционные тесты с Docker API
- Тесты обработки ошибок и восстановления

**Организация тестов:**
- Тесты размещаются рядом с исходным кодом с суффиксом `.test.rs`
- Интеграционные тесты в отдельной директории `tests/`
- Использование mock-объектов для внешних зависимостей
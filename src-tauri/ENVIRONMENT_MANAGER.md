# Environment Manager - Менеджер окружений

## Обзор

Модуль `environment_manager` реализует систему управления изолированными окружениями для безопасного запуска проектов. Поддерживает два режима изоляции: Docker контейнеры (Sandbox) и виртуальные окружения языков (Direct).

## Реализованные требования

### ✅ Требование 4.1: Режим песочницы по умолчанию
- Система автоматически выбирает режим Sandbox (Docker) если Docker доступен
- Для неизвестных проектов используется максимальная изоляция

### ✅ Требование 4.2: Ограничения безопасности Docker
Реализованы следующие ограничения:
- **no-root**: Контейнер запускается от пользователя с UID 1000 (не root)
- **read-only FS**: Файловая система контейнера монтируется в режиме только для чтения
- **no-new-privileges**: Запрещено повышение привилегий внутри контейнера
- **cap-drop ALL**: Удалены все Linux capabilities
- **tmpfs**: Временная файловая система для /tmp с ограничениями (100MB, noexec, nosuid)

### ✅ Требование 4.5: Fallback механизмы
- При недоступности Docker автоматически используется режим Direct
- Создаются виртуальные окружения для Python (venv) и Node.js (npm)
- Зависимости устанавливаются в изолированное окружение

## Архитектура

### Трейт EnvironmentManagerTrait

```rust
pub trait EnvironmentManagerTrait {
    fn create_environment(&self, project: &ProjectInfo, project_path: &Path) -> Future<Result<Environment>>;
    fn install_dependencies(&self, env: &Environment, deps: &[Dependency]) -> Future<Result<()>>;
    fn cleanup_environment(&self, env: &Environment) -> Future<Result<()>>;
}
```

### Режимы изоляции

#### Sandbox (Docker)
- Полная изоляция через Docker контейнеры
- Автоматическая генерация Dockerfile для разных стеков
- Ограничения безопасности на уровне контейнера
- Поддержка: Node.js, Python, Rust, Go, Java

#### Direct (Virtual Environment)
- Виртуальные окружения языков программирования
- Python: venv
- Node.js: npm install в локальной директории
- Используется когда Docker недоступен

## Структуры данных

### DockerConfig
```rust
pub struct DockerConfig {
    pub image: String,           // Базовый Docker образ
    pub working_dir: String,     // Рабочая директория в контейнере
    pub ports: Vec<u16>,         // Проброшенные порты
    pub volumes: Vec<(String, String)>,  // Монтируемые тома
    pub environment: Vec<(String, String)>,  // Переменные окружения
    pub read_only: bool,         // Флаг read-only FS
    pub no_root: bool,           // Флаг запуска без root
}
```

### Environment
```rust
pub struct Environment {
    pub id: String,              // Уникальный ID окружения
    pub mode: IsolationMode,     // Режим изоляции
    pub working_dir: PathBuf,    // Рабочая директория
    pub container_id: Option<String>,  // ID Docker контейнера (если есть)
}
```

## Использование

### Создание окружения

```rust
let manager = EnvironmentManager::new();
let project_info = ProjectInfo { /* ... */ };
let project_path = Path::new("/path/to/project");

let env = manager.create_environment(&project_info, project_path).await?;
```

### Установка зависимостей

```rust
let dependencies = vec![
    Dependency { name: "express".to_string(), version: Some("4.18.0".to_string()), dev: false },
];

manager.install_dependencies(&env, &dependencies).await?;
```

### Очистка окружения

```rust
manager.cleanup_environment(&env).await?;
```

## Поддерживаемые стеки технологий

| Стек | Docker образ | Порты по умолчанию | Особенности |
|------|--------------|-------------------|-------------|
| Node.js | node:{version}-alpine | 3000, 8000, 8080 | npm install |
| Python | python:{version}-alpine | 5000, 8000, 8080 | pip install |
| Rust | rust:alpine | 8000, 8080 | cargo build |
| Go | golang:alpine | 8000, 8080 | go build |
| Unknown | alpine:latest | 8000, 8080 | Базовый shell |

## Безопасность

### Docker Security Options
```bash
--security-opt no-new-privileges  # Запрет повышения привилегий
--cap-drop ALL                    # Удаление всех capabilities
--user 1000:1000                  # Запуск от непривилегированного пользователя
--read-only                       # Read-only файловая система
--tmpfs /tmp:rw,noexec,nosuid,size=100m  # Временная FS с ограничениями
```

### Volume Mounting
- В режиме read-only все volumes монтируются с флагом `:ro`
- Исходный код проекта доступен только для чтения
- Запись возможна только в tmpfs

## Тестирование

Модуль покрыт модульными тестами:
- Проверка генерации Docker конфигурации
- Проверка наличия флагов безопасности
- Проверка генерации Dockerfile для разных стеков
- Проверка структур данных

Запуск тестов:
```bash
cargo test environment_manager
```

## Ограничения и известные проблемы

1. **Docker требуется для полной изоляции**: Без Docker используется менее безопасный режим Direct
2. **Read-only FS может ломать некоторые приложения**: Приложения, которые пишут в свою директорию, могут не работать
3. **Порты определяются статически**: Нет автоматического определения нужных портов из конфигурации проекта

## Будущие улучшения

- [ ] Автоматическое определение портов из конфигурационных файлов
- [ ] Поддержка Docker Compose проектов
- [ ] Кэширование Docker образов для ускорения запуска
- [ ] Мониторинг ресурсов контейнера (CPU, память)
- [ ] Поддержка других механизмов изоляции (Podman, LXC)

# ProcessController - Контроллер процессов

## Обзор

ProcessController отвечает за управление жизненным циклом запущенных процессов проектов. Модуль реализует требования 6.1-6.5 спецификации AutoLaunch.

## Основные возможности

### 1. Запуск процессов (Требование 6.1)
- Запуск процессов в Docker контейнерах (режим песочницы)
- Запуск процессов в прямом режиме (виртуальное окружение)
- Автоматическое определение портов приложения
- Мониторинг статуса процесса в реальном времени

### 2. Остановка процессов (Требование 6.2)
- Корректное завершение процессов с использованием SIGTERM (Unix)
- Принудительное завершение при необходимости
- Остановка Docker контейнеров с таймаутом
- Автоматическая очистка ресурсов после остановки

### 3. Перезапуск процессов (Требование 6.3)
- Перезапуск = остановка + запуск
- Сохранение команды запуска для корректного перезапуска
- Задержка между остановкой и запуском для корректного завершения

### 4. Автоматическая очистка ресурсов (Требование 6.4)
- Удаление Docker контейнеров после остановки
- Очистка временных файлов
- Освобождение портов

### 5. Управление всеми процессами (Требование 6.5)
- Остановка всех запущенных проектов
- Получение списка запущенных процессов
- Проверка наличия запущенных процессов

## API

### Основные методы

```rust
// Создание контроллера
let controller = ProcessController::new();

// Запуск процесса
let handle = controller.start_process(&environment, "npm start").await?;

// Остановка процесса
controller.stop_process(&handle).await?;

// Перезапуск процесса
controller.restart_process(&handle).await?;

// Получение статуса
let status = controller.get_process_status(&handle).await?;

// Получение логов
let logs = controller.get_process_logs(&handle).await?;

// Определение порта приложения
let port = controller.detect_application_port(&handle).await?;

// Остановка всех процессов
let stopped_ids = controller.stop_all_processes().await?;

// Получение списка запущенных процессов
let running = controller.get_running_processes();

// Проверка наличия запущенных процессов
let has_running = controller.has_running_processes();
```

## Структуры данных

### ProcessHandle
```rust
pub struct ProcessHandle {
    pub id: String,              // Уникальный ID процесса
    pub pid: Option<u32>,        // PID процесса (для прямого режима)
    pub container_id: Option<String>, // ID контейнера (для Docker)
    pub ports: Vec<u16>,         // Порты приложения
}
```

### ExecutionStatus
```rust
pub enum ExecutionStatus {
    Preparing,                   // Подготовка к запуску
    Installing,                  // Установка зависимостей
    Starting,                    // Запуск процесса
    Running,                     // Процесс работает
    Stopping,                    // Остановка процесса
    Stopped,                     // Процесс остановлен
    Failed { error: String },    // Ошибка выполнения
}
```

## Определение портов

ProcessController автоматически определяет порты приложения несколькими способами:

### 1. Из команды запуска
```bash
npm start --port 3000
python app.py --port=8080
node server.js -p 5000
```

### 2. Из логов приложения
Поддерживаемые паттерны:
- "listening on port 3000"
- "server running on port 8080"
- "localhost:5000"
- "127.0.0.1:8000"
- "0.0.0.0:3000"

### 3. Стандартные порты по умолчанию
- Node.js: 3000
- Python: 5000
- Другие: 8000

## Мониторинг процессов

ProcessController автоматически мониторит статус запущенных процессов:

- Проверка каждую секунду
- Обновление статуса (Starting → Running)
- Определение завершения процесса
- Сбор логов в реальном времени (до 1000 записей)

## Безопасность

### Docker режим
- Контейнеры запускаются с ограничениями безопасности
- Таймаут при остановке (10 секунд)
- Принудительное удаление контейнеров

### Прямой режим
- Корректное завершение через SIGTERM (Unix)
- Принудительное завершение при необходимости
- Очистка временных файлов

## Обработка ошибок

Все методы возвращают `Result<T, AutoLaunchError>`:

```rust
match controller.start_process(&env, command).await {
    Ok(handle) => println!("Процесс запущен: {}", handle.id),
    Err(AutoLaunchError::Process(msg)) => eprintln!("Ошибка: {}", msg),
    Err(e) => eprintln!("Неожиданная ошибка: {}", e),
}
```

## Интеграция с Tauri

ProcessController интегрирован с Tauri через команды:

```typescript
// Перезапуск проекта
await invoke('restart_project', { projectId: 'project-123' });

// Получение логов
const logs = await invoke('get_process_logs', { projectId: 'project-123' });

// Остановка всех проектов
const stopped = await invoke('stop_all_projects');

// Проверка запущенных проектов
const hasRunning = await invoke('has_running_projects');
```

## Тестирование

Модуль включает unit-тесты для всех основных функций:

```bash
cargo test process_controller_test
```

Тесты покрывают:
- Создание контроллера
- Запуск и остановку процессов
- Определение портов из команд
- Извлечение портов из логов
- Получение логов процессов
- Остановку всех процессов
- Получение списка запущенных процессов

## Примеры использования

### Запуск Node.js приложения
```rust
let env = Environment {
    id: "node-app".to_string(),
    mode: IsolationMode::Direct(VirtualEnvConfig {
        working_dir: PathBuf::from("/path/to/project"),
        env_vars: vec![],
    }),
    working_dir: PathBuf::from("/path/to/project"),
    container_id: None,
};

let handle = controller.start_process(&env, "npm start").await?;
println!("Приложение запущено на портах: {:?}", handle.ports);
```

### Мониторинг статуса
```rust
loop {
    let status = controller.get_process_status(&handle).await?;
    match status {
        ExecutionStatus::Running => println!("Работает"),
        ExecutionStatus::Stopped => break,
        ExecutionStatus::Failed { error } => {
            eprintln!("Ошибка: {}", error);
            break;
        }
        _ => {}
    }
    tokio::time::sleep(Duration::from_secs(1)).await;
}
```

### Остановка при закрытии приложения
```rust
// При закрытии приложения
if controller.has_running_processes() {
    println!("Обнаружены запущенные процессы. Останавливаем...");
    let stopped = controller.stop_all_processes().await?;
    println!("Остановлено процессов: {}", stopped.len());
}
```

## Ограничения

1. **Логи**: Хранятся только последние 1000 записей на процесс
2. **Мониторинг**: Проверка статуса каждую секунду (может быть настроено)
3. **Таймаут остановки**: 10 секунд для Docker контейнеров
4. **Определение портов**: Работает для стандартных паттернов

## Будущие улучшения

- [ ] Потоковое чтение логов из stdout/stderr
- [ ] Настраиваемые таймауты
- [ ] Поддержка приостановки процессов (pause/resume)
- [ ] Метрики использования ресурсов (CPU, память)
- [ ] Уведомления о событиях процессов
- [ ] Поддержка групп процессов

## См. также

- [EnvironmentManager](./ENVIRONMENT_MANAGER.md) - Управление окружениями
- [SecurityScanner](./SECURITY_SCANNER.md) - Сканирование безопасности
- [Database](./DATABASE.md) - Работа с базой данных

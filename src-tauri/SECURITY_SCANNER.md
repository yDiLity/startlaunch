# SecurityScanner - Сканер безопасности

## Обзор

SecurityScanner - это компонент AutoLaunch, который анализирует проекты и команды на предмет потенциальных угроз безопасности. Он помогает защитить систему пользователя от выполнения вредоносного кода.

## Возможности

### 1. Сканирование команд

SecurityScanner анализирует команды запуска на предмет опасных операций:

**Критические угрозы:**
- Удаление корневой директории (`rm -rf /`)
- Fork bomb атаки
- Перезапись системных устройств

**Высокие угрозы:**
- Рекурсивное удаление файлов (`rm -rf`)
- Выполнение с правами суперпользователя (`sudo`)
- Выполнение скриптов из интернета (`curl | bash`, `wget | sh`)
- Динамическое выполнение кода (`eval`, `exec`)
- Небезопасные права доступа (`chmod 777`)

**Средние угрозы:**
- Подавление вывода ошибок (`>/dev/null 2>&1`)
- Фоновое выполнение процессов (`nohup`, `&`)

### 2. Система доверенных репозиториев

Пользователи могут добавлять репозитории в список доверенных, чтобы пропустить проверки безопасности для известных безопасных проектов.

**Особенности:**
- Нормализация URL для единообразного сравнения
- Сохранение списка в локальном файле и базе данных
- Поддержка различных форматов URL (с/без .git, с/без слэша в конце)

### 3. Сканирование проектов

Анализ всего проекта, включая:
- Команды запуска из конфигурационных файлов
- Скрипты в package.json, pyproject.toml и других файлах
- Предупреждения из анализатора проекта

## API

### Rust API

```rust
use crate::security_scanner::SecurityScanner;

// Создание сканера
let mut scanner = SecurityScanner::new()?;

// Сканирование команды
let warnings = scanner.scan_command("sudo rm -rf /tmp");

// Проверка доверия к репозиторию
let is_trusted = scanner.is_trusted_repository("https://github.com/user/repo");

// Добавление в доверенные
scanner.add_trusted_repository("https://github.com/user/repo")?;

// Удаление из доверенных
scanner.remove_trusted_repository("https://github.com/user/repo")?;

// Получение списка доверенных
let repos = scanner.get_trusted_repositories();

// Сканирование проекта
let warnings = scanner.scan_project(&project_info);
```

### Tauri Commands

```typescript
import { invoke } from "@tauri-apps/api/tauri";

// Сканирование безопасности проекта
const warnings = await invoke("scan_project_security", { 
  project_info: projectInfo 
});

// Сканирование команды
const warnings = await invoke("scan_command_security", { 
  command: "npm start" 
});

// Проверка доверия
const isTrusted = await invoke("is_trusted_repository", { 
  repo_url: "https://github.com/user/repo" 
});

// Добавление в доверенные
await invoke("add_trusted_repository", { 
  repo_url: "https://github.com/user/repo" 
});

// Удаление из доверенных
await invoke("remove_trusted_repository", { 
  repo_url: "https://github.com/user/repo" 
});

// Получение списка доверенных
const repos = await invoke("get_trusted_repositories");
```

## UI Компонент

### SecurityWarnings Component

React компонент для отображения предупреждений безопасности:

```tsx
import SecurityWarnings from "./components/SecurityWarnings";

<SecurityWarnings 
  warnings={securityWarnings}
  onTrustRepository={handleTrustRepository}
  isTrusted={isTrusted}
/>
```

**Props:**
- `warnings`: Массив предупреждений безопасности
- `onTrustRepository`: Callback для добавления репозитория в доверенные
- `isTrusted`: Флаг, указывающий, является ли репозиторий доверенным

**Особенности UI:**
- Цветовая кодировка по уровню угрозы
- Иконки для визуального различения уровней
- Предложения по устранению угроз
- Кнопка добавления в доверенные
- Блокировка запуска при критических угрозах

## Хранение данных

### Локальный файл
Список доверенных репозиториев хранится в:
- Windows: `%APPDATA%/autolaunch/trusted_repos.json`
- macOS: `~/Library/Application Support/autolaunch/trusted_repos.json`
- Linux: `~/.config/autolaunch/trusted_repos.json`

### База данных
Таблица `trusted_repositories`:
```sql
CREATE TABLE trusted_repositories (
    id TEXT PRIMARY KEY,
    repo_url TEXT NOT NULL UNIQUE,
    added_at TEXT NOT NULL
);
```

## Паттерны обнаружения

Сканер использует регулярные выражения для обнаружения опасных паттернов:

```rust
// Критические
r"rm\s+-rf\s+/"           // Удаление корня
r":\(\)\{\s*:\|:&\s*\};:" // Fork bomb
r"dd\s+if=/dev/zero"      // Перезапись устройства

// Высокие
r"rm\s+-rf"               // Рекурсивное удаление
r"sudo\s+"                // Sudo
r"curl.*\|.*bash"         // Curl pipe bash
r"eval\s*\("              // Eval
r"chmod\s+777"            // Небезопасные права

// Средние
r">/dev/null\s+2>&1"      // Подавление ошибок
r"nohup\s+"               // Nohup
r"&\s*$"                  // Фоновое выполнение
```

## Тестирование

Модульные тесты покрывают:
- Обнаружение всех типов угроз
- Работу с доверенными репозиториями
- Нормализацию URL
- Сканирование проектов
- Множественные предупреждения в одной команде

Запуск тестов:
```bash
cd src-tauri
cargo test security_scanner
```

## Требования

Реализация соответствует требованиям спецификации:

- **Требование 4.3**: Показывать предупреждение при обнаружении потенциально опасных команд ✓
- **Требование 4.4**: Сохранять статус доверия к проекту для будущих запусков ✓

## Будущие улучшения

- Машинное обучение для обнаружения новых угроз
- Интеграция с базами данных известных вредоносных репозиториев
- Анализ содержимого файлов проекта
- Песочница для безопасного выполнения подозрительных команд
- Отчеты о безопасности с детальной информацией

# AutoLaunch

AutoLaunch — это кроссплатформенное десктопное приложение, которое автоматически анализирует, настраивает и запускает проекты с GitHub.

## Возможности

- 🔍 Автоматический анализ структуры проекта и определение стека технологий
- 🚀 Запуск проектов в direct или sandbox окружении
- 🔒 Безопасность через Docker контейнеризацию и анализ угроз
- 📦 Сохранение снимков проектов для быстрого перезапуска
- 📚 История запущенных проектов с поиском и тегами
- ⚙️ Гибкие настройки изоляции и автоочистки

## Поддерживаемые технологии

- Node.js (React, Express, Vue, и др.)
- Python (Flask, Django, FastAPI, и др.)
- Rust (Cargo проекты)
- Go (Go modules)
- Java (Maven, Gradle)
- Docker (Dockerfile, docker-compose)
- Статические сайты (HTML/CSS/JS)

## Установка и запуск

### Требования

- [Bun](https://bun.sh/) (актуальная стабильная версия)
- [Rust](https://rustup.rs/) (для сборки Tauri приложения)
- [Git](https://git-scm.com/)
- [Docker](https://www.docker.com/) (опционально, для изоляции)

### Установка Rust (Windows)

1. Скачайте и запустите [rustup-init.exe](https://rustup.rs/)
2. Следуйте инструкциям установщика
3. Перезапустите терминал
4. Проверьте установку: `cargo --version`

### Запуск в режиме разработки

1. Клонируйте репозиторий:
   ```bash
   git clone https://github.com/autolaunch/autolaunch.git
   cd autolaunch
   ```

2. Установите зависимости:
   ```bash
   bun install
   ```

3. Запустите веб-версию для разработки:
   ```bash
   bun run dev
   ```
   Откройте http://localhost:1420 в браузере

4. Запустите Tauri приложение (требует Rust):
   ```bash
   bun run tauri dev
   ```

### Сборка для продакшена

```bash
bun run build
bun run tauri build
```

## Архитектура

Каноническое описание архитектуры находится в `docs/README.md`.

Проект построен на архитектуре Tauri:
- **Frontend**: React + TypeScript + Vite
- **Backend**: Rust с Tauri
- **База данных**: SQLite
- **Изоляция**: Docker API

### Структура проекта

```
autolaunch/
├── src/                    # React фронтенд
├── src-tauri/             # Rust бэкенд
│   ├── src/
│   │   ├── main.rs        # Точка входа
│   │   ├── commands.rs    # Tauri команды
│   │   ├── models.rs      # Модели данных
│   │   ├── project_analyzer.rs  # Анализ проектов
│   │   ├── environment_manager.rs  # Управление окружениями
│   │   ├── process_controller.rs   # Управление процессами
│   │   ├── security_scanner.rs     # Сканер безопасности
│   │   └── database.rs    # Работа с БД
│   └── Cargo.toml
├── .kiro/specs/           # Спецификации проекта
└── package.json
```

## Разработка

### Запуск тестов

```bash
# Rust тесты
cd src-tauri
cargo test

# Property-based тесты
cargo test --features proptest
```

### Линтинг и форматирование

```bash
# Rust
cd src-tauri
cargo fmt
cargo clippy

# Frontend
bunx tsc --noEmit
bunx biome check src --write
```

## Текущий статус

✅ **Завершено:**
- Базовая структура проекта Tauri
- Анализатор проектов с поддержкой основных стеков
- Парсер GitHub URL
- Система моделей данных
- SQLite база данных с миграциями
- Менеджер окружений, процессов и детекции портов
- React UI для анализа, запуска, логов, истории и настроек
- Система предупреждений безопасности и trusted repositories
- Система снимков проектов
- Property-based и интеграционные тесты в кодовой базе

⚠️ **Ограничения окружения:**
- Полная локальная проверка Tauri/Rust на Windows требует Visual Studio с компонентом C++ toolchain
- Docker-изоляция доступна только при установленном и запущенном Docker

## Лицензия

MIT License - см. [LICENSE](LICENSE) файл для деталей.

## Вклад в проект

1. Форкните репозиторий
2. Создайте ветку для новой функции (`git checkout -b feature/amazing-feature`)
3. Зафиксируйте изменения (`git commit -m 'Add amazing feature'`)
4. Отправьте в ветку (`git push origin feature/amazing-feature`)
5. Откройте Pull Request

## Поддержка

Если у вас есть вопросы или проблемы, создайте [issue](https://github.com/autolaunch/autolaunch/issues) в репозитории.

# Tech Context

## Стек технологий

### Frontend
| Технология       | Версия   | Назначение                        |
|------------------|----------|-----------------------------------|
| React            | ^18.2.0  | UI фреймворк                      |
| TypeScript       | ^5.2.2   | Типизация                         |
| Vite             | ^4.5.0   | Сборщик                           |
| lucide-react     | ^0.294.0 | Иконки                            |
| @tauri-apps/api  | ^1.5.0   | IPC с Rust backend                |

### Backend (Rust)
| Крейт              | Версия  | Назначение                          |
|--------------------|---------|-------------------------------------|
| tauri              | 1.5     | Desktop framework                   |
| tokio              | 1.0     | Async runtime                       |
| sqlx               | 0.7     | SQLite async ORM                    |
| serde / serde_json | 1.0     | Сериализация                        |
| uuid               | 1.0     | UUID v4                             |
| chrono             | 0.4     | Дата/время                          |
| thiserror          | 1.0     | Иерархия ошибок                     |
| anyhow             | 1.0     | Контекст ошибок                     |
| tracing            | 0.1     | Логирование                         |
| regex              | 1.0     | Регулярные выражения                |
| url                | 2.0     | Парсинг URL                         |
| reqwest            | 0.11    | HTTP клиент (GitHub API)            |
| git2               | 0.18    | Git операции                        |
| docker-api         | 0.14    | Docker Engine API                   |
| dirs               | 5.0     | Системные директории                |
| proptest           | 1.0     | Property-based тестирование (dev)   |
| tempfile           | 3.0     | Временные файлы в тестах (dev)      |

## Окружение разработки

- **ОС**: Windows (win32, bash shell)
- **Пакетный менеджер JS**: bun
- **Rust edition**: 2021
- **Tauri версия**: 1.x (не 2.x)
- **Линтер**: biome (только для TS/JS файлов, не MD)

## Команды

```bash
# Разработка (управляет пользователь)
bun run dev          # Vite dev server
bun run tauri dev    # Tauri + Vite

# Сборка
bun run build
bun run tauri build

# Тесты Rust
cd src-tauri && cargo test
cd src-tauri && cargo test --features proptest

# Линтинг Rust
cd src-tauri && cargo fmt && cargo clippy
```

## Ограничения

- Docker должен быть предустановлен (не входит в дистрибутив)
- Tauri 1.x API (не совместим с Tauri 2.x)
- Windows: fallback-изоляция ограничена (нет bubblewrap/firejail)
- GUI-приложения в контейнерах: не в MVP scope

## CI/CD

Не настроен. Только локальная разработка.

# Парсер и валидатор GitHub URL

## Обзор

Модуль `url_parser` реализует парсинг, валидацию и нормализацию GitHub URL для задачи 11 спецификации autolaunch-core.

## Требования

Реализованы следующие требования:

- **Требование 1.1**: Извлечение владельца и имени репозитория из валидного GitHub URL
- **Требование 1.2**: Понятные сообщения об ошибках для невалидных URL
- **Требование 1.3**: Автоматическое преобразование формата owner/repo в полный GitHub URL
- **Требование 1.5**: Сохранение информации о проекте в локальной базе данных

## Использование

### Базовое использование

```rust
use autolaunch::url_parser::GitHubUrlParser;

// Парсинг формата owner/repo
let info = GitHubUrlParser::parse("facebook/react")?;
assert_eq!(info.owner, "facebook");
assert_eq!(info.repo_name, "react");
assert_eq!(info.normalized_url, "https://github.com/facebook/react");

// Парсинг полного URL
let info = GitHubUrlParser::parse("https://github.com/microsoft/vscode")?;
assert_eq!(info.owner, "microsoft");
assert_eq!(info.repo_name, "vscode");

// Нормализация URL
let normalized = GitHubUrlParser::normalize("facebook/react")?;
assert_eq!(normalized, "https://github.com/facebook/react");
```

### Поддерживаемые форматы

1. **Короткий формат**: `owner/repo`
   - Пример: `facebook/react`
   - Автоматически преобразуется в `https://github.com/facebook/react`

2. **Полный HTTPS URL**: `https://github.com/owner/repo`
   - Пример: `https://github.com/microsoft/vscode`

3. **URL с .git суффиксом**: `https://github.com/owner/repo.git`
   - Пример: `https://github.com/rust-lang/rust.git`
   - Суффикс `.git` автоматически удаляется

4. **URL с www**: `https://www.github.com/owner/repo`
   - Пример: `https://www.github.com/torvalds/linux`
   - Нормализуется к `https://github.com/torvalds/linux`

## Валидация

### Правила для владельца (owner)

- Длина: 1-39 символов
- Допустимые символы: буквы, цифры, дефисы, подчеркивания
- Не может начинаться или заканчиваться дефисом

### Правила для имени репозитория (repo)

- Длина: 1-100 символов
- Допустимые символы: буквы, цифры, дефисы, подчеркивания, точки

## Обработка ошибок

Все ошибки возвращаются как `AutoLaunchError::InvalidUrl` с понятными сообщениями:

```rust
// Пустой URL
GitHubUrlParser::parse("")
// Ошибка: "URL не может быть пустым"

// Не-GitHub URL
GitHubUrlParser::parse("https://gitlab.com/test/repo")
// Ошибка: "URL должен быть GitHub репозиторием (github.com), получен: gitlab.com"

// Слишком длинное имя владельца
GitHubUrlParser::parse(&format!("{}/repo", "a".repeat(40)))
// Ошибка: "Имя владельца слишком длинное (максимум 39 символов)"

// Недопустимые символы
GitHubUrlParser::parse("owner@invalid/repo")
// Ошибка: "Имя владельца содержит недопустимые символы"
```

## Тестирование

### Модульные тесты

Модуль содержит 40+ модульных тестов, покрывающих:
- Различные форматы входных данных
- Граничные случаи (максимальная длина, специальные символы)
- Обработку ошибок
- Нормализацию URL

Запуск тестов:
```bash
cargo test url_parser::tests
```

### Property-Based тесты

Реализованы property-based тесты с использованием библиотеки `proptest`:

- **Property 1**: URL парсинг и нормализация (100 итераций)
- **Property 2**: Обработка невалидных входных данных (100 итераций)
- **Property 3**: Сохранение данных проекта в БД (50 итераций)

Запуск property-based тестов:
```bash
cargo test url_parser_property_test
cargo test database_property_test
```

## Интеграция с базой данных

После успешного парсинга URL, информация о проекте сохраняется в базе данных:

```rust
let repo_info = GitHubUrlParser::parse(&url)?;

let project = Project {
    id: Uuid::new_v4().to_string(),
    github_url: repo_info.normalized_url,
    owner: repo_info.owner,
    repo_name: repo_info.repo_name,
    // ... другие поля
};

db.save_project(&project).await?;
```

## Структура данных

```rust
pub struct GitHubRepoInfo {
    pub owner: String,           // Владелец репозитория
    pub repo_name: String,       // Имя репозитория
    pub normalized_url: String,  // Нормализованный URL
}
```

## Примеры использования в commands.rs

```rust
async fn analyze_repository_impl(url: String, state: State<'_, AppState>) -> Result<ProjectInfo> {
    // Парсим и нормализуем URL (Требование 1.1, 1.2, 1.3)
    let repo_info = GitHubUrlParser::parse(&url)?;
    
    // Клонируем репозиторий (Требование 1.4)
    let local_path = clone_repository(
        &repo_info.normalized_url, 
        &repo_info.owner, 
        &repo_info.repo_name
    ).await?;
    
    // Анализируем проект
    let analyzer = ProjectAnalyzer::new();
    let project_info = analyzer.analyze_project(&local_path)?;
    
    // Сохраняем в БД (Требование 1.5)
    let project = Project {
        id: Uuid::new_v4().to_string(),
        github_url: repo_info.normalized_url,
        owner: repo_info.owner,
        repo_name: repo_info.repo_name,
        // ...
    };
    
    db.save_project(&project).await?;
    
    Ok(project_info)
}
```

## Производительность

- Парсинг URL: O(n) где n - длина строки
- Валидация: O(n) с использованием регулярных выражений
- Нормализация: O(1) после парсинга

## Безопасность

- Все входные данные валидируются перед использованием
- Защита от SQL-инъекций через параметризованные запросы
- Ограничение длины входных данных
- Проверка допустимых символов

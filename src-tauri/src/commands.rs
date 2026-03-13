use crate::error::{AutoLaunchError, ErrorContext, Result};
use crate::models::{Project, ProjectInfo, TechStack, TrustLevel, ProcessHandle, ExecutionStatus, SecurityWarning, EnvironmentType};
use crate::project_analyzer::ProjectAnalyzer;
use crate::environment_manager::EnvironmentManager;
use crate::process_controller::ProcessController;
use crate::security_scanner::SecurityScanner;
use crate::snapshot_manager::SnapshotManager;
use crate::settings_manager::{SettingsManager, AppSettings, IsolationMode as SettingsIsolationMode, Theme};
use crate::url_parser::GitHubUrlParser;
use crate::AppState;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;
use chrono::Utc;

// Глобальное хранилище для менеджеров
lazy_static::lazy_static! {
    static ref ENVIRONMENT_MANAGER: EnvironmentManager = EnvironmentManager::new();
    static ref PROCESS_CONTROLLER: ProcessController = ProcessController::new();
    static ref SECURITY_SCANNER: Mutex<SecurityScanner> = Mutex::new(SecurityScanner::new().expect("Не удалось инициализировать SecurityScanner"));
    static ref SNAPSHOT_MANAGER: SnapshotManager = SnapshotManager::new().expect("Не удалось инициализировать SnapshotManager");
    static ref SETTINGS_MANAGER: Mutex<SettingsManager> = Mutex::new(SettingsManager::new().expect("Не удалось инициализировать SettingsManager"));
    static ref RUNNING_PROJECTS: Arc<Mutex<HashMap<String, (crate::environment_manager::Environment, ProcessHandle)>>> = 
        Arc::new(Mutex::new(HashMap::new()));
}

#[tauri::command]
pub async fn analyze_repository(
    url: String,
    state: State<'_, AppState>,
) -> std::result::Result<ProjectInfo, ErrorContext> {
    let result = analyze_repository_impl(url, state).await;
    match result {
        Ok(info) => Ok(info),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn analyze_repository_impl(url: String, state: State<'_, AppState>) -> Result<ProjectInfo> {
    tracing::info!("Начало анализа репозитория: {}", url);
    
    // Парсим и нормализуем URL (Требование 1.1, 1.2, 1.3)
    let repo_info = GitHubUrlParser::parse(&url)?;
    tracing::debug!("URL распознан: owner={}, repo={}", repo_info.owner, repo_info.repo_name);
    
    // Клонируем репозиторий (Требование 1.4)
    tracing::info!("Клонирование репозитория: {}", repo_info.normalized_url);
    let local_path = clone_repository(&repo_info.normalized_url, &repo_info.owner, &repo_info.repo_name).await?;
    tracing::info!("Репозиторий клонирован в: {:?}", local_path);
    
    // Анализируем проект
    tracing::info!("Анализ структуры проекта");
    let analyzer = ProjectAnalyzer::new();
    let project_info = analyzer.analyze_project(&local_path)?;
    tracing::info!("Обнаружен стек: {:?}", project_info.stack);
    
    // Сохраняем информацию о проекте в БД (Требование 1.5)
    let project = Project {
        id: Uuid::new_v4().to_string(),
        github_url: repo_info.normalized_url,
        owner: repo_info.owner,
        repo_name: repo_info.repo_name,
        local_path: local_path.to_string_lossy().to_string(),
        detected_stack: project_info.stack.to_string(),
        trust_level: TrustLevel::Unknown.to_string(),
        created_at: Utc::now().to_rfc3339(),
        last_run_at: None,
        tags: "[]".to_string(),
    };
    
    let db = state.db.lock().await;
    db.save_project(&project).await?;
    
    tracing::info!("Анализ репозитория завершен успешно");
    Ok(project_info)
}

async fn clone_repository(url: &str, owner: &str, repo_name: &str) -> Result<PathBuf> {
    tracing::info!("Подготовка к клонированию репозитория: {}/{}", owner, repo_name);
    
    let temp_dir = std::env::temp_dir();
    let project_dir = temp_dir.join("autolaunch").join(format!("{}_{}", owner, repo_name));

    // Удаляем существующую директорию если есть
    if project_dir.exists() {
        tracing::debug!("Удаление существующей директории: {:?}", project_dir);
        std::fs::remove_dir_all(&project_dir)?;
    }

    // Создаем родительскую директорию
    if let Some(parent) = project_dir.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Клонируем репозиторий
    tracing::info!("Клонирование из {} в {:?}", url, project_dir);
    git2::Repository::clone(url, &project_dir)?;
    
    tracing::info!("Клонирование завершено успешно");
    Ok(project_dir)
}

#[tauri::command]
pub async fn start_project(
    project_id: String,
    state: State<'_, AppState>,
) -> std::result::Result<String, ErrorContext> {
    let result = start_project_impl(project_id, state).await;
    match result {
        Ok(message) => Ok(message),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn start_project_impl(project_id: String, state: State<'_, AppState>) -> Result<String> {
    // Получаем информацию о проекте из БД
    let project = {
        let db = state.db.lock().await;
        db.get_project(&project_id).await?
            .ok_or_else(|| AutoLaunchError::ProjectAnalysis("Проект не найден".to_string()))?
    };

    // Анализируем проект заново для получения актуальной информации
    let analyzer = ProjectAnalyzer::new();
    let project_path = PathBuf::from(&project.local_path);
    let project_info = analyzer.analyze_project(&project_path)?;

    // Создаем окружение
    let environment = ENVIRONMENT_MANAGER.create_environment(&project_info, &project_path).await?;

    // Определяем команду запуска
    let start_command = project_info.entry_command
        .unwrap_or_else(|| "echo 'Команда запуска не определена'".to_string());

    // Запускаем процесс
    let process_handle = PROCESS_CONTROLLER.start_process(&environment, &start_command).await?;

    // Сохраняем информацию о запущенном проекте
    {
        let mut running_projects = RUNNING_PROJECTS.lock().unwrap();
        running_projects.insert(project_id.clone(), (environment, process_handle.clone()));
    }

    // Обновляем время последнего запуска в БД
    let mut updated_project = project;
    updated_project.last_run_at = Some(Utc::now().to_rfc3339());
    {
        let db = state.db.lock().await;
        db.save_project(&updated_project).await?;
    }

    // Требование 10.1, 10.2: Пытаемся определить порт и открыть браузер
    let port_info = if let Some(port) = PROCESS_CONTROLLER.detect_application_port(&process_handle).await? {
        // Ждем немного, чтобы приложение успело запуститься
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        
        // Проверяем доступность порта
        if PROCESS_CONTROLLER.check_port_availability(port, 10).await? {
            // Автоматически открываем браузер
            let _ = PROCESS_CONTROLLER.open_browser_for_port(port).await;
            format!(" Приложение доступно на http://localhost:{}", port)
        } else {
            format!(" Ожидается запуск на порту {}", port)
        }
    } else {
        String::new()
    };

    Ok(format!("Проект успешно запущен! ID процесса: {}.{}", process_handle.id, port_info))
}

#[tauri::command]
pub async fn stop_project(
    project_id: String,
    state: State<'_, AppState>,
) -> std::result::Result<(), ErrorContext> {
    let result = stop_project_impl(project_id, state).await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn stop_project_impl(project_id: String, _state: State<'_, AppState>) -> Result<()> {
    // Получаем информацию о запущенном проекте
    let (environment, process_handle) = {
        let mut running_projects = RUNNING_PROJECTS.lock().unwrap();
        running_projects.remove(&project_id)
            .ok_or_else(|| AutoLaunchError::Process("Проект не запущен".to_string()))?
    };

    // Останавливаем процесс
    PROCESS_CONTROLLER.stop_process(&process_handle).await?;

    // Очищаем окружение
    ENVIRONMENT_MANAGER.cleanup_environment(&environment).await?;

    Ok(())
}

#[tauri::command]
pub async fn get_project_history(
    state: State<'_, AppState>,
) -> std::result::Result<Vec<Project>, ErrorContext> {
    let result = get_project_history_impl(state).await;
    match result {
        Ok(projects) => Ok(projects),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn get_project_history_impl(state: State<'_, AppState>) -> Result<Vec<Project>> {
    let db = state.db.lock().await;
    db.get_all_projects().await
}

#[tauri::command]
pub async fn save_project_snapshot(
    project_id: String,
    state: State<'_, AppState>,
) -> std::result::Result<String, ErrorContext> {
    let result = save_project_snapshot_impl(project_id, state).await;
    match result {
        Ok(snapshot_id) => Ok(snapshot_id),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn save_project_snapshot_impl(project_id: String, state: State<'_, AppState>) -> Result<String> {
    tracing::info!("Сохранение снимка для проекта: {}", project_id);

    // Получаем информацию о проекте из БД
    let project = {
        let db = state.db.lock().await;
        db.get_project(&project_id).await?
            .ok_or_else(|| AutoLaunchError::ProjectAnalysis("Проект не найден".to_string()))?
    };

    // Анализируем проект для получения актуальной информации
    let analyzer = ProjectAnalyzer::new();
    let project_path = PathBuf::from(&project.local_path);
    let project_info = analyzer.analyze_project(&project_path)?;

    // Получаем информацию о запущенном проекте (если запущен)
    let (environment_type, ports, env_vars) = {
        let running_projects = RUNNING_PROJECTS.lock().unwrap();
        if let Some((environment, process_handle)) = running_projects.get(&project_id) {
            let env_type = match environment.mode {
                crate::environment_manager::IsolationMode::Sandbox(_) => EnvironmentType::Docker,
                crate::environment_manager::IsolationMode::Direct(_) => EnvironmentType::Direct,
            };
            (env_type, process_handle.ports.clone(), vec![])
        } else {
            // Если проект не запущен, используем значения по умолчанию
            (EnvironmentType::Direct, vec![], vec![])
        }
    };

    // Создаем снимок (Требование 7.2)
    let snapshot = SNAPSHOT_MANAGER.create_snapshot(
        &project_id,
        &project_path,
        &project_info,
        environment_type,
        ports,
        env_vars,
    ).await?;

    // Сохраняем информацию о снимке в БД
    {
        let db = state.db.lock().await;
        db.save_snapshot(&snapshot).await?;
    }

    tracing::info!("Снимок успешно создан: {}", snapshot.id);
    Ok(snapshot.id)
}

// Команды для работы с настройками (Требование 9)

#[tauri::command]
pub async fn get_settings() -> std::result::Result<AppSettings, ErrorContext> {
    let result = get_settings_impl().await;
    match result {
        Ok(settings) => Ok(settings),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn get_settings_impl() -> Result<AppSettings> {
    tracing::info!("Получение текущих настроек приложения");
    let manager = SETTINGS_MANAGER.lock().unwrap();
    Ok(manager.get_settings().clone())
}

#[tauri::command]
pub async fn update_settings(
    settings: AppSettings,
) -> std::result::Result<(), ErrorContext> {
    let result = update_settings_impl(settings).await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn update_settings_impl(settings: AppSettings) -> Result<()> {
    tracing::info!("Обновление настроек приложения");
    let mut manager = SETTINGS_MANAGER.lock().unwrap();
    manager.update_settings(settings)?;
    
    // Применяем настройку автоочистки если включена (Требование 9.4)
    if manager.get_settings().auto_cleanup {
        tracing::info!("Автоочистка включена, будет применяться при остановке проектов");
    }
    
    Ok(())
}

#[tauri::command]
pub async fn set_default_isolation_mode(
    mode: String,
) -> std::result::Result<(), ErrorContext> {
    let result = set_default_isolation_mode_impl(mode).await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn set_default_isolation_mode_impl(mode: String) -> Result<()> {
    tracing::info!("Установка режима изоляции по умолчанию: {}", mode);
    
    let isolation_mode = match mode.to_lowercase().as_str() {
        "sandbox" => SettingsIsolationMode::Sandbox,
        "direct" => SettingsIsolationMode::Direct,
        _ => return Err(AutoLaunchError::InvalidInput(format!("Неизвестный режим изоляции: {}", mode))),
    };
    
    let mut manager = SETTINGS_MANAGER.lock().unwrap();
    manager.set_default_isolation_mode(isolation_mode)?;
    
    Ok(())
}

#[tauri::command]
pub async fn set_snapshots_path(
    path: String,
) -> std::result::Result<(), ErrorContext> {
    let result = set_snapshots_path_impl(path).await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn set_snapshots_path_impl(path: String) -> Result<()> {
    tracing::info!("Установка пути для снимков: {}", path);
    
    let mut manager = SETTINGS_MANAGER.lock().unwrap();
    manager.set_snapshots_path(path)?;
    
    Ok(())
}

#[tauri::command]
pub async fn set_theme(
    theme: String,
) -> std::result::Result<(), ErrorContext> {
    let result = set_theme_impl(theme).await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn set_theme_impl(theme: String) -> Result<()> {
    tracing::info!("Установка темы оформления: {}", theme);
    
    let theme_enum = match theme.to_lowercase().as_str() {
        "light" => Theme::Light,
        "dark" => Theme::Dark,
        "system" => Theme::System,
        _ => return Err(AutoLaunchError::InvalidInput(format!("Неизвестная тема: {}", theme))),
    };
    
    let mut manager = SETTINGS_MANAGER.lock().unwrap();
    manager.set_theme(theme_enum)?;
    
    Ok(())
}

#[tauri::command]
pub async fn set_auto_cleanup(
    enabled: bool,
) -> std::result::Result<(), ErrorContext> {
    let result = set_auto_cleanup_impl(enabled).await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn set_auto_cleanup_impl(enabled: bool) -> Result<()> {
    tracing::info!("Установка автоочистки: {}", enabled);
    
    let mut manager = SETTINGS_MANAGER.lock().unwrap();
    manager.set_auto_cleanup(enabled)?;
    
    Ok(())
}

#[tauri::command]
pub async fn reset_settings_to_defaults() -> std::result::Result<(), ErrorContext> {
    let result = reset_settings_to_defaults_impl().await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn reset_settings_to_defaults_impl() -> Result<()> {
    tracing::info!("Сброс настроек к значениям по умолчанию");
    
    let mut manager = SETTINGS_MANAGER.lock().unwrap();
    manager.reset_to_defaults()?;
    
    Ok(())
}

#[tauri::command]
pub async fn get_project_status(
    project_id: String,
) -> std::result::Result<serde_json::Value, ErrorContext> {
    let result = get_project_status_impl(project_id).await;
    match result {
        Ok(status) => Ok(status),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn get_project_status_impl(project_id: String) -> Result<serde_json::Value> {
    let running_projects = RUNNING_PROJECTS.lock().unwrap();
    
    if let Some((environment, process_handle)) = running_projects.get(&project_id) {
        let status = PROCESS_CONTROLLER.get_process_status(process_handle).await?;
        let port = PROCESS_CONTROLLER.detect_application_port(process_handle).await?;
        
        Ok(serde_json::json!({
            "running": true,
            "status": format!("{:?}", status),
            "process_id": process_handle.id,
            "container_id": process_handle.container_id,
            "ports": process_handle.ports,
            "detected_port": port,
            "environment_type": match environment.mode {
                crate::environment_manager::IsolationMode::Sandbox(_) => "docker",
                crate::environment_manager::IsolationMode::Direct(_) => "direct"
            }
        }))
    } else {
        Ok(serde_json::json!({
            "running": false,
            "status": "stopped"
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::url_parser::GitHubUrlParser;

    #[test]
    fn test_parse_github_url_owner_repo_format() {
        let result = GitHubUrlParser::parse("facebook/react").unwrap();
        assert_eq!(result.owner, "facebook");
        assert_eq!(result.repo_name, "react");
        assert_eq!(result.normalized_url, "https://github.com/facebook/react");
    }

    #[test]
    fn test_parse_github_url_full_url() {
        let result = GitHubUrlParser::parse("https://github.com/microsoft/vscode").unwrap();
        assert_eq!(result.owner, "microsoft");
        assert_eq!(result.repo_name, "vscode");
        assert_eq!(result.normalized_url, "https://github.com/microsoft/vscode");
    }

    #[test]
    fn test_parse_github_url_invalid() {
        let result = GitHubUrlParser::parse("https://gitlab.com/test/repo");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_github_url_malformed() {
        let result = GitHubUrlParser::parse("not-a-url");
        assert!(result.is_err());
    }
}

// Команды для работы с безопасностью

#[tauri::command]
pub async fn scan_project_security(
    project_info: ProjectInfo,
) -> std::result::Result<Vec<SecurityWarning>, ErrorContext> {
    let result = scan_project_security_impl(project_info).await;
    match result {
        Ok(warnings) => Ok(warnings),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn scan_project_security_impl(project_info: ProjectInfo) -> Result<Vec<SecurityWarning>> {
    let scanner = SECURITY_SCANNER.lock().unwrap();
    Ok(scanner.scan_project(&project_info))
}

#[tauri::command]
pub async fn scan_command_security(
    command: String,
) -> std::result::Result<Vec<SecurityWarning>, ErrorContext> {
    let result = scan_command_security_impl(command).await;
    match result {
        Ok(warnings) => Ok(warnings),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn scan_command_security_impl(command: String) -> Result<Vec<SecurityWarning>> {
    let scanner = SECURITY_SCANNER.lock().unwrap();
    Ok(scanner.scan_command(&command))
}

#[tauri::command]
pub async fn is_trusted_repository(
    repo_url: String,
    state: State<'_, AppState>,
) -> std::result::Result<bool, ErrorContext> {
    let result = is_trusted_repository_impl(repo_url, state).await;
    match result {
        Ok(is_trusted) => Ok(is_trusted),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn is_trusted_repository_impl(repo_url: String, state: State<'_, AppState>) -> Result<bool> {
    let db = state.db.lock().await;
    db.is_trusted_repository(&repo_url).await
}

#[tauri::command]
pub async fn add_trusted_repository(
    repo_url: String,
    state: State<'_, AppState>,
) -> std::result::Result<(), ErrorContext> {
    let result = add_trusted_repository_impl(repo_url, state).await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn add_trusted_repository_impl(repo_url: String, state: State<'_, AppState>) -> Result<()> {
    let db = state.db.lock().await;
    db.add_trusted_repository(&repo_url).await?;
    
    // Также добавляем в локальный сканер
    let mut scanner = SECURITY_SCANNER.lock().unwrap();
    scanner.add_trusted_repository(&repo_url)?;
    
    Ok(())
}

#[tauri::command]
pub async fn remove_trusted_repository(
    repo_url: String,
    state: State<'_, AppState>,
) -> std::result::Result<(), ErrorContext> {
    let result = remove_trusted_repository_impl(repo_url, state).await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn remove_trusted_repository_impl(repo_url: String, state: State<'_, AppState>) -> Result<()> {
    let db = state.db.lock().await;
    db.remove_trusted_repository(&repo_url).await?;
    
    // Также удаляем из локального сканера
    let mut scanner = SECURITY_SCANNER.lock().unwrap();
    scanner.remove_trusted_repository(&repo_url)?;
    
    Ok(())
}

#[tauri::command]
pub async fn get_trusted_repositories(
    state: State<'_, AppState>,
) -> std::result::Result<Vec<String>, ErrorContext> {
    let result = get_trusted_repositories_impl(state).await;
    match result {
        Ok(repos) => Ok(repos),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn get_trusted_repositories_impl(state: State<'_, AppState>) -> Result<Vec<String>> {
    let db = state.db.lock().await;
    db.get_trusted_repositories().await
}

// Команды для управления процессами (Требование 6)

#[tauri::command]
pub async fn restart_project(
    project_id: String,
) -> std::result::Result<(), ErrorContext> {
    let result = restart_project_impl(project_id).await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn restart_project_impl(project_id: String) -> Result<()> {
    // Получаем handle процесса
    let process_handle = {
        let running_projects = RUNNING_PROJECTS.lock().unwrap();
        running_projects.get(&project_id)
            .map(|(_, handle)| handle.clone())
            .ok_or_else(|| AutoLaunchError::Process("Проект не запущен".to_string()))?
    };

    // Перезапускаем процесс (Требование 6.3: Перезапуск = остановка + запуск)
    PROCESS_CONTROLLER.restart_process(&process_handle).await?;

    Ok(())
}

#[tauri::command]
pub async fn get_process_logs(
    project_id: String,
) -> std::result::Result<Vec<crate::models::LogEntry>, ErrorContext> {
    let result = get_process_logs_impl(project_id).await;
    match result {
        Ok(logs) => Ok(logs),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn get_process_logs_impl(project_id: String) -> Result<Vec<crate::models::LogEntry>> {
    let process_handle = {
        let running_projects = RUNNING_PROJECTS.lock().unwrap();
        running_projects.get(&project_id)
            .map(|(_, handle)| handle.clone())
            .ok_or_else(|| AutoLaunchError::Process("Проект не запущен".to_string()))?
    };

    PROCESS_CONTROLLER.get_process_logs(&process_handle).await
}

#[tauri::command]
pub async fn stop_all_projects() -> std::result::Result<Vec<String>, ErrorContext> {
    let result = stop_all_projects_impl().await;
    match result {
        Ok(stopped) => Ok(stopped),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn stop_all_projects_impl() -> Result<Vec<String>> {
    // Требование 6.5: Остановить все запущенные проекты
    let stopped_ids = PROCESS_CONTROLLER.stop_all_processes().await?;
    
    // Очищаем окружения для всех остановленных проектов
    let environments_to_cleanup: Vec<_> = {
        let mut running_projects = RUNNING_PROJECTS.lock().unwrap();
        stopped_ids.iter()
            .filter_map(|id| running_projects.remove(id))
            .map(|(env, _)| env)
            .collect()
    };

    for env in environments_to_cleanup {
        let _ = ENVIRONMENT_MANAGER.cleanup_environment(&env).await;
    }

    Ok(stopped_ids)
}

#[tauri::command]
pub async fn get_running_projects() -> std::result::Result<Vec<String>, ErrorContext> {
    let running_projects = RUNNING_PROJECTS.lock().unwrap();
    Ok(running_projects.keys().cloned().collect())
}

#[tauri::command]
pub async fn has_running_projects() -> std::result::Result<bool, ErrorContext> {
    Ok(PROCESS_CONTROLLER.has_running_processes())
}

// Команды для работы со снимками проектов (Требование 7)

#[tauri::command]
pub async fn load_project_snapshot(
    snapshot_id: String,
    state: State<'_, AppState>,
) -> std::result::Result<String, ErrorContext> {
    let result = load_project_snapshot_impl(snapshot_id, state).await;
    match result {
        Ok(message) => Ok(message),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn load_project_snapshot_impl(snapshot_id: String, state: State<'_, AppState>) -> Result<String> {
    tracing::info!("Загрузка снимка: {}", snapshot_id);

    // Получаем информацию о снимке из БД
    let snapshot = {
        let db = state.db.lock().await;
        db.get_snapshot(&snapshot_id).await?
            .ok_or_else(|| AutoLaunchError::ProjectAnalysis("Снимок не найден".to_string()))?
    };

    // Загружаем снимок (Требование 7.3: быстрый запуск)
    let (snapshot_path, metadata) = SNAPSHOT_MANAGER.load_snapshot(&snapshot_id).await?;

    // Создаем окружение из снимка
    let analyzer = ProjectAnalyzer::new();
    let project_info = analyzer.analyze_project(&snapshot_path)?;

    let environment = ENVIRONMENT_MANAGER.create_environment(&project_info, &snapshot_path).await?;

    // Определяем команду запуска из метаданных
    let start_command = metadata.entry_command
        .unwrap_or_else(|| "echo 'Команда запуска не определена'".to_string());

    // Запускаем процесс
    let process_handle = PROCESS_CONTROLLER.start_process(&environment, &start_command).await?;

    // Сохраняем информацию о запущенном проекте
    {
        let mut running_projects = RUNNING_PROJECTS.lock().unwrap();
        running_projects.insert(snapshot.project_id.clone(), (environment, process_handle.clone()));
    }

    // Обновляем время последнего запуска проекта в БД
    if let Some(mut project) = {
        let db = state.db.lock().await;
        db.get_project(&snapshot.project_id).await?
    } {
        project.last_run_at = Some(Utc::now().to_rfc3339());
        let db = state.db.lock().await;
        db.save_project(&project).await?;
    }

    Ok(format!("Проект запущен из снимка! ID процесса: {}", process_handle.id))
}

#[tauri::command]
pub async fn delete_project_snapshot(
    snapshot_id: String,
    state: State<'_, AppState>,
) -> std::result::Result<(), ErrorContext> {
    let result = delete_project_snapshot_impl(snapshot_id, state).await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn delete_project_snapshot_impl(snapshot_id: String, state: State<'_, AppState>) -> Result<()> {
    tracing::info!("Удаление снимка: {}", snapshot_id);

    // Удаляем файлы снимка (Требование 7.5: полная очистка)
    SNAPSHOT_MANAGER.delete_snapshot(&snapshot_id).await?;

    // Удаляем запись из БД
    {
        let db = state.db.lock().await;
        db.delete_snapshot(&snapshot_id).await?;
    }

    tracing::info!("Снимок {} успешно удален", snapshot_id);
    Ok(())
}

#[tauri::command]
pub async fn get_project_snapshots(
    project_id: String,
    state: State<'_, AppState>,
) -> std::result::Result<Vec<crate::models::ProjectSnapshot>, ErrorContext> {
    let result = get_project_snapshots_impl(project_id, state).await;
    match result {
        Ok(snapshots) => Ok(snapshots),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn get_project_snapshots_impl(project_id: String, state: State<'_, AppState>) -> Result<Vec<crate::models::ProjectSnapshot>> {
    let db = state.db.lock().await;
    db.get_snapshots_for_project(&project_id).await
}

#[tauri::command]
pub async fn get_all_snapshots(
    state: State<'_, AppState>,
) -> std::result::Result<Vec<crate::models::ProjectSnapshot>, ErrorContext> {
    let result = get_all_snapshots_impl(state).await;
    match result {
        Ok(snapshots) => Ok(snapshots),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn get_all_snapshots_impl(state: State<'_, AppState>) -> Result<Vec<crate::models::ProjectSnapshot>> {
    let db = state.db.lock().await;
    db.get_all_snapshots().await
}

#[tauri::command]
pub async fn cleanup_old_snapshots(
    max_age_days: u64,
    state: State<'_, AppState>,
) -> std::result::Result<Vec<String>, ErrorContext> {
    let result = cleanup_old_snapshots_impl(max_age_days, state).await;
    match result {
        Ok(deleted) => Ok(deleted),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn cleanup_old_snapshots_impl(max_age_days: u64, state: State<'_, AppState>) -> Result<Vec<String>> {
    tracing::info!("Очистка снимков старше {} дней", max_age_days);

    // Удаляем старые снимки
    let deleted_snapshots = SNAPSHOT_MANAGER.cleanup_old_snapshots(max_age_days).await?;

    // Удаляем записи из БД
    {
        let db = state.db.lock().await;
        for snapshot_id in &deleted_snapshots {
            db.delete_snapshot(snapshot_id).await?;
        }
    }

    tracing::info!("Удалено {} старых снимков", deleted_snapshots.len());
    Ok(deleted_snapshots)
}

// Команды для менеджера проектов (Требование 8)

#[tauri::command]
pub async fn update_project_tags(
    project_id: String,
    tags: Vec<String>,
    state: State<'_, AppState>,
) -> std::result::Result<(), ErrorContext> {
    let result = update_project_tags_impl(project_id, tags, state).await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn update_project_tags_impl(project_id: String, tags: Vec<String>, state: State<'_, AppState>) -> Result<()> {
    tracing::info!("Обновление тегов для проекта {}: {:?}", project_id, tags);
    
    let db = state.db.lock().await;
    
    // Получаем проект
    let mut project = db.get_project(&project_id).await?
        .ok_or_else(|| AutoLaunchError::NotFound(format!("Проект {} не найден", project_id)))?;
    
    // Обновляем теги
    project.tags = serde_json::to_string(&tags)?;
    
    // Сохраняем проект
    db.save_project(&project).await?;
    
    tracing::info!("Теги успешно обновлены для проекта {}", project_id);
    Ok(())
}

#[tauri::command]
pub async fn get_project_tags(
    project_id: String,
    state: State<'_, AppState>,
) -> std::result::Result<Vec<String>, ErrorContext> {
    let result = get_project_tags_impl(project_id, state).await;
    match result {
        Ok(tags) => Ok(tags),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn get_project_tags_impl(project_id: String, state: State<'_, AppState>) -> Result<Vec<String>> {
    let db = state.db.lock().await;
    
    let project = db.get_project(&project_id).await?
        .ok_or_else(|| AutoLaunchError::NotFound(format!("Проект {} не найден", project_id)))?;
    
    let tags: Vec<String> = serde_json::from_str(&project.tags).unwrap_or_default();
    Ok(tags)
}

#[tauri::command]
pub async fn search_projects_by_query(
    query: String,
    state: State<'_, AppState>,
) -> std::result::Result<Vec<Project>, ErrorContext> {
    let result = search_projects_by_query_impl(query, state).await;
    match result {
        Ok(projects) => Ok(projects),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn search_projects_by_query_impl(query: String, state: State<'_, AppState>) -> Result<Vec<Project>> {
    tracing::info!("Поиск проектов по запросу: {}", query);
    
    let db = state.db.lock().await;
    let projects = db.search_projects(&query).await?;
    
    tracing::info!("Найдено {} проектов", projects.len());
    Ok(projects)
}

#[tauri::command]
pub async fn filter_projects_by_tags(
    tags: Vec<String>,
    state: State<'_, AppState>,
) -> std::result::Result<Vec<Project>, ErrorContext> {
    let result = filter_projects_by_tags_impl(tags, state).await;
    match result {
        Ok(projects) => Ok(projects),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn filter_projects_by_tags_impl(tags: Vec<String>, state: State<'_, AppState>) -> Result<Vec<Project>> {
    tracing::info!("Фильтрация проектов по тегам: {:?}", tags);
    
    let db = state.db.lock().await;
    let all_projects = db.get_all_projects().await?;
    
    // Фильтруем проекты по тегам
    let filtered: Vec<Project> = all_projects.into_iter()
        .filter(|project| {
            let project_tags: Vec<String> = serde_json::from_str(&project.tags).unwrap_or_default();
            tags.iter().any(|tag| project_tags.contains(tag))
        })
        .collect();
    
    tracing::info!("Найдено {} проектов с указанными тегами", filtered.len());
    Ok(filtered)
}

#[tauri::command]
pub async fn delete_project(
    project_id: String,
    state: State<'_, AppState>,
) -> std::result::Result<(), ErrorContext> {
    let result = delete_project_impl(project_id, state).await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn delete_project_impl(project_id: String, state: State<'_, AppState>) -> Result<()> {
    tracing::info!("Удаление проекта {}", project_id);
    
    let db = state.db.lock().await;
    
    // Получаем проект для удаления локальных файлов
    if let Some(project) = db.get_project(&project_id).await? {
        // Удаляем локальную директорию проекта
        let local_path = std::path::Path::new(&project.local_path);
        if local_path.exists() {
            tracing::info!("Удаление локальной директории: {:?}", local_path);
            std::fs::remove_dir_all(local_path)?;
        }
        
        // Удаляем все снимки проекта
        let snapshots = db.get_snapshots_for_project(&project_id).await?;
        for snapshot in snapshots {
            let snapshot_path = std::path::Path::new(&snapshot.snapshot_path);
            if snapshot_path.exists() {
                tracing::info!("Удаление снимка: {:?}", snapshot_path);
                std::fs::remove_dir_all(snapshot_path)?;
            }
            db.delete_snapshot(&snapshot.id).await?;
        }
    }
    
    // Удаляем проект из БД
    db.delete_project(&project_id).await?;
    
    tracing::info!("Проект {} успешно удален", project_id);
    Ok(())
}

#[tauri::command]
pub async fn update_project_last_run(
    project_id: String,
    state: State<'_, AppState>,
) -> std::result::Result<(), ErrorContext> {
    let result = update_project_last_run_impl(project_id, state).await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn update_project_last_run_impl(project_id: String, state: State<'_, AppState>) -> Result<()> {
    tracing::info!("Обновление времени последнего запуска для проекта {}", project_id);
    
    let db = state.db.lock().await;
    
    // Получаем проект
    let mut project = db.get_project(&project_id).await?
        .ok_or_else(|| AutoLaunchError::NotFound(format!("Проект {} не найден", project_id)))?;
    
    // Обновляем время последнего запуска
    project.last_run_at = Some(chrono::Utc::now().to_rfc3339());
    
    // Сохраняем проект
    db.save_project(&project).await?;
    
    tracing::info!("Время последнего запуска обновлено для проекта {}", project_id);
    Ok(())
}

#[tauri::command]
pub async fn get_all_tags(
    state: State<'_, AppState>,
) -> std::result::Result<Vec<String>, ErrorContext> {
    let result = get_all_tags_impl(state).await;
    match result {
        Ok(tags) => Ok(tags),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn get_all_tags_impl(state: State<'_, AppState>) -> Result<Vec<String>> {
    tracing::info!("Получение всех уникальных тегов");
    
    let db = state.db.lock().await;
    let all_projects = db.get_all_projects().await?;
    
    // Собираем все уникальные теги
    let mut all_tags = std::collections::HashSet::new();
    for project in all_projects {
        let tags: Vec<String> = serde_json::from_str(&project.tags).unwrap_or_default();
        for tag in tags {
            all_tags.insert(tag);
        }
    }
    
    let mut tags: Vec<String> = all_tags.into_iter().collect();
    tags.sort();
    
    tracing::info!("Найдено {} уникальных тегов", tags.len());
    Ok(tags)
}

// Команды для работы с веб-интерфейсом и портами (Требование 10)

#[tauri::command]
pub async fn detect_and_open_browser(
    project_id: String,
) -> std::result::Result<serde_json::Value, ErrorContext> {
    let result = detect_and_open_browser_impl(project_id).await;
    match result {
        Ok(info) => Ok(info),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn detect_and_open_browser_impl(project_id: String) -> Result<serde_json::Value> {
    tracing::info!("Детекция порта и открытие браузера для проекта {}", project_id);
    
    // Получаем handle процесса
    let process_handle = {
        let running_projects = RUNNING_PROJECTS.lock().unwrap();
        running_projects.get(&project_id)
            .map(|(_, handle)| handle.clone())
            .ok_or_else(|| AutoLaunchError::Process("Проект не запущен".to_string()))?
    };

    // Требование 10.1: Автоматически определить порт из логов или конфигурации
    let detected_port = PROCESS_CONTROLLER.detect_application_port(&process_handle).await?;
    
    if let Some(port) = detected_port {
        // Требование 10.4: Проверяем доступность порта
        let is_available = PROCESS_CONTROLLER.check_port_availability(port, 30).await?;
        
        if is_available {
            // Требование 10.2: Автоматически открыть браузер на localhost:port
            PROCESS_CONTROLLER.open_browser_for_port(port).await?;
            
            Ok(serde_json::json!({
                "success": true,
                "port": port,
                "url": format!("http://localhost:{}", port),
                "message": format!("Браузер открыт на http://localhost:{}", port)
            }))
        } else {
            // Требование 10.4: Показать сообщение с инструкциями
            Ok(serde_json::json!({
                "success": false,
                "port": port,
                "message": format!(
                    "Приложение не запустилось на ожидаемом порту {}. \
                    Проверьте логи приложения для диагностики проблемы. \
                    Возможно, порт занят другим приложением или требуется дополнительная настройка.",
                    port
                )
            }))
        }
    } else {
        Ok(serde_json::json!({
            "success": false,
            "message": "Не удалось определить порт приложения. Проверьте логи для получения дополнительной информации."
        }))
    }
}

#[tauri::command]
pub async fn check_port_status(
    port: u16,
) -> std::result::Result<bool, ErrorContext> {
    let result = check_port_status_impl(port).await;
    match result {
        Ok(is_available) => Ok(is_available),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn check_port_status_impl(port: u16) -> Result<bool> {
    tracing::info!("Проверка доступности порта {}", port);
    PROCESS_CONTROLLER.check_port_availability(port, 5).await
}

#[tauri::command]
pub async fn open_browser_url(
    url: String,
) -> std::result::Result<(), ErrorContext> {
    let result = open_browser_url_impl(url).await;
    match result {
        Ok(_) => Ok(()),
        Err(e) => Err(ErrorContext::from(e)),
    }
}

async fn open_browser_url_impl(url: String) -> Result<()> {
    tracing::info!("Открытие браузера: {}", url);
    
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(&["/C", "start", &url])
            .spawn()?;
    }
    
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&url)
            .spawn()?;
    }
    
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&url)
            .spawn()?;
    }
    
    Ok(())
}

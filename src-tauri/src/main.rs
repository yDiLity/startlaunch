// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod database;
mod models;
mod project_analyzer;
mod environment_manager;
mod process_controller;
mod security_scanner;
mod snapshot_manager;
mod settings_manager;
mod error;
mod url_parser;

#[cfg(test)]
mod project_manager_test;

#[cfg(test)]
mod settings_manager_test;

#[cfg(test)]
mod url_parser_test;

#[cfg(test)]
mod url_parser_property_test;

#[cfg(test)]
mod database_property_test;

#[cfg(test)]
mod project_analyzer_property_test;

use commands::*;
use database::Database;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing_subscriber;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Mutex<Database>>,
}

#[tokio::main]
async fn main() {
    // Инициализация улучшенной системы логирования
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    tracing::info!("Запуск AutoLaunch приложения");

    // Инициализация базы данных
    let db = match Database::new().await {
        Ok(db) => {
            tracing::info!("База данных успешно инициализирована");
            db
        }
        Err(e) => {
            tracing::error!("Не удалось инициализировать базу данных: {}", e);
            panic!("Критическая ошибка при инициализации базы данных");
        }
    };

    let app_state = AppState {
        db: Arc::new(Mutex::new(db)),
    };

    tracing::info!("Запуск Tauri приложения");

    tauri::Builder::default()
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            analyze_repository,
            start_project,
            stop_project,
            restart_project,
            get_project_status,
            get_project_history,
            get_process_logs,
            stop_all_projects,
            get_running_projects,
            has_running_projects,
            save_project_snapshot,
            load_project_snapshot,
            delete_project_snapshot,
            get_project_snapshots,
            get_all_snapshots,
            cleanup_old_snapshots,
            get_settings,
            update_settings,
            set_default_isolation_mode,
            set_snapshots_path,
            set_theme,
            set_auto_cleanup,
            reset_settings_to_defaults,
            scan_project_security,
            scan_command_security,
            is_trusted_repository,
            add_trusted_repository,
            remove_trusted_repository,
            get_trusted_repositories,
            update_project_tags,
            get_project_tags,
            search_projects_by_query,
            filter_projects_by_tags,
            delete_project,
            update_project_last_run,
            get_all_tags,
            detect_and_open_browser,
            check_port_status,
            open_browser_url
        ])
        .run(tauri::generate_context!())
        .expect("Ошибка при запуске Tauri приложения");
}
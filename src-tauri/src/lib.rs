pub mod commands;
pub mod database;
pub mod models;
pub mod project_analyzer;
pub mod environment_manager;
pub mod process_controller;
pub mod security_scanner;
pub mod snapshot_manager;
pub mod settings_manager;
pub mod url_parser;
pub mod error;

pub use commands::*;
pub use error::*;

#[cfg(test)]
mod ui_property_tests {
    include!("ui_property_test.rs");
}
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Project {
    pub id: String,
    pub github_url: String,
    pub owner: String,
    pub repo_name: String,
    pub local_path: String,
    pub detected_stack: String,
    pub trust_level: String,
    pub created_at: String,
    pub last_run_at: Option<String>,
    pub tags: String, // JSON array
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInfo {
    pub stack: TechStack,
    pub entry_command: Option<String>,
    pub dependencies: Vec<Dependency>,
    pub config_files: Vec<ConfigFile>,
    pub security_warnings: Vec<SecurityWarning>,
    pub trust_level: TrustLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl ToString for TechStack {
    fn to_string(&self) -> String {
        match self {
            TechStack::NodeJs { version } => format!("NodeJs({})", version.as_deref().unwrap_or("unknown")),
            TechStack::Python { version } => format!("Python({})", version.as_deref().unwrap_or("unknown")),
            TechStack::Rust { edition } => format!("Rust({})", edition.as_deref().unwrap_or("unknown")),
            TechStack::Go { version } => format!("Go({})", version.as_deref().unwrap_or("unknown")),
            TechStack::Java { version } => format!("Java({})", version.as_deref().unwrap_or("unknown")),
            TechStack::Docker { compose } => format!("Docker(compose: {})", compose),
            TechStack::Static { framework } => format!("Static({})", framework.as_deref().unwrap_or("unknown")),
            TechStack::Unknown => "Unknown".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub name: String,
    pub version: Option<String>,
    pub dev: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFile {
    pub path: PathBuf,
    pub file_type: ConfigFileType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConfigFileType {
    PackageJson,
    RequirementsTxt,
    PyprojectToml,
    CargoToml,
    Dockerfile,
    DockerCompose,
    GoMod,
    PomXml,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityWarning {
    pub level: SecurityLevel,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecurityLevel {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrustLevel {
    Unknown,
    Trusted,
    Untrusted,
}

impl ToString for TrustLevel {
    fn to_string(&self) -> String {
        match self {
            TrustLevel::Unknown => "Unknown".to_string(),
            TrustLevel::Trusted => "Trusted".to_string(),
            TrustLevel::Untrusted => "Untrusted".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExecutionStatus {
    Preparing,
    Installing,
    Starting,
    Running,
    Stopping,
    Stopped,
    Failed { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessHandle {
    pub id: String,
    pub pid: Option<u32>,
    pub container_id: Option<String>,
    pub ports: Vec<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String,
}

// Модели для снимков проектов (Требование 7)

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ProjectSnapshot {
    pub id: String,
    pub project_id: String,
    pub snapshot_path: String,
    pub environment_type: String,
    pub metadata: String, // JSON
    pub created_at: String,
    pub size_bytes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    pub entry_command: Option<String>,
    pub ports: Vec<u16>,
    pub environment_variables: Vec<(String, String)>,
    pub dependencies: Vec<Dependency>,
    pub tech_stack: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EnvironmentType {
    Docker,
    Direct,
}
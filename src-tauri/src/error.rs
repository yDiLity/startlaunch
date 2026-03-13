use thiserror::Error;

#[derive(Debug, Error)]
pub enum AutoLaunchError {
    #[error("Ошибка анализа проекта: {0}")]
    ProjectAnalysis(String),
    
    #[error("Ошибка окружения: {0}")]
    Environment(String),
    
    #[error("Ошибка процесса: {0}")]
    Process(String),
    
    #[error("Ошибка безопасности: {0}")]
    Security(String),
    
    #[error("Ошибка базы данных: {0}")]
    Database(#[from] sqlx::Error),
    
    #[error("Ошибка ввода-вывода: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Ошибка сети: {0}")]
    Network(#[from] reqwest::Error),
    
    #[error("Ошибка Git: {0}")]
    Git(#[from] git2::Error),
    
    #[error("Невалидный URL: {0}")]
    InvalidUrl(String),
    
    #[error("Невалидные входные данные: {0}")]
    InvalidInput(String),
    
    #[error("Не найдено: {0}")]
    NotFound(String),
    
    #[error("Ошибка сериализации JSON: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, AutoLaunchError>;

#[derive(Debug, serde::Serialize)]
pub struct ErrorContext {
    pub error: String,
    pub suggestion: Option<String>,
    pub user_friendly_message: String,
}

impl From<AutoLaunchError> for ErrorContext {
    fn from(error: AutoLaunchError) -> Self {
        let (suggestion, user_friendly_message) = match &error {
            AutoLaunchError::InvalidUrl(_) => (
                Some("Проверьте правильность URL репозитория".to_string()),
                "Неверный формат URL репозитория".to_string(),
            ),
            AutoLaunchError::Network(_) => (
                Some("Проверьте подключение к интернету".to_string()),
                "Ошибка сетевого подключения".to_string(),
            ),
            AutoLaunchError::Git(_) => (
                Some("Убедитесь, что репозиторий существует и доступен".to_string()),
                "Не удалось клонировать репозиторий".to_string(),
            ),
            _ => (None, error.to_string()),
        };

        ErrorContext {
            error: error.to_string(),
            suggestion,
            user_friendly_message,
        }
    }
}
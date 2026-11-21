use serde::Serialize;

/// Error type returned by every Tauri command.
/// Serialized as `{ kind, message }` so the frontend can branch on `kind`.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    Hardware(String),
    #[error("{0}")]
    Catalog(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Validation(String),
    /// Ollama is not installed or not reachable.
    #[error("{0}")]
    RuntimeUnavailable(String),
    /// Ollama returned an error.
    #[error("{0}")]
    Runtime(String),
    #[error("{0}")]
    Network(String),
    #[error("Cancelled")]
    Cancelled,
    #[error("{0}")]
    Database(String),
    #[error("{0}")]
    Internal(String),
}

impl AppError {
    fn kind(&self) -> &'static str {
        match self {
            AppError::Hardware(_) => "hardware",
            AppError::Catalog(_) => "catalog",
            AppError::NotFound(_) => "not_found",
            AppError::Validation(_) => "validation",
            AppError::RuntimeUnavailable(_) => "runtime_unavailable",
            AppError::Runtime(_) => "runtime",
            AppError::Network(_) => "network",
            AppError::Cancelled => "cancelled",
            AppError::Database(_) => "database",
            AppError::Internal(_) => "internal",
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeStruct;
        let mut st = s.serialize_struct("AppError", 2)?;
        st.serialize_field("kind", self.kind())?;
        st.serialize_field("message", &self.to_string())?;
        st.end()
    }
}

impl From<rusqlite::Error> for AppError {
    fn from(e: rusqlite::Error) -> Self {
        AppError::Database(e.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum NightShiftError {
    #[error("agenda not found: {0}")]
    AgendaNotFound(String),

    #[error("invalid agenda: {0}")]
    InvalidAgenda(String),

    #[error("evidence source not allowed: {0}")]
    EvidenceSourceNotAllowed(String),

    #[error("authority ceiling violated: requested {requested:?} exceeds ceiling {ceiling:?}")]
    AuthorityCeilingExceeded {
        requested: String,
        ceiling: String,
    },

    #[error("run aborted by coordination preflight: {0}")]
    PreflightBlocked(String),

    #[error("store error: {0}")]
    Store(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T, E = NightShiftError> = std::result::Result<T, E>;

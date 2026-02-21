use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ApiError {
    #[error("HTTP {status}: {message}")]
    Http { status: u16, message: String },

    #[error("Network error: {0}")]
    Network(String),

    #[error("Failed to parse response: {0}")]
    Parse(String),

    #[error("Unauthorized: invalid or missing API key")]
    Unauthorized,

    #[error("Forbidden: insufficient permissions")]
    Forbidden,

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Rate limited: try again later")]
    RateLimited,
}

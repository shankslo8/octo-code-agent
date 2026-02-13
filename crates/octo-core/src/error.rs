use thiserror::Error;

#[derive(Error, Debug)]
pub enum OctoError {
    #[error("Provider error: {0}")]
    Provider(#[from] ProviderError),

    #[error("Tool error: {0}")]
    Tool(#[from] ToolError),

    #[error("Storage error: {0}")]
    Storage(#[from] StorageError),

    #[error("Config error: {0}")]
    Config(#[from] ConfigError),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Cancelled")]
    Cancelled,
}

#[derive(Error, Debug, Clone)]
pub enum ProviderError {
    #[error("HTTP error: {0}")]
    Http(String),

    #[error("API error ({status}): {message}")]
    Api { status: u16, message: String },

    #[error("Rate limited, retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    #[error("Stream error: {0}")]
    Stream(String),

    #[error("Unsupported model: {0}")]
    UnsupportedModel(String),

    #[error("Missing API key for provider: {0}")]
    MissingApiKey(String),
}

#[derive(Error, Debug)]
pub enum ToolError {
    #[error("Tool not found: {0}")]
    NotFound(String),

    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Timeout after {0}s")]
    Timeout(u64),

    #[error("Permission denied for tool '{tool}' action '{action}'")]
    PermissionDenied { tool: String, action: String },
}

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Migration error: {0}")]
    Migration(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Config file error: {0}")]
    File(String),

    #[error("Invalid configuration: {0}")]
    Invalid(String),

    #[error("Missing required field: {0}")]
    MissingField(String),
}

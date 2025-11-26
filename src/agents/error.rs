//! Error types for the AI Agent system

use thiserror::Error;

/// Errors that can occur during agent operations
#[derive(Debug, Error)]
pub enum AgentError {
    /// Agent not found
    #[error("Agent not found: {0}")]
    NotFound(String),

    /// Session not found
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// LLM provider error
    #[error("LLM error: {0}")]
    Llm(#[from] LlmError),

    /// Tool execution error
    #[error("Tool execution error: {0}")]
    ToolExecution(String),

    /// Memory/persistence error
    #[error("Memory error: {0}")]
    Memory(String),

    /// Execution error
    #[error("Execution error: {0}")]
    Execution(String),

    /// Max iterations reached
    #[error("Max iterations ({0}) reached without completion")]
    MaxIterations(u32),

    /// Timeout
    #[error("Operation timed out after {0}s")]
    Timeout(u64),

    /// Budget exceeded
    #[error("Token budget exceeded: used {used}, limit {limit}")]
    BudgetExceeded { used: u32, limit: u32 },

    /// Cancelled
    #[error("Operation was cancelled")]
    Cancelled,

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Errors specific to LLM provider operations
#[derive(Debug, Error)]
pub enum LlmError {
    /// Provider not found
    #[error("LLM provider not found: {0}")]
    ProviderNotFound(String),

    /// Model not found
    #[error("Model not found: {0}")]
    ModelNotFound(String),

    /// API error
    #[error("API error: {status} - {message}")]
    Api { status: u16, message: String },

    /// Rate limited
    #[error("Rate limited: retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },

    /// Authentication error
    #[error("Authentication error: {0}")]
    Authentication(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Streaming error
    #[error("Streaming error: {0}")]
    Streaming(String),

    /// Parse error
    #[error("Parse error: {0}")]
    Parse(String),

    /// Token limit exceeded
    #[error("Token limit exceeded: {tokens} tokens exceeds model limit of {limit}")]
    TokenLimitExceeded { tokens: u32, limit: u32 },

    /// Content filtered
    #[error("Content filtered by safety system")]
    ContentFiltered,

    /// Invalid request
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Timeout
    #[error("Request timed out")]
    Timeout,
}

impl From<reqwest::Error> for LlmError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            LlmError::Timeout
        } else if err.is_connect() {
            LlmError::Network(format!("Connection error: {}", err))
        } else {
            LlmError::Network(err.to_string())
        }
    }
}

impl From<serde_json::Error> for AgentError {
    fn from(err: serde_json::Error) -> Self {
        AgentError::Serialization(err.to_string())
    }
}

impl From<std::io::Error> for AgentError {
    fn from(err: std::io::Error) -> Self {
        AgentError::Internal(format!("IO error: {}", err))
    }
}

/// Result type alias for agent operations
pub type AgentResult<T> = Result<T, AgentError>;

/// Result type alias for LLM operations
pub type LlmResult<T> = Result<T, LlmError>;

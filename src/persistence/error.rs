//! Persistence layer error types

use thiserror::Error;

/// Errors that can occur in the persistence layer
#[derive(Debug, Error)]
pub enum PersistenceError {
    /// Database connection error
    #[error("Database connection error: {0}")]
    Connection(String),

    /// Item not found
    #[error("Item not found: {entity_type} with identifier '{identifier}'")]
    NotFound {
        entity_type: String,
        identifier: String,
    },

    /// Duplicate entry
    #[error("Duplicate entry: {entity_type} with name '{name}' already exists")]
    Duplicate { entity_type: String, name: String },

    /// Version conflict for optimistic locking
    #[error("Version conflict: expected version {expected}, but found version {actual}")]
    VersionConflict { expected: u64, actual: u64 },

    /// Migration error
    #[error("Migration error: {0}")]
    Migration(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Transaction error
    #[error("Transaction error: {0}")]
    Transaction(String),

    /// Invalid rollback operation
    #[error("Invalid rollback: commit '{commit_hash}' not found or cannot be rolled back")]
    InvalidRollback { commit_hash: String },

    /// Commit not found
    #[error("Commit not found: '{commit_hash}'")]
    CommitNotFound { commit_hash: String },

    /// Tag not found
    #[error("Tag not found: '{name}'")]
    TagNotFound { name: String },

    /// Tag already exists
    #[error("Tag already exists: '{name}'")]
    TagExists { name: String },

    /// Database error from SQLx
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Generic internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl PersistenceError {
    /// Convert to HTTP status code for API responses
    pub fn status_code(&self) -> axum::http::StatusCode {
        use axum::http::StatusCode;
        match self {
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::Duplicate { .. } => StatusCode::CONFLICT,
            Self::VersionConflict { .. } => StatusCode::CONFLICT,
            Self::TagExists { .. } => StatusCode::CONFLICT,
            Self::InvalidRollback { .. } => StatusCode::BAD_REQUEST,
            Self::CommitNotFound { .. } => StatusCode::NOT_FOUND,
            Self::TagNotFound { .. } => StatusCode::NOT_FOUND,
            Self::Serialization(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

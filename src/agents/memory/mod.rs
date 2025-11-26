//! Memory/persistence module for agent conversations
//!
//! Provides storage backends for conversation history:
//! - In-memory (default, lost on restart)
//! - File-based (persisted to disk)
//! - Database (SQLite/PostgreSQL)

mod in_memory;
mod file;
mod database;
mod strategy;

pub use in_memory::InMemoryStore;
pub use file::FileStore;
pub use database::DatabaseStore;
pub use strategy::*;

use async_trait::async_trait;
use std::sync::Arc;

use crate::agents::config::{MemoryBackend, MemoryConfig};
use crate::agents::domain::{ConversationSession, Message, SessionSummary};
use crate::agents::error::AgentResult;

/// Trait for conversation storage backends
#[async_trait]
pub trait ConversationStore: Send + Sync {
    /// Save a conversation session
    async fn save(&self, session: &ConversationSession) -> AgentResult<()>;

    /// Load a conversation session by ID
    async fn load(&self, session_id: &str) -> AgentResult<Option<ConversationSession>>;

    /// Delete a conversation session
    async fn delete(&self, session_id: &str) -> AgentResult<()>;

    /// List sessions for an agent
    async fn list(
        &self,
        agent_name: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> AgentResult<Vec<SessionSummary>>;

    /// Add a message to an existing session
    async fn add_message(&self, session_id: &str, message: Message) -> AgentResult<()>;

    /// Get or create a session
    async fn get_or_create(&self, session_id: &str, agent_name: &str) -> AgentResult<ConversationSession> {
        if let Some(session) = self.load(session_id).await? {
            Ok(session)
        } else {
            let session = ConversationSession::new(session_id.to_string(), agent_name.to_string());
            self.save(&session).await?;
            Ok(session)
        }
    }
}

/// Create a conversation store from configuration
pub fn create_store(config: &MemoryConfig) -> AgentResult<Arc<dyn ConversationStore>> {
    match config.backend {
        MemoryBackend::InMemory => {
            Ok(Arc::new(InMemoryStore::new(config.max_messages as usize)))
        }
        MemoryBackend::File => {
            let path = config.file_path.clone().unwrap_or_else(|| "data/sessions".to_string());
            Ok(Arc::new(FileStore::new(path)?))
        }
        MemoryBackend::Database => {
            // Database store requires async initialization
            // Return in-memory as fallback for now
            // Real implementation would use DatabaseStore::new(config.database_url.clone())
            Ok(Arc::new(InMemoryStore::new(config.max_messages as usize)))
        }
    }
}

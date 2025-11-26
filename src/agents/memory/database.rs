//! Database-based conversation store (SQLite/PostgreSQL)

use async_trait::async_trait;

use super::ConversationStore;
use crate::agents::domain::{ConversationSession, Message, SessionSummary};
use crate::agents::error::{AgentError, AgentResult};

/// Database conversation store
/// TODO: Implement with sqlx
pub struct DatabaseStore {
    // pool: sqlx::AnyPool,
}

impl DatabaseStore {
    /// Create a new database store
    pub async fn new(_database_url: &str) -> AgentResult<Self> {
        // TODO: Implement database connection
        // let pool = sqlx::AnyPool::connect(database_url).await?;
        // Create tables if needed

        Err(AgentError::Memory(
            "Database store not yet implemented".to_string(),
        ))
    }
}

#[async_trait]
impl ConversationStore for DatabaseStore {
    async fn save(&self, _session: &ConversationSession) -> AgentResult<()> {
        Err(AgentError::Memory(
            "Database store not yet implemented".to_string(),
        ))
    }

    async fn load(&self, _session_id: &str) -> AgentResult<Option<ConversationSession>> {
        Err(AgentError::Memory(
            "Database store not yet implemented".to_string(),
        ))
    }

    async fn delete(&self, _session_id: &str) -> AgentResult<()> {
        Err(AgentError::Memory(
            "Database store not yet implemented".to_string(),
        ))
    }

    async fn list(
        &self,
        _agent_name: Option<&str>,
        _limit: usize,
        _offset: usize,
    ) -> AgentResult<Vec<SessionSummary>> {
        Err(AgentError::Memory(
            "Database store not yet implemented".to_string(),
        ))
    }

    async fn add_message(&self, _session_id: &str, _message: Message) -> AgentResult<()> {
        Err(AgentError::Memory(
            "Database store not yet implemented".to_string(),
        ))
    }
}

//! In-memory conversation store

use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::ConversationStore;
use crate::agents::domain::{ConversationSession, Message, SessionSummary};
use crate::agents::error::{AgentError, AgentResult};

/// In-memory conversation store
pub struct InMemoryStore {
    sessions: Arc<RwLock<HashMap<String, ConversationSession>>>,
    max_messages_per_session: usize,
}

impl InMemoryStore {
    /// Create a new in-memory store
    pub fn new(max_messages_per_session: usize) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            max_messages_per_session,
        }
    }
}

#[async_trait]
impl ConversationStore for InMemoryStore {
    async fn save(&self, session: &ConversationSession) -> AgentResult<()> {
        let mut sessions = self.sessions.write().await;
        sessions.insert(session.session_id.clone(), session.clone());
        Ok(())
    }

    async fn load(&self, session_id: &str) -> AgentResult<Option<ConversationSession>> {
        let sessions = self.sessions.read().await;
        Ok(sessions.get(session_id).cloned())
    }

    async fn delete(&self, session_id: &str) -> AgentResult<()> {
        let mut sessions = self.sessions.write().await;
        sessions.remove(session_id);
        Ok(())
    }

    async fn list(
        &self,
        agent_name: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> AgentResult<Vec<SessionSummary>> {
        let sessions = self.sessions.read().await;

        let mut summaries: Vec<SessionSummary> = sessions
            .values()
            .filter(|s| agent_name.map_or(true, |name| s.agent_name == name))
            .map(|s| s.to_summary())
            .collect();

        // Sort by updated_at descending
        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        // Apply pagination
        Ok(summaries.into_iter().skip(offset).take(limit).collect())
    }

    async fn add_message(&self, session_id: &str, message: Message) -> AgentResult<()> {
        let mut sessions = self.sessions.write().await;

        if let Some(session) = sessions.get_mut(session_id) {
            session.add_message(message);

            // Trim if exceeds max messages
            if session.messages.len() > self.max_messages_per_session {
                let remove_count = session.messages.len() - self.max_messages_per_session;
                session.messages.drain(0..remove_count);
            }

            Ok(())
        } else {
            Err(AgentError::SessionNotFound(session_id.to_string()))
        }
    }
}

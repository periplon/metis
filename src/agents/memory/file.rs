//! File-based conversation store

use async_trait::async_trait;
use std::path::PathBuf;
use tokio::fs;

use super::ConversationStore;
use crate::agents::domain::{ConversationSession, Message, SessionSummary};
use crate::agents::error::{AgentError, AgentResult};

/// File-based conversation store
pub struct FileStore {
    base_path: PathBuf,
}

impl FileStore {
    /// Create a new file store
    pub fn new(base_path: impl Into<PathBuf>) -> AgentResult<Self> {
        let base_path = base_path.into();

        // Create directory if it doesn't exist (sync for constructor)
        std::fs::create_dir_all(&base_path).map_err(|e| {
            AgentError::Memory(format!("Failed to create directory: {}", e))
        })?;

        Ok(Self { base_path })
    }

    fn session_path(&self, session_id: &str) -> PathBuf {
        self.base_path.join(format!("{}.json", session_id))
    }
}

#[async_trait]
impl ConversationStore for FileStore {
    async fn save(&self, session: &ConversationSession) -> AgentResult<()> {
        let path = self.session_path(&session.session_id);
        let content = serde_json::to_string_pretty(session)?;

        fs::write(&path, content).await.map_err(|e| {
            AgentError::Memory(format!("Failed to write session file: {}", e))
        })?;

        Ok(())
    }

    async fn load(&self, session_id: &str) -> AgentResult<Option<ConversationSession>> {
        let path = self.session_path(session_id);

        if !path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&path).await.map_err(|e| {
            AgentError::Memory(format!("Failed to read session file: {}", e))
        })?;

        let session: ConversationSession = serde_json::from_str(&content)?;
        Ok(Some(session))
    }

    async fn delete(&self, session_id: &str) -> AgentResult<()> {
        let path = self.session_path(session_id);

        if path.exists() {
            fs::remove_file(&path).await.map_err(|e| {
                AgentError::Memory(format!("Failed to delete session file: {}", e))
            })?;
        }

        Ok(())
    }

    async fn list(
        &self,
        agent_name: Option<&str>,
        limit: usize,
        offset: usize,
    ) -> AgentResult<Vec<SessionSummary>> {
        let mut entries = fs::read_dir(&self.base_path).await.map_err(|e| {
            AgentError::Memory(format!("Failed to read directory: {}", e))
        })?;

        let mut summaries = Vec::new();

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            AgentError::Memory(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();

            if path.extension().map_or(false, |ext| ext == "json") {
                if let Ok(content) = fs::read_to_string(&path).await {
                    if let Ok(session) = serde_json::from_str::<ConversationSession>(&content) {
                        if agent_name.map_or(true, |name| session.agent_name == name) {
                            summaries.push(session.to_summary());
                        }
                    }
                }
            }
        }

        // Sort by updated_at descending
        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        // Apply pagination
        Ok(summaries.into_iter().skip(offset).take(limit).collect())
    }

    async fn add_message(&self, session_id: &str, message: Message) -> AgentResult<()> {
        let mut session = self.load(session_id).await?.ok_or_else(|| {
            AgentError::SessionNotFound(session_id.to_string())
        })?;

        session.add_message(message);
        self.save(&session).await
    }
}

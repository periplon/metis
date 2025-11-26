//! Domain types for the AI Agent system
//!
//! Core abstractions that define the agent system's domain model.

mod agent;
mod message;
mod response;
mod tool_call;

pub use agent::*;
pub use message::*;
pub use response::*;
pub use tool_call::*;

use async_trait::async_trait;
use serde_json::Value;

/// Port trait for agent operations (follows existing ResourcePort/ToolPort pattern)
#[async_trait]
pub trait AgentPort: Send + Sync {
    /// Execute an agent with the given input
    async fn execute(
        &self,
        name: &str,
        input: Value,
        session_id: Option<String>,
    ) -> anyhow::Result<AgentResponse>;

    /// Execute an agent with streaming response
    fn execute_stream(
        &self,
        name: &str,
        input: Value,
        session_id: Option<String>,
    ) -> AgentStream;

    /// List all available agents
    async fn list_agents(&self) -> anyhow::Result<Vec<AgentInfo>>;

    /// Get agent details by name
    async fn get_agent(&self, name: &str) -> anyhow::Result<Option<AgentInfo>>;

    /// Get a conversation session by ID
    async fn get_session(&self, session_id: &str) -> anyhow::Result<Option<ConversationSession>>;

    /// List sessions for an agent
    async fn list_sessions(
        &self,
        agent_name: &str,
        limit: usize,
        offset: usize,
    ) -> anyhow::Result<Vec<SessionSummary>>;

    /// Delete a session
    async fn delete_session(&self, session_id: &str) -> anyhow::Result<()>;
}

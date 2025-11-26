//! Core agent implementations
//!
//! Provides different agent types:
//! - SingleTurnAgent: One request → one response
//! - MultiTurnAgent: Maintains conversation history
//! - ReActAgent: Reasoning + Action loop with tool calling

mod single_turn;
mod multi_turn;
mod react;

pub use single_turn::SingleTurnAgent;
pub use multi_turn::MultiTurnAgent;
pub use react::ReActAgent;

use std::sync::Arc;

use crate::agents::config::AgentConfig;
use crate::agents::domain::AgentType;
use crate::agents::error::AgentResult;
use crate::agents::llm::LlmProvider;
use crate::agents::memory::ConversationStore;
use crate::domain::ToolPort;

/// Trait for executable agents
pub trait Agent: Send + Sync {
    /// Get the agent's configuration
    fn config(&self) -> &AgentConfig;

    /// Execute the agent
    fn execute(
        &self,
        input: serde_json::Value,
        session_id: Option<String>,
    ) -> crate::agents::domain::AgentStream;
}

/// Create an agent from configuration
pub fn create_agent(
    config: AgentConfig,
    llm_provider: Arc<dyn LlmProvider>,
    memory_store: Arc<dyn ConversationStore>,
    tool_handler: Arc<dyn ToolPort>,
) -> AgentResult<Arc<dyn Agent>> {
    match config.agent_type {
        AgentType::SingleTurn => {
            let agent = SingleTurnAgent::new(config, llm_provider);
            Ok(Arc::new(agent))
        }
        AgentType::MultiTurn => {
            let agent = MultiTurnAgent::new(config, llm_provider, memory_store);
            Ok(Arc::new(agent))
        }
        AgentType::ReAct => {
            let agent = ReActAgent::new(config, llm_provider, memory_store, tool_handler);
            Ok(Arc::new(agent))
        }
    }
}

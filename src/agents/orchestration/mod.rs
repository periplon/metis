//! Multi-agent orchestration patterns
//!
//! Provides different ways to coordinate multiple agents:
//! - Sequential: Agents execute in order, passing results
//! - Hierarchical: Manager agent delegates to workers
//! - Collaborative: Agents work in parallel, results merged

mod sequential;
mod hierarchical;
mod collaborative;

pub use sequential::SequentialOrchestrator;
pub use hierarchical::HierarchicalOrchestrator;
pub use collaborative::CollaborativeOrchestrator;

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

use crate::agents::config::{OrchestrationConfig, OrchestrationPattern};
use crate::agents::core::Agent;
use crate::agents::domain::AgentStream;

/// Orchestration engine for multi-agent coordination
pub struct OrchestrationEngine {
    agents: HashMap<String, Arc<dyn Agent>>,
}

impl OrchestrationEngine {
    /// Create a new orchestration engine
    pub fn new(agents: HashMap<String, Arc<dyn Agent>>) -> Self {
        Self { agents }
    }

    /// Execute an orchestration
    pub fn execute(&self, config: &OrchestrationConfig, input: Value) -> AgentStream {
        match config.pattern {
            OrchestrationPattern::Sequential => {
                SequentialOrchestrator::execute(&self.agents, config, input)
            }
            OrchestrationPattern::Hierarchical => {
                HierarchicalOrchestrator::execute(&self.agents, config, input)
            }
            OrchestrationPattern::Collaborative => {
                CollaborativeOrchestrator::execute(&self.agents, config, input)
            }
        }
    }

    /// Get an agent by name
    pub fn get_agent(&self, name: &str) -> Option<Arc<dyn Agent>> {
        self.agents.get(name).cloned()
    }

    /// List available agents
    pub fn list_agents(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }
}

//! Hierarchical orchestration pattern

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use serde_json::{json, Value};

use crate::agents::config::OrchestrationConfig;
use crate::agents::core::Agent;
use crate::agents::domain::{AgentChunk, AgentResponse, AgentStatus, AgentStream, AgentStreamSender};

/// Hierarchical orchestrator: manager delegates to workers
pub struct HierarchicalOrchestrator;

impl HierarchicalOrchestrator {
    /// Execute with hierarchical pattern
    pub fn execute(
        agents: &HashMap<String, Arc<dyn Agent>>,
        config: &OrchestrationConfig,
        input: Value,
    ) -> AgentStream {
        let (sender, stream) = AgentStream::channel(64);

        let agents = agents.clone();
        let config = config.clone();

        tokio::spawn(async move {
            Self::execute_internal(agents, config, input, sender).await;
        });

        stream
    }

    async fn execute_internal(
        agents: HashMap<String, Arc<dyn Agent>>,
        config: OrchestrationConfig,
        _input: Value,
        sender: AgentStreamSender,
    ) {
        let start_time = Instant::now();

        // Get manager agent
        let manager_name = match &config.manager_agent {
            Some(name) => name,
            None => {
                let _ = sender.send(AgentChunk::error(
                    "Hierarchical orchestration requires a manager_agent".to_string()
                )).await;
                return;
            }
        };

        let _manager = match agents.get(manager_name) {
            Some(a) => a,
            None => {
                let _ = sender.send(AgentChunk::error(format!(
                    "Manager agent not found: {}",
                    manager_name
                ))).await;
                return;
            }
        };

        // Send starting status
        if sender.send(AgentChunk::status(AgentStatus::Starting)).await.is_err() {
            return;
        }

        // TODO: Implement full hierarchical orchestration
        // The manager agent would have access to worker agents as tools
        // When it calls a worker, we intercept and execute that agent
        //
        // For now, return a placeholder response
        let _ = sender.send(AgentChunk::thought(
            "Hierarchical orchestration is not yet fully implemented. \
             The manager agent would delegate to workers via tool calls.".to_string()
        )).await;

        let execution_time = start_time.elapsed().as_millis() as u64;
        let response = AgentResponse {
            output: json!({
                "message": "Hierarchical orchestration pending full implementation",
                "manager": manager_name,
                "workers": config.agents.iter().map(|a| &a.agent).collect::<Vec<_>>()
            }),
            tool_calls: Vec::new(),
            reasoning_steps: Vec::new(),
            session_id: None,
            iterations: 0,
            usage: None,
            execution_time_ms: execution_time,
        };

        let _ = sender.send(AgentChunk::complete(response)).await;
    }
}

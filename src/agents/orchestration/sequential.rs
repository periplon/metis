//! Sequential orchestration pattern

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use futures::StreamExt;
use rhai::{Engine, Scope};
use serde_json::{json, Value};

use crate::agents::config::OrchestrationConfig;
use crate::agents::core::Agent;
use crate::agents::domain::{AgentChunk, AgentResponse, AgentStatus, AgentStream, AgentStreamSender};

/// Sequential orchestrator: agents execute in order
pub struct SequentialOrchestrator;

impl SequentialOrchestrator {
    /// Execute agents sequentially
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
        input: Value,
        sender: AgentStreamSender,
    ) {
        let start_time = Instant::now();
        let rhai_engine = Engine::new();

        let mut current_input = input;
        let mut all_results: HashMap<String, Value> = HashMap::new();
        let mut all_tool_calls = Vec::new();

        // Send starting status
        if sender.send(AgentChunk::status(AgentStatus::Starting)).await.is_err() {
            return;
        }

        for agent_ref in &config.agents {
            // Check condition if present
            if let Some(condition) = &agent_ref.condition {
                let mut scope = Scope::new();
                scope.push("results", all_results.clone());
                scope.push("input", current_input.clone());

                match rhai_engine.eval_with_scope::<bool>(&mut scope, condition) {
                    Ok(should_run) => {
                        if !should_run {
                            let _ = sender.send(AgentChunk::thought(format!(
                                "Skipping agent '{}' due to condition: {}",
                                agent_ref.agent, condition
                            ))).await;
                            continue;
                        }
                    }
                    Err(e) => {
                        let _ = sender.send(AgentChunk::error(format!(
                            "Condition evaluation failed for '{}': {}",
                            agent_ref.agent, e
                        ))).await;
                        return;
                    }
                }
            }

            // Get the agent
            let agent = match agents.get(&agent_ref.agent) {
                Some(a) => a,
                None => {
                    let _ = sender.send(AgentChunk::error(format!(
                        "Agent not found: {}",
                        agent_ref.agent
                    ))).await;
                    return;
                }
            };

            // Apply input transformation if present
            let agent_input = if let Some(transform) = &agent_ref.input_transform {
                let mut scope = Scope::new();
                scope.push("results", all_results.clone());
                scope.push("input", current_input.clone());

                match rhai_engine.eval_with_scope::<rhai::Dynamic>(&mut scope, transform) {
                    Ok(transformed) => {
                        serde_json::to_value(transformed).unwrap_or(current_input.clone())
                    }
                    Err(e) => {
                        let _ = sender.send(AgentChunk::error(format!(
                            "Input transformation failed for '{}': {}",
                            agent_ref.agent, e
                        ))).await;
                        return;
                    }
                }
            } else {
                current_input.clone()
            };

            // Send status
            let _ = sender.send(AgentChunk::thought(format!(
                "Executing agent: {}",
                agent_ref.agent
            ))).await;

            // Execute agent
            let mut agent_stream = agent.execute(agent_input, None);
            let mut agent_response: Option<AgentResponse> = None;

            while let Some(result) = agent_stream.next().await {
                match result {
                    Ok(chunk) => {
                        // Forward chunks (optionally prefix with agent name)
                        match &chunk {
                            AgentChunk::Text { content } => {
                                if sender.send(AgentChunk::text(content)).await.is_err() {
                                    return;
                                }
                            }
                            AgentChunk::Complete { response } => {
                                agent_response = Some(response.clone());
                            }
                            AgentChunk::Error { message } => {
                                let _ = sender.send(AgentChunk::error(format!(
                                    "Agent '{}' failed: {}",
                                    agent_ref.agent, message
                                ))).await;
                                return;
                            }
                            _ => {
                                // Forward other chunks
                                if sender.send(chunk).await.is_err() {
                                    return;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = sender.send(AgentChunk::error(format!(
                            "Agent '{}' error: {}",
                            agent_ref.agent, e
                        ))).await;
                        return;
                    }
                }
            }

            // Store result and prepare for next agent
            if let Some(response) = agent_response {
                all_results.insert(agent_ref.agent.clone(), response.output.clone());
                all_tool_calls.extend(response.tool_calls);

                // Pass output to next agent
                current_input = json!({ "prompt": response.output });
            }
        }

        // Send final response
        let execution_time = start_time.elapsed().as_millis() as u64;
        let final_response = AgentResponse {
            output: json!({ "results": all_results }),
            tool_calls: all_tool_calls,
            reasoning_steps: Vec::new(),
            session_id: None,
            iterations: config.agents.len() as u32,
            usage: None,
            execution_time_ms: execution_time,
        };

        let _ = sender.send(AgentChunk::complete(final_response)).await;
    }
}

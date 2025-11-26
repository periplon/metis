//! Collaborative orchestration pattern

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use futures::future::join_all;
use futures::StreamExt;
use rhai::{Engine, Scope};
use serde_json::{json, Value};

use crate::agents::config::{MergeStrategy, OrchestrationConfig};
use crate::agents::core::Agent;
use crate::agents::domain::{AgentChunk, AgentResponse, AgentStatus, AgentStream, AgentStreamSender};

/// Collaborative orchestrator: agents work in parallel
pub struct CollaborativeOrchestrator;

impl CollaborativeOrchestrator {
    /// Execute with collaborative pattern
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

        // Send starting status
        if sender.send(AgentChunk::status(AgentStatus::Starting)).await.is_err() {
            return;
        }

        let _ = sender.send(AgentChunk::thought(format!(
            "Executing {} agents in parallel",
            config.agents.len()
        ))).await;

        // Execute all agents in parallel
        let futures: Vec<_> = config.agents.iter()
            .filter_map(|agent_ref| {
                agents.get(&agent_ref.agent).map(|agent| {
                    let agent = agent.clone();
                    let input = input.clone();
                    let name = agent_ref.agent.clone();

                    async move {
                        let mut stream = agent.execute(input, None);
                        let mut result: Option<AgentResponse> = None;

                        while let Some(chunk_result) = stream.next().await {
                            if let Ok(AgentChunk::Complete { response }) = chunk_result {
                                result = Some(response);
                            }
                        }

                        (name, result)
                    }
                })
            })
            .collect();

        let results: Vec<(String, Option<AgentResponse>)> = join_all(futures).await;

        // Collect successful results
        let mut outputs: HashMap<String, Value> = HashMap::new();
        let mut all_tool_calls = Vec::new();

        for (name, result) in results {
            if let Some(response) = result {
                outputs.insert(name.clone(), response.output);
                all_tool_calls.extend(response.tool_calls);

                let _ = sender.send(AgentChunk::thought(format!(
                    "Agent '{}' completed",
                    name
                ))).await;
            } else {
                let _ = sender.send(AgentChunk::thought(format!(
                    "Agent '{}' failed or produced no result",
                    name
                ))).await;
            }
        }

        // Merge results based on strategy
        let merged_output = Self::merge_results(&outputs, &config.merge_strategy);

        // Send final response
        let execution_time = start_time.elapsed().as_millis() as u64;
        let response = AgentResponse {
            output: merged_output,
            tool_calls: all_tool_calls,
            reasoning_steps: Vec::new(),
            session_id: None,
            iterations: config.agents.len() as u32,
            usage: None,
            execution_time_ms: execution_time,
        };

        let _ = sender.send(AgentChunk::complete(response)).await;
    }

    fn merge_results(outputs: &HashMap<String, Value>, strategy: &MergeStrategy) -> Value {
        match strategy {
            MergeStrategy::Concat => {
                // Concatenate all outputs
                let combined: Vec<Value> = outputs.values().cloned().collect();
                json!({
                    "results": outputs,
                    "combined": combined
                })
            }
            MergeStrategy::Vote => {
                // Simple voting - return the most common output
                // For text outputs, this groups by content
                let mut counts: HashMap<String, usize> = HashMap::new();

                for value in outputs.values() {
                    let key = value.to_string();
                    *counts.entry(key).or_insert(0) += 1;
                }

                let winner = counts.into_iter()
                    .max_by_key(|(_, count)| *count)
                    .map(|(key, _)| key);

                json!({
                    "results": outputs,
                    "winner": winner
                })
            }
            MergeStrategy::Custom { script } => {
                // Use Rhai to merge
                let engine = Engine::new();
                let mut scope = Scope::new();
                scope.push("outputs", outputs.clone());

                match engine.eval_with_scope::<rhai::Dynamic>(&mut scope, script) {
                    Ok(result) => {
                        serde_json::to_value(result).unwrap_or_else(|_| json!({
                            "results": outputs,
                            "error": "Failed to serialize merge result"
                        }))
                    }
                    Err(e) => {
                        json!({
                            "results": outputs,
                            "merge_error": e.to_string()
                        })
                    }
                }
            }
        }
    }
}

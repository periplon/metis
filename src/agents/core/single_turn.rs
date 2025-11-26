//! Single-turn agent implementation

use std::sync::Arc;
use std::time::Instant;

use serde_json::{json, Value};

use super::Agent;
use crate::agents::config::AgentConfig;
use crate::agents::domain::{
    AgentChunk, AgentResponse, AgentStatus, AgentStream, AgentStreamSender, Message,
};
use crate::agents::llm::{CompletionRequest, LlmProvider};
use futures::StreamExt;

/// Single-turn agent: one request → one response, no history
pub struct SingleTurnAgent {
    config: AgentConfig,
    llm: Arc<dyn LlmProvider>,
}

impl SingleTurnAgent {
    /// Create a new single-turn agent
    pub fn new(config: AgentConfig, llm: Arc<dyn LlmProvider>) -> Self {
        Self { config, llm }
    }

    async fn execute_internal(
        config: AgentConfig,
        llm: Arc<dyn LlmProvider>,
        input: Value,
        sender: AgentStreamSender,
    ) {
        let start_time = Instant::now();

        // Send starting status
        if sender.send(AgentChunk::status(AgentStatus::Starting)).await.is_err() {
            return;
        }

        // Build messages
        let prompt = input
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let messages = vec![
            Message::system(&config.system_prompt),
            Message::user(prompt),
        ];

        // Build completion request
        let request = CompletionRequest {
            messages,
            model: Some(config.llm.model.clone()),
            temperature: config.temperature.or(config.llm.temperature),
            max_tokens: config.max_tokens.or(config.llm.max_tokens),
            stream: true,
            ..Default::default()
        };

        // Send generating status
        if sender.send(AgentChunk::status(AgentStatus::Generating)).await.is_err() {
            return;
        }

        // Stream the response
        let mut stream = llm.complete_stream(request);
        let mut full_content = String::new();

        while let Some(result) = stream.next().await {
            match result {
                Ok(chunk) => {
                    if !chunk.content.is_empty() {
                        full_content.push_str(&chunk.content);
                        if sender.send(AgentChunk::text(&chunk.content)).await.is_err() {
                            return;
                        }
                    }

                    // Send usage if available
                    if let Some(usage) = chunk.usage {
                        let _ = sender.send(AgentChunk::usage(crate::agents::domain::TokenUsage {
                            prompt_tokens: usage.prompt_tokens,
                            completion_tokens: usage.completion_tokens,
                            total_tokens: usage.total_tokens,
                        })).await;
                    }
                }
                Err(e) => {
                    let _ = sender.send(AgentChunk::error(e.to_string())).await;
                    return;
                }
            }
        }

        // Send complete response
        let execution_time = start_time.elapsed().as_millis() as u64;
        let response = AgentResponse {
            output: json!({ "content": full_content }),
            tool_calls: Vec::new(),
            reasoning_steps: Vec::new(),
            session_id: None,
            iterations: 1,
            usage: None,
            execution_time_ms: execution_time,
        };

        let _ = sender.send(AgentChunk::complete(response)).await;
    }
}

impl Agent for SingleTurnAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    fn execute(&self, input: Value, _session_id: Option<String>) -> AgentStream {
        let (sender, stream) = AgentStream::channel(64);

        let config = self.config.clone();
        let llm = self.llm.clone();

        tokio::spawn(async move {
            Self::execute_internal(config, llm, input, sender).await;
        });

        stream
    }
}

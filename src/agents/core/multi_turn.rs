//! Multi-turn conversational agent implementation

use std::sync::Arc;
use std::time::Instant;

use serde_json::{json, Value};
use uuid::Uuid;

use super::Agent;
use crate::agents::config::AgentConfig;
use crate::agents::domain::{
    AgentChunk, AgentResponse, AgentStatus, AgentStream, AgentStreamSender, Message,
};
use crate::agents::llm::{CompletionRequest, LlmProvider};
use crate::agents::memory::{apply_strategy, ConversationStore};
use futures::StreamExt;

/// Multi-turn conversational agent with history
pub struct MultiTurnAgent {
    config: AgentConfig,
    llm: Arc<dyn LlmProvider>,
    memory: Arc<dyn ConversationStore>,
}

impl MultiTurnAgent {
    /// Create a new multi-turn agent
    pub fn new(
        config: AgentConfig,
        llm: Arc<dyn LlmProvider>,
        memory: Arc<dyn ConversationStore>,
    ) -> Self {
        Self { config, llm, memory }
    }

    async fn execute_internal(
        config: AgentConfig,
        llm: Arc<dyn LlmProvider>,
        memory: Arc<dyn ConversationStore>,
        input: Value,
        session_id: Option<String>,
        sender: AgentStreamSender,
    ) {
        let start_time = Instant::now();

        // Send starting status
        if sender.send(AgentChunk::status(AgentStatus::Starting)).await.is_err() {
            return;
        }

        // Get or create session
        let session_id = session_id.unwrap_or_else(|| Uuid::new_v4().to_string());

        let mut session = match memory.get_or_create(&session_id, &config.name).await {
            Ok(s) => s,
            Err(e) => {
                let _ = sender.send(AgentChunk::error(format!("Failed to load session: {}", e))).await;
                return;
            }
        };

        // Get user prompt
        let prompt = input
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Add user message to session
        let user_message = Message::user(prompt);
        session.add_message(user_message.clone());

        // Build messages with system prompt + history
        let mut messages = vec![Message::system(&config.system_prompt)];

        // Apply memory strategy
        let history_messages = apply_strategy(
            &session.messages,
            &config.memory.strategy,
            None,
        );
        messages.extend(history_messages);

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

        // Save assistant response to session
        let assistant_message = Message::assistant(&full_content);
        session.add_message(assistant_message);

        // Persist session
        if let Err(e) = memory.save(&session).await {
            tracing::warn!("Failed to save session: {}", e);
        }

        // Send complete response
        let execution_time = start_time.elapsed().as_millis() as u64;
        let response = AgentResponse {
            output: json!({ "content": full_content }),
            tool_calls: Vec::new(),
            reasoning_steps: Vec::new(),
            session_id: Some(session_id),
            iterations: 1,
            usage: None,
            execution_time_ms: execution_time,
        };

        let _ = sender.send(AgentChunk::complete(response)).await;
    }
}

impl Agent for MultiTurnAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    fn execute(&self, input: Value, session_id: Option<String>) -> AgentStream {
        let (sender, stream) = AgentStream::channel(64);

        let config = self.config.clone();
        let llm = self.llm.clone();
        let memory = self.memory.clone();

        tokio::spawn(async move {
            Self::execute_internal(config, llm, memory, input, session_id, sender).await;
        });

        stream
    }
}

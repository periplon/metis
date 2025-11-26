//! ReAct agent implementation (Reasoning + Acting)

use std::sync::Arc;
use std::time::Instant;

use futures::StreamExt;
use serde_json::{json, Value};
use uuid::Uuid;

use super::Agent;
use crate::agents::config::AgentConfig;
use crate::agents::domain::{
    AgentChunk, AgentResponse, AgentStatus, AgentStream, AgentStreamSender,
    Message, ToolCallResult, ToolDefinition,
};
use crate::agents::llm::{CompletionRequest, FinishReason, LlmProvider, ToolCallAccumulator};
use crate::agents::memory::{apply_strategy, ConversationStore};
use crate::domain::ToolPort;

/// ReAct agent: Reasoning + Action loop with tool calling
pub struct ReActAgent {
    config: AgentConfig,
    llm: Arc<dyn LlmProvider>,
    memory: Arc<dyn ConversationStore>,
    tool_handler: Arc<dyn ToolPort>,
}

impl ReActAgent {
    /// Create a new ReAct agent
    pub fn new(
        config: AgentConfig,
        llm: Arc<dyn LlmProvider>,
        memory: Arc<dyn ConversationStore>,
        tool_handler: Arc<dyn ToolPort>,
    ) -> Self {
        Self {
            config,
            llm,
            memory,
            tool_handler,
        }
    }

    async fn execute_internal(
        config: AgentConfig,
        llm: Arc<dyn LlmProvider>,
        memory: Arc<dyn ConversationStore>,
        tool_handler: Arc<dyn ToolPort>,
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

        // Apply memory strategy to get conversation history
        let history_messages = apply_strategy(
            &session.messages,
            &config.memory.strategy,
            None,
        );
        messages.extend(history_messages);

        // Build tool definitions (includes both regular tools and MCP tools)
        let tools = Self::build_tool_definitions(&tool_handler, &config.available_tools, &config.mcp_tools).await;

        let mut all_tool_calls: Vec<ToolCallResult> = Vec::new();
        let mut reasoning_steps: Vec<String> = Vec::new();
        let mut final_content = String::new();

        // ReAct loop
        for iteration in 0..config.max_iterations {
            // Send status
            if sender.send(AgentChunk::status(AgentStatus::Thinking)).await.is_err() {
                return;
            }

            // Build completion request with tools
            let request = CompletionRequest {
                messages: messages.clone(),
                model: Some(config.llm.model.clone()),
                temperature: config.temperature.or(config.llm.temperature),
                max_tokens: config.max_tokens.or(config.llm.max_tokens),
                tools: if tools.is_empty() { None } else { Some(tools.clone()) },
                stream: true,
                ..Default::default()
            };

            // Stream the LLM response
            let mut stream = llm.complete_stream(request);
            let mut content = String::new();
            let mut tool_accumulator = ToolCallAccumulator::new();
            let mut finish_reason = FinishReason::Stop;

            while let Some(result) = stream.next().await {
                match result {
                    Ok(chunk) => {
                        if !chunk.content.is_empty() {
                            content.push_str(&chunk.content);
                            if sender.send(AgentChunk::text(&chunk.content)).await.is_err() {
                                return;
                            }
                        }

                        // Accumulate tool call deltas
                        for delta in &chunk.tool_calls {
                            tool_accumulator.apply_delta(delta);
                        }

                        if let Some(reason) = chunk.finish_reason {
                            finish_reason = reason;
                        }
                    }
                    Err(e) => {
                        let _ = sender.send(AgentChunk::error(e.to_string())).await;
                        return;
                    }
                }
            }

            // Build complete tool calls
            let tool_calls = tool_accumulator.build();

            // If no tool calls, we're done
            if tool_calls.is_empty() || finish_reason == FinishReason::Stop {
                final_content = content;
                break;
            }

            // Add reasoning step
            if !content.is_empty() {
                reasoning_steps.push(format!("Iteration {}: {}", iteration + 1, content));
                let _ = sender.send(AgentChunk::thought(&content)).await;
            }

            // Add assistant message with tool calls
            messages.push(Message::assistant_with_tools(&content, tool_calls.clone()));

            // Execute tool calls
            for tool_call in &tool_calls {
                // Send tool call status
                if sender.send(AgentChunk::status(AgentStatus::CallingTool {
                    tool_name: tool_call.name.clone(),
                })).await.is_err() {
                    return;
                }

                // Send tool call chunk
                if sender.send(AgentChunk::tool_call(tool_call)).await.is_err() {
                    return;
                }

                // Execute the tool
                let tool_start = Instant::now();
                let result = tool_handler.execute_tool(&tool_call.name, tool_call.arguments.clone()).await;
                let tool_time = tool_start.elapsed().as_millis() as u64;

                let tool_result = match result {
                    Ok(output) => {
                        ToolCallResult::success(
                            tool_call.id.clone(),
                            tool_call.name.clone(),
                            tool_call.arguments.clone(),
                            output.clone(),
                            tool_time,
                        )
                    }
                    Err(e) => {
                        ToolCallResult::failure(
                            tool_call.id.clone(),
                            tool_call.name.clone(),
                            tool_call.arguments.clone(),
                            e.to_string(),
                            tool_time,
                        )
                    }
                };

                // Send tool result chunk
                if sender.send(AgentChunk::tool_result(&tool_result)).await.is_err() {
                    return;
                }

                // Add tool result to messages
                messages.push(Message::tool_result(&tool_call.id, &tool_result.output));

                // Track tool call
                all_tool_calls.push(tool_result);
            }
        }

        // Save assistant response to session
        let assistant_message = Message::assistant(&final_content);
        session.add_message(assistant_message);

        // Persist session
        if let Err(e) = memory.save(&session).await {
            tracing::warn!("Failed to save session: {}", e);
        }

        // Send complete response
        let execution_time = start_time.elapsed().as_millis() as u64;
        let iterations = config.max_iterations.min(reasoning_steps.len() as u32 + 1);
        let response = AgentResponse {
            output: json!({ "content": final_content }),
            tool_calls: all_tool_calls,
            reasoning_steps,
            session_id: Some(session_id),
            iterations,
            usage: None,
            execution_time_ms: execution_time,
        };

        let _ = sender.send(AgentChunk::complete(response)).await;
    }

    async fn build_tool_definitions(
        tool_handler: &Arc<dyn ToolPort>,
        available_tools: &[String],
        mcp_tools: &[String],
    ) -> Vec<ToolDefinition> {
        let all_tools = match tool_handler.list_tools().await {
            Ok(tools) => tools,
            Err(_) => return Vec::new(),
        };

        let mut definitions = Vec::new();

        // Handle regular tools
        if available_tools.is_empty() && mcp_tools.is_empty() {
            // Use all tools (including MCP tools)
            definitions = all_tools
                .into_iter()
                .map(|t| ToolDefinition {
                    name: t.name,
                    description: t.description,
                    parameters: t.input_schema,
                })
                .collect();
        } else {
            // Filter to specified regular tools
            for tool in &all_tools {
                // Check if it's in available_tools (regular tools)
                if available_tools.contains(&tool.name) {
                    definitions.push(ToolDefinition {
                        name: tool.name.clone(),
                        description: tool.description.clone(),
                        parameters: tool.input_schema.clone(),
                    });
                    continue;
                }

                // Check if it matches MCP tool patterns
                if tool.name.starts_with("mcp__") {
                    // Extract server and tool name from "mcp__{server}_{tool}"
                    if let Some(name_part) = tool.name.strip_prefix("mcp__") {
                        if let Some((server, tool_name)) = name_part.split_once('_') {
                            for spec in mcp_tools {
                                if let Some((spec_server, spec_tool)) = spec.split_once(':') {
                                    if spec_server == server {
                                        if spec_tool == "*" || spec_tool == tool_name {
                                            definitions.push(ToolDefinition {
                                                name: tool.name.clone(),
                                                description: tool.description.clone(),
                                                parameters: tool.input_schema.clone(),
                                            });
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        definitions
    }
}

impl Agent for ReActAgent {
    fn config(&self) -> &AgentConfig {
        &self.config
    }

    fn execute(&self, input: Value, session_id: Option<String>) -> AgentStream {
        let (sender, stream) = AgentStream::channel(64);

        let config = self.config.clone();
        let llm = self.llm.clone();
        let memory = self.memory.clone();
        let tool_handler = self.tool_handler.clone();

        tokio::spawn(async move {
            Self::execute_internal(config, llm, memory, tool_handler, input, session_id, sender).await;
        });

        stream
    }
}

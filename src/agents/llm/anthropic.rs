//! Anthropic LLM Provider with streaming support

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};
use std::env;

use super::{
    CompletionRequest, CompletionResponse, FinishReason, LlmProvider, LlmStream, LlmStreamSender,
    StreamChunk, TokenUsage, ToolCallDelta,
};
use crate::adapters::secrets::SharedSecretsStore;
use crate::agents::config::LlmProviderConfig;
use crate::agents::domain::{Message, Role, ToolCall};
use crate::agents::error::{LlmError, LlmResult};

/// Anthropic LLM Provider
pub struct AnthropicProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    model: String,
    default_temperature: Option<f32>,
    default_max_tokens: Option<u32>,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider from configuration
    pub fn new(config: &LlmProviderConfig) -> LlmResult<Self> {
        let api_key = if let Some(env_var) = &config.api_key_env {
            env::var(env_var).map_err(|_| {
                LlmError::Authentication(format!(
                    "Environment variable {} not set",
                    env_var
                ))
            })?
        } else {
            env::var("ANTHROPIC_API_KEY").map_err(|_| {
                LlmError::Authentication("ANTHROPIC_API_KEY environment variable not set".to_string())
            })?
        };

        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.anthropic.com".to_string());

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            base_url,
            model: config.model.clone(),
            default_temperature: config.temperature,
            default_max_tokens: config.max_tokens,
        })
    }

    /// Create a new Anthropic provider using secrets store for API key
    pub async fn new_with_secrets(
        config: &LlmProviderConfig,
        secrets: SharedSecretsStore,
    ) -> LlmResult<Self> {
        let env_var = config.api_key_env.as_deref().unwrap_or("ANTHROPIC_API_KEY");

        let api_key = secrets.get_or_env(env_var).await.ok_or_else(|| {
            LlmError::Authentication(format!(
                "API key not found in secrets store or environment variable {}",
                env_var
            ))
        })?;

        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.anthropic.com".to_string());

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            base_url,
            model: config.model.clone(),
            default_temperature: config.temperature,
            default_max_tokens: config.max_tokens,
        })
    }

    /// Build the request body for Anthropic API
    fn build_request_body(&self, request: &CompletionRequest) -> Value {
        let (system_prompt, messages) = self.convert_messages(&request.messages);

        let mut body = json!({
            "model": request.model.as_ref().unwrap_or(&self.model),
            "messages": messages,
            "max_tokens": request.max_tokens.or(self.default_max_tokens).unwrap_or(4096),
        });

        if let Some(system) = system_prompt {
            body["system"] = json!(system);
        }

        if let Some(temp) = request.temperature.or(self.default_temperature) {
            body["temperature"] = json!(temp);
        }

        if let Some(stop) = &request.stop {
            body["stop_sequences"] = json!(stop);
        }

        if let Some(tools) = &request.tools {
            if !tools.is_empty() {
                body["tools"] = json!(tools.iter().map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.parameters
                    })
                }).collect::<Vec<_>>());
            }
        }

        if let Some(tool_choice) = &request.tool_choice {
            body["tool_choice"] = match tool_choice {
                super::ToolChoice::Auto => json!({ "type": "auto" }),
                super::ToolChoice::None => json!({ "type": "none" }),
                super::ToolChoice::Required => json!({ "type": "any" }),
                super::ToolChoice::Tool { name } => json!({
                    "type": "tool",
                    "name": name
                }),
            };
        }

        if request.stream {
            body["stream"] = json!(true);
        }

        body
    }

    /// Convert internal messages to Anthropic format
    /// Returns (system_prompt, messages)
    fn convert_messages(&self, messages: &[Message]) -> (Option<String>, Vec<Value>) {
        let mut system_prompt = None;
        let mut converted = Vec::new();

        for m in messages {
            match m.role {
                Role::System => {
                    system_prompt = Some(m.content.clone());
                }
                Role::User => {
                    converted.push(json!({
                        "role": "user",
                        "content": m.content
                    }));
                }
                Role::Assistant => {
                    if let Some(tool_calls) = &m.tool_calls {
                        let mut content = Vec::new();

                        if !m.content.is_empty() {
                            content.push(json!({
                                "type": "text",
                                "text": m.content
                            }));
                        }

                        for tc in tool_calls {
                            content.push(json!({
                                "type": "tool_use",
                                "id": tc.id,
                                "name": tc.name,
                                "input": tc.arguments
                            }));
                        }

                        converted.push(json!({
                            "role": "assistant",
                            "content": content
                        }));
                    } else {
                        converted.push(json!({
                            "role": "assistant",
                            "content": m.content
                        }));
                    }
                }
                Role::Tool => {
                    // Anthropic expects tool results in user messages
                    converted.push(json!({
                        "role": "user",
                        "content": [{
                            "type": "tool_result",
                            "tool_use_id": m.tool_call_id.as_ref().unwrap_or(&String::new()),
                            "content": m.content
                        }]
                    }));
                }
            }
        }

        (system_prompt, converted)
    }

    /// Parse a non-streaming response
    fn parse_response(&self, response: &AnthropicResponse) -> LlmResult<CompletionResponse> {
        let mut content = String::new();
        let mut tool_calls = Vec::new();

        for block in &response.content {
            match block.block_type.as_str() {
                "text" => {
                    if let Some(text) = &block.text {
                        content.push_str(text);
                    }
                }
                "tool_use" => {
                    if let (Some(id), Some(name), Some(input)) =
                        (&block.id, &block.name, &block.input)
                    {
                        tool_calls.push(ToolCall {
                            id: id.clone(),
                            name: name.clone(),
                            arguments: input.clone(),
                        });
                    }
                }
                _ => {}
            }
        }

        let message = if tool_calls.is_empty() {
            Message::assistant(content)
        } else {
            Message::assistant_with_tools(content, tool_calls)
        };

        let finish_reason = match response.stop_reason.as_deref() {
            Some("end_turn") | Some("stop_sequence") => FinishReason::Stop,
            Some("max_tokens") => FinishReason::Length,
            Some("tool_use") => FinishReason::ToolCalls,
            _ => FinishReason::Stop,
        };

        let usage = Some(TokenUsage {
            prompt_tokens: response.usage.input_tokens,
            completion_tokens: response.usage.output_tokens,
            total_tokens: response.usage.input_tokens + response.usage.output_tokens,
        });

        Ok(CompletionResponse {
            message,
            finish_reason,
            usage,
        })
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn complete(&self, request: CompletionRequest) -> LlmResult<CompletionResponse> {
        let body = self.build_request_body(&request);

        let response = self
            .client
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::Api {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let anthropic_response: AnthropicResponse = response.json().await.map_err(|e| {
            LlmError::Parse(format!("Failed to parse response: {}", e))
        })?;

        self.parse_response(&anthropic_response)
    }

    fn complete_stream(&self, request: CompletionRequest) -> LlmStream {
        let (sender, stream) = LlmStream::channel(64);

        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let base_url = self.base_url.clone();
        let mut req = request;
        req.stream = true;
        let body = self.build_request_body(&req);

        tokio::spawn(async move {
            let result = Self::stream_completion(client, api_key, base_url, body, sender.clone()).await;
            if let Err(e) = result {
                let _ = sender.send_error(e).await;
            }
        });

        stream
    }

    fn count_tokens(&self, text: &str) -> u32 {
        // Anthropic uses a different tokenizer
        // Approximate: ~4 chars per token
        (text.len() / 4) as u32
    }

    fn context_window(&self) -> u32 {
        match self.model.as_str() {
            m if m.contains("claude-3-opus") => 200000,
            m if m.contains("claude-3-sonnet") || m.contains("claude-3-5-sonnet") => 200000,
            m if m.contains("claude-3-haiku") => 200000,
            _ => 200000,
        }
    }

    fn max_output_tokens(&self) -> u32 {
        match self.model.as_str() {
            m if m.contains("claude-3-5-sonnet") => 8192,
            _ => 4096,
        }
    }
}

impl AnthropicProvider {
    async fn stream_completion(
        client: reqwest::Client,
        api_key: String,
        base_url: String,
        body: Value,
        sender: LlmStreamSender,
    ) -> LlmResult<()> {
        let response = client
            .post(format!("{}/v1/messages", base_url))
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = response.status();
        if !status.is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(LlmError::Api {
                status: status.as_u16(),
                message: error_text,
            });
        }

        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        #[allow(unused_assignments)]
        let mut current_tool_use_id = String::new();
        #[allow(unused_assignments)]
        let mut current_tool_name = String::new();
        let mut tool_call_index = 0usize;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| LlmError::Streaming(e.to_string()))?;
            let text = String::from_utf8_lossy(&chunk);
            buffer.push_str(&text);

            // Process complete lines
            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                // Parse SSE event
                if line.starts_with("event: ") {
                    let _event_type = &line[7..];
                    continue;
                }

                if line.starts_with("data: ") {
                    let data = &line[6..];

                    if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                        match event.event_type.as_str() {
                            "content_block_start" => {
                                if let Some(content_block) = &event.content_block {
                                    if content_block.block_type == "tool_use" {
                                        current_tool_use_id = content_block.id.clone().unwrap_or_default();
                                        current_tool_name = content_block.name.clone().unwrap_or_default();

                                        // Send tool call start
                                        let delta = ToolCallDelta::new(tool_call_index)
                                            .with_id(&current_tool_use_id)
                                            .with_name(&current_tool_name);

                                        if sender.send(StreamChunk::tool_call(delta)).await.is_err() {
                                            return Ok(());
                                        }

                                        tool_call_index += 1;
                                    }
                                }
                            }
                            "content_block_delta" => {
                                if let Some(delta) = &event.delta {
                                    match delta.delta_type.as_str() {
                                        "text_delta" => {
                                            if let Some(text) = &delta.text {
                                                if sender.send(StreamChunk::text(text)).await.is_err() {
                                                    return Ok(());
                                                }
                                            }
                                        }
                                        "input_json_delta" => {
                                            if let Some(partial_json) = &delta.partial_json {
                                                let delta = ToolCallDelta::new(tool_call_index.saturating_sub(1))
                                                    .with_arguments(partial_json);

                                                if sender.send(StreamChunk::tool_call(delta)).await.is_err() {
                                                    return Ok(());
                                                }
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                            }
                            "message_delta" => {
                                if let Some(delta) = &event.delta {
                                    if let Some(stop_reason) = &delta.stop_reason {
                                        let finish_reason = match stop_reason.as_str() {
                                            "end_turn" | "stop_sequence" => FinishReason::Stop,
                                            "max_tokens" => FinishReason::Length,
                                            "tool_use" => FinishReason::ToolCalls,
                                            _ => FinishReason::Stop,
                                        };

                                        let usage = event.usage.map(|u| TokenUsage {
                                            prompt_tokens: u.input_tokens.unwrap_or(0),
                                            completion_tokens: u.output_tokens.unwrap_or(0),
                                            total_tokens: u.input_tokens.unwrap_or(0) + u.output_tokens.unwrap_or(0),
                                        });

                                        if sender.send(StreamChunk::finish(finish_reason, usage)).await.is_err() {
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                            "message_stop" => {
                                return Ok(());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

// Anthropic API response types

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
    stop_reason: Option<String>,
    usage: AnthropicUsage,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
    id: Option<String>,
    name: Option<String>,
    input: Option<Value>,
}

#[derive(Debug, Deserialize)]
struct AnthropicUsage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct AnthropicStreamEvent {
    #[serde(rename = "type")]
    event_type: String,
    content_block: Option<StreamContentBlock>,
    delta: Option<StreamDelta>,
    usage: Option<StreamUsage>,
}

#[derive(Debug, Deserialize)]
struct StreamContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    id: Option<String>,
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamDelta {
    #[serde(rename = "type")]
    delta_type: String,
    text: Option<String>,
    partial_json: Option<String>,
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamUsage {
    input_tokens: Option<u32>,
    output_tokens: Option<u32>,
}

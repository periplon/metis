//! OpenAI LLM Provider with streaming support

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

/// OpenAI LLM Provider
pub struct OpenAiProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    model: String,
    default_temperature: Option<f32>,
    default_max_tokens: Option<u32>,
}

impl OpenAiProvider {
    /// Create a new OpenAI provider from configuration
    pub fn new(config: &LlmProviderConfig) -> LlmResult<Self> {
        let api_key = if let Some(env_var) = &config.api_key_env {
            env::var(env_var).map_err(|_| {
                LlmError::Authentication(format!(
                    "Environment variable {} not set",
                    env_var
                ))
            })?
        } else {
            env::var("OPENAI_API_KEY").map_err(|_| {
                LlmError::Authentication("OPENAI_API_KEY environment variable not set".to_string())
            })?
        };

        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            base_url,
            model: config.model.clone(),
            default_temperature: config.temperature,
            default_max_tokens: config.max_tokens,
        })
    }

    /// Create a new OpenAI provider using secrets store for API key
    ///
    /// Checks the secrets store first, then falls back to environment variables.
    pub async fn new_with_secrets(
        config: &LlmProviderConfig,
        secrets: SharedSecretsStore,
    ) -> LlmResult<Self> {
        let env_var = config.api_key_env.as_deref().unwrap_or("OPENAI_API_KEY");

        let api_key = secrets.get_or_env(env_var).await.ok_or_else(|| {
            LlmError::Authentication(format!(
                "API key not found in secrets store or environment variable {}",
                env_var
            ))
        })?;

        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://api.openai.com/v1".to_string());

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            base_url,
            model: config.model.clone(),
            default_temperature: config.temperature,
            default_max_tokens: config.max_tokens,
        })
    }

    /// Build the request body for OpenAI API
    fn build_request_body(&self, request: &CompletionRequest) -> Value {
        let mut body = json!({
            "model": request.model.as_ref().unwrap_or(&self.model),
            "messages": self.convert_messages(&request.messages),
        });

        if let Some(temp) = request.temperature.or(self.default_temperature) {
            body["temperature"] = json!(temp);
        }

        if let Some(max_tokens) = request.max_tokens.or(self.default_max_tokens) {
            body["max_tokens"] = json!(max_tokens);
        }

        if let Some(stop) = &request.stop {
            body["stop"] = json!(stop);
        }

        if let Some(tools) = &request.tools {
            if !tools.is_empty() {
                body["tools"] = json!(tools.iter().map(|t| {
                    // Ensure parameters is a valid JSON Schema object
                    // OpenAI requires at minimum {"type": "object"} for function parameters
                    let params = if t.parameters.is_null() || t.parameters.as_object().map_or(true, |o| o.is_empty()) {
                        json!({
                            "type": "object",
                            "properties": {},
                            "required": []
                        })
                    } else if t.parameters.get("type").is_none() {
                        // Add "type": "object" if missing
                        let mut p = t.parameters.clone();
                        if let Some(obj) = p.as_object_mut() {
                            obj.insert("type".to_string(), json!("object"));
                        }
                        p
                    } else {
                        t.parameters.clone()
                    };
                    json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": params
                        }
                    })
                }).collect::<Vec<_>>());
            }
        }

        if let Some(tool_choice) = &request.tool_choice {
            body["tool_choice"] = match tool_choice {
                super::ToolChoice::Auto => json!("auto"),
                super::ToolChoice::None => json!("none"),
                super::ToolChoice::Required => json!("required"),
                super::ToolChoice::Tool { name } => json!({
                    "type": "function",
                    "function": { "name": name }
                }),
            };
        }

        if request.stream {
            body["stream"] = json!(true);
            body["stream_options"] = json!({ "include_usage": true });
        }

        body
    }

    /// Convert internal messages to OpenAI format
    fn convert_messages(&self, messages: &[Message]) -> Vec<Value> {
        messages
            .iter()
            .map(|m| {
                let mut msg = json!({
                    "role": match m.role {
                        Role::System => "system",
                        Role::User => "user",
                        Role::Assistant => "assistant",
                        Role::Tool => "tool",
                    },
                    "content": m.content,
                });

                if let Some(tool_calls) = &m.tool_calls {
                    msg["tool_calls"] = json!(tool_calls.iter().map(|tc| {
                        json!({
                            "id": tc.id,
                            "type": "function",
                            "function": {
                                "name": tc.name,
                                "arguments": serde_json::to_string(&tc.arguments).unwrap_or_default()
                            }
                        })
                    }).collect::<Vec<_>>());
                }

                if let Some(tool_call_id) = &m.tool_call_id {
                    msg["tool_call_id"] = json!(tool_call_id);
                }

                if let Some(name) = &m.name {
                    msg["name"] = json!(name);
                }

                msg
            })
            .collect()
    }

    /// Parse a non-streaming response
    fn parse_response(&self, response: &OpenAiResponse) -> LlmResult<CompletionResponse> {
        let choice = response.choices.first().ok_or_else(|| {
            LlmError::Parse("No choices in response".to_string())
        })?;

        let tool_calls: Vec<ToolCall> = choice
            .message
            .tool_calls
            .as_ref()
            .map(|tcs| {
                tcs.iter()
                    .map(|tc| ToolCall {
                        id: tc.id.clone(),
                        name: tc.function.name.clone(),
                        arguments: serde_json::from_str(&tc.function.arguments)
                            .unwrap_or(Value::Object(Default::default())),
                    })
                    .collect()
            })
            .unwrap_or_default();

        let message = if tool_calls.is_empty() {
            Message::assistant(choice.message.content.clone().unwrap_or_default())
        } else {
            Message::assistant_with_tools(
                choice.message.content.clone().unwrap_or_default(),
                tool_calls,
            )
        };

        let finish_reason = match choice.finish_reason.as_deref() {
            Some("stop") => FinishReason::Stop,
            Some("length") => FinishReason::Length,
            Some("tool_calls") => FinishReason::ToolCalls,
            Some("content_filter") => FinishReason::ContentFilter,
            _ => FinishReason::Stop,
        };

        let usage = response.usage.as_ref().map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });

        Ok(CompletionResponse {
            message,
            finish_reason,
            usage,
        })
    }

    /// Parse SSE data line
    #[allow(dead_code)]
    fn parse_sse_line(&self, line: &str) -> Option<StreamChunk> {
        if !line.starts_with("data: ") {
            return None;
        }

        let data = &line[6..];
        if data == "[DONE]" {
            return None;
        }

        let parsed: OpenAiStreamResponse = serde_json::from_str(data).ok()?;
        let choice = parsed.choices.first()?;

        let mut chunk = StreamChunk {
            content: choice.delta.content.clone().unwrap_or_default(),
            tool_calls: Vec::new(),
            finish_reason: None,
            usage: None,
        };

        // Handle tool calls
        if let Some(tool_calls) = &choice.delta.tool_calls {
            for tc in tool_calls {
                let mut delta = ToolCallDelta::new(tc.index);
                if let Some(id) = &tc.id {
                    delta = delta.with_id(id);
                }
                if let Some(func) = &tc.function {
                    if let Some(name) = &func.name {
                        delta = delta.with_name(name);
                    }
                    if let Some(args) = &func.arguments {
                        delta = delta.with_arguments(args);
                    }
                }
                chunk.tool_calls.push(delta);
            }
        }

        // Handle finish reason
        if let Some(reason) = &choice.finish_reason {
            chunk.finish_reason = Some(match reason.as_str() {
                "stop" => FinishReason::Stop,
                "length" => FinishReason::Length,
                "tool_calls" => FinishReason::ToolCalls,
                "content_filter" => FinishReason::ContentFilter,
                _ => FinishReason::Stop,
            });
        }

        // Handle usage (in final chunk with stream_options)
        if let Some(usage) = &parsed.usage {
            chunk.usage = Some(TokenUsage {
                prompt_tokens: usage.prompt_tokens,
                completion_tokens: usage.completion_tokens,
                total_tokens: usage.total_tokens,
            });
        }

        Some(chunk)
    }
}

#[async_trait]
impl LlmProvider for OpenAiProvider {
    fn name(&self) -> &str {
        "openai"
    }

    fn model(&self) -> &str {
        &self.model
    }

    async fn complete(&self, request: CompletionRequest) -> LlmResult<CompletionResponse> {
        let body = self.build_request_body(&request);

        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
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

        let openai_response: OpenAiResponse = response.json().await.map_err(|e| {
            LlmError::Parse(format!("Failed to parse response: {}", e))
        })?;

        self.parse_response(&openai_response)
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
        // Use tiktoken for accurate token counting
        // For now, use a simple approximation (4 chars per token)
        // TODO: Integrate tiktoken-rs properly
        (text.len() / 4) as u32
    }

    fn context_window(&self) -> u32 {
        // Return context window based on model
        match self.model.as_str() {
            m if m.contains("gpt-4-turbo") || m.contains("gpt-4o") => 128000,
            m if m.contains("gpt-4-32k") => 32768,
            m if m.contains("gpt-4") => 8192,
            m if m.contains("gpt-3.5-turbo-16k") => 16384,
            m if m.contains("gpt-3.5-turbo") => 4096,
            _ => 8192,
        }
    }

    fn max_output_tokens(&self) -> u32 {
        match self.model.as_str() {
            m if m.contains("gpt-4o") => 16384,
            m if m.contains("gpt-4-turbo") => 4096,
            _ => 4096,
        }
    }
}

impl OpenAiProvider {
    async fn stream_completion(
        client: reqwest::Client,
        api_key: String,
        base_url: String,
        body: Value,
        sender: LlmStreamSender,
    ) -> LlmResult<()> {
        let response = client
            .post(format!("{}/chat/completions", base_url))
            .header("Authorization", format!("Bearer {}", api_key))
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

                if line.starts_with("data: ") {
                    let data = &line[6..];
                    if data == "[DONE]" {
                        return Ok(());
                    }

                    if let Ok(parsed) = serde_json::from_str::<OpenAiStreamResponse>(data) {
                        if let Some(choice) = parsed.choices.first() {
                            let mut chunk = StreamChunk {
                                content: choice.delta.content.clone().unwrap_or_default(),
                                tool_calls: Vec::new(),
                                finish_reason: None,
                                usage: None,
                            };

                            // Handle tool calls
                            if let Some(tool_calls) = &choice.delta.tool_calls {
                                for tc in tool_calls {
                                    let mut delta = ToolCallDelta::new(tc.index);
                                    if let Some(id) = &tc.id {
                                        delta = delta.with_id(id);
                                    }
                                    if let Some(func) = &tc.function {
                                        if let Some(name) = &func.name {
                                            delta = delta.with_name(name);
                                        }
                                        if let Some(args) = &func.arguments {
                                            delta = delta.with_arguments(args);
                                        }
                                    }
                                    chunk.tool_calls.push(delta);
                                }
                            }

                            // Handle finish reason
                            if let Some(reason) = &choice.finish_reason {
                                chunk.finish_reason = Some(match reason.as_str() {
                                    "stop" => FinishReason::Stop,
                                    "length" => FinishReason::Length,
                                    "tool_calls" => FinishReason::ToolCalls,
                                    "content_filter" => FinishReason::ContentFilter,
                                    _ => FinishReason::Stop,
                                });
                            }

                            // Handle usage
                            if let Some(usage) = &parsed.usage {
                                chunk.usage = Some(TokenUsage {
                                    prompt_tokens: usage.prompt_tokens,
                                    completion_tokens: usage.completion_tokens,
                                    total_tokens: usage.total_tokens,
                                });
                            }

                            if sender.send(chunk).await.is_err() {
                                return Ok(()); // Receiver dropped
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

// OpenAI API response types

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAiToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OpenAiToolCall {
    id: String,
    function: OpenAiFunction,
}

#[derive(Debug, Deserialize)]
struct OpenAiFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamResponse {
    choices: Vec<OpenAiStreamChoice>,
    usage: Option<OpenAiUsage>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamChoice {
    delta: OpenAiDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAiDelta {
    content: Option<String>,
    tool_calls: Option<Vec<OpenAiStreamToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamToolCall {
    index: usize,
    id: Option<String>,
    function: Option<OpenAiStreamFunction>,
}

#[derive(Debug, Deserialize)]
struct OpenAiStreamFunction {
    name: Option<String>,
    arguments: Option<String>,
}

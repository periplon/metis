//! Groq LLM Provider with streaming support
//!
//! Groq provides ultra-fast inference for open-source LLMs using their LPU hardware.
//! Uses OpenAI-compatible API format.

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

/// Default Groq API base URL
const GROQ_API_BASE_URL: &str = "https://api.groq.com/openai/v1";

/// Default environment variable for Groq API key
const GROQ_API_KEY_ENV: &str = "GROQ_API_KEY";

/// Groq LLM Provider
///
/// Provides access to Groq's ultra-fast inference API for models like:
/// - llama-3.3-70b-versatile (131K context, 32K output)
/// - llama-3.1-8b-instant (131K context)
/// - mixtral-8x7b-32768 (32K context)
/// - gemma2-9b-it (8K context)
pub struct GroqProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    model: String,
    default_temperature: Option<f32>,
    default_max_tokens: Option<u32>,
}

impl GroqProvider {
    /// Create a new Groq provider from configuration
    ///
    /// API key is read from the environment variable specified in config,
    /// or defaults to `GROQ_API_KEY`.
    pub fn new(config: &LlmProviderConfig) -> LlmResult<Self> {
        let env_var = config.api_key_env.as_deref().unwrap_or(GROQ_API_KEY_ENV);

        let api_key = env::var(env_var).map_err(|_| {
            LlmError::Authentication(format!("Environment variable {} not set", env_var))
        })?;

        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| GROQ_API_BASE_URL.to_string());

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            base_url,
            model: config.model.clone(),
            default_temperature: config.temperature,
            default_max_tokens: config.max_tokens,
        })
    }

    /// Create a new Groq provider using secrets store for API key
    ///
    /// Checks the secrets store first for API keys, then falls back to
    /// environment variables if not found.
    pub async fn new_with_secrets(
        config: &LlmProviderConfig,
        secrets: SharedSecretsStore,
    ) -> LlmResult<Self> {
        let env_var = config.api_key_env.as_deref().unwrap_or(GROQ_API_KEY_ENV);

        let api_key = secrets.get_or_env(env_var).await.ok_or_else(|| {
            LlmError::Authentication(format!(
                "API key not found in secrets store or environment variable {}",
                env_var
            ))
        })?;

        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| GROQ_API_BASE_URL.to_string());

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            base_url,
            model: config.model.clone(),
            default_temperature: config.temperature,
            default_max_tokens: config.max_tokens,
        })
    }

    /// Build the request body for Groq API (OpenAI-compatible format)
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
                    let params = if t.parameters.is_null() || t.parameters.as_object().map_or(true, |o| o.is_empty()) {
                        json!({
                            "type": "object",
                            "properties": {},
                            "required": []
                        })
                    } else {
                        let mut p = t.parameters.clone();
                        if let Some(obj) = p.as_object_mut() {
                            if !obj.contains_key("type") {
                                obj.insert("type".to_string(), json!("object"));
                            }
                            if !obj.contains_key("properties") {
                                obj.insert("properties".to_string(), json!({}));
                            }
                        }
                        p
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

    /// Convert internal messages to OpenAI/Groq format
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
    fn parse_response(&self, response: &GroqResponse) -> LlmResult<CompletionResponse> {
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

    /// Stream completion from Groq API
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

            // Process complete SSE lines
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

                    if let Ok(parsed) = serde_json::from_str::<GroqStreamResponse>(data) {
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

                            // Handle usage (in final chunk with stream_options)
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

#[async_trait]
impl LlmProvider for GroqProvider {
    fn name(&self) -> &str {
        "groq"
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

        let groq_response: GroqResponse = response.json().await.map_err(|e| {
            LlmError::Parse(format!("Failed to parse Groq response: {}", e))
        })?;

        self.parse_response(&groq_response)
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
        // Approximate token count: ~4 characters per token
        // Groq uses various tokenizers depending on the model
        (text.len() / 4) as u32
    }

    fn context_window(&self) -> u32 {
        // Return context window based on model
        match self.model.as_str() {
            // Llama 3.x models - 128K context
            m if m.contains("llama-3.3") || m.contains("llama-3.1") => 131072,
            // Llama 4 models - 128K context
            m if m.contains("llama-4") => 131072,
            // Mixtral - 32K context
            m if m.contains("mixtral") => 32768,
            // Gemma models - 8K context
            m if m.contains("gemma") => 8192,
            // Qwen models - 128K context
            m if m.contains("qwen") => 131072,
            // Compound models - 128K context
            m if m.contains("compound") => 131072,
            // Default conservative estimate
            _ => 8192,
        }
    }

    fn max_output_tokens(&self) -> u32 {
        // Return max output tokens based on model
        match self.model.as_str() {
            // Llama 3.1 8B - same as context (for instant model)
            m if m.contains("llama-3.1-8b") => 131072,
            // Llama 3.3 70B - 32K output
            m if m.contains("llama-3.3-70b") => 32768,
            // Llama 4 models - 8K output
            m if m.contains("llama-4") => 8192,
            // Mixtral - 4K output
            m if m.contains("mixtral") => 4096,
            // Compound models - 8K output
            m if m.contains("compound") => 8192,
            // Qwen - 40K output
            m if m.contains("qwen") => 40960,
            // Default
            _ => 4096,
        }
    }
}

// Groq API response types (OpenAI-compatible)

#[derive(Debug, Deserialize)]
struct GroqResponse {
    choices: Vec<GroqChoice>,
    usage: Option<GroqUsage>,
}

#[derive(Debug, Deserialize)]
struct GroqChoice {
    message: GroqMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GroqMessage {
    content: Option<String>,
    tool_calls: Option<Vec<GroqToolCall>>,
}

#[derive(Debug, Deserialize)]
struct GroqToolCall {
    id: String,
    function: GroqFunction,
}

#[derive(Debug, Deserialize)]
struct GroqFunction {
    name: String,
    arguments: String,
}

#[derive(Debug, Deserialize)]
struct GroqUsage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct GroqStreamResponse {
    choices: Vec<GroqStreamChoice>,
    usage: Option<GroqUsage>,
}

#[derive(Debug, Deserialize)]
struct GroqStreamChoice {
    delta: GroqDelta,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GroqDelta {
    content: Option<String>,
    tool_calls: Option<Vec<GroqStreamToolCall>>,
}

#[derive(Debug, Deserialize)]
struct GroqStreamToolCall {
    index: usize,
    id: Option<String>,
    function: Option<GroqStreamFunction>,
}

#[derive(Debug, Deserialize)]
struct GroqStreamFunction {
    name: Option<String>,
    arguments: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::domain::ToolDefinition;

    /// Helper to create a minimal GroqProvider for testing
    /// Note: This bypasses API key requirement for unit testing
    fn create_test_provider(model: &str) -> GroqProvider {
        GroqProvider {
            client: reqwest::Client::new(),
            api_key: "test_key".to_string(),
            base_url: GROQ_API_BASE_URL.to_string(),
            model: model.to_string(),
            default_temperature: Some(0.7),
            default_max_tokens: Some(1024),
        }
    }

    #[test]
    fn test_context_window_models() {
        // Llama 3.3 - 131K context
        let provider = create_test_provider("llama-3.3-70b-versatile");
        assert_eq!(provider.context_window(), 131072);

        // Llama 3.1 - 131K context
        let provider = create_test_provider("llama-3.1-8b-instant");
        assert_eq!(provider.context_window(), 131072);

        // Mixtral - 32K context
        let provider = create_test_provider("mixtral-8x7b-32768");
        assert_eq!(provider.context_window(), 32768);

        // Gemma - 8K context
        let provider = create_test_provider("gemma2-9b-it");
        assert_eq!(provider.context_window(), 8192);

        // Unknown model - default 8K
        let provider = create_test_provider("unknown-model");
        assert_eq!(provider.context_window(), 8192);
    }

    #[test]
    fn test_max_output_tokens_models() {
        // Llama 3.1 8B - 131K output
        let provider = create_test_provider("llama-3.1-8b-instant");
        assert_eq!(provider.max_output_tokens(), 131072);

        // Llama 3.3 70B - 32K output
        let provider = create_test_provider("llama-3.3-70b-versatile");
        assert_eq!(provider.max_output_tokens(), 32768);

        // Mixtral - 4K output
        let provider = create_test_provider("mixtral-8x7b-32768");
        assert_eq!(provider.max_output_tokens(), 4096);

        // Unknown model - default 4K
        let provider = create_test_provider("unknown-model");
        assert_eq!(provider.max_output_tokens(), 4096);
    }

    #[test]
    fn test_provider_name() {
        let provider = create_test_provider("llama-3.3-70b-versatile");
        assert_eq!(provider.name(), "groq");
    }

    #[test]
    fn test_provider_model() {
        let provider = create_test_provider("llama-3.3-70b-versatile");
        assert_eq!(provider.model(), "llama-3.3-70b-versatile");
    }

    #[test]
    fn test_build_request_body_basic() {
        let provider = create_test_provider("llama-3.3-70b-versatile");

        let request = CompletionRequest {
            messages: vec![
                Message::system("You are a helpful assistant"),
                Message::user("Hello!"),
            ],
            model: None,
            temperature: None,
            max_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
            stream: false,
        };

        let body = provider.build_request_body(&request);

        assert_eq!(body["model"], "llama-3.3-70b-versatile");
        assert!(body["messages"].is_array());
        assert_eq!(body["messages"].as_array().unwrap().len(), 2);
        // Should use default temperature from provider (use approximate comparison for f32)
        let temp = body["temperature"].as_f64().unwrap();
        assert!((temp - 0.7).abs() < 0.001);
        // Should use default max_tokens from provider
        assert_eq!(body["max_tokens"], 1024);
    }

    #[test]
    fn test_build_request_body_with_overrides() {
        let provider = create_test_provider("llama-3.3-70b-versatile");

        let request = CompletionRequest {
            messages: vec![Message::user("Hello!")],
            model: Some("llama-3.1-8b-instant".to_string()),
            temperature: Some(0.5),
            max_tokens: Some(2048),
            tools: None,
            tool_choice: None,
            stop: Some(vec!["STOP".to_string()]),
            stream: false,
        };

        let body = provider.build_request_body(&request);

        // Should use overridden model
        assert_eq!(body["model"], "llama-3.1-8b-instant");
        // Should use request temperature, not default
        assert_eq!(body["temperature"], 0.5);
        // Should use request max_tokens, not default
        assert_eq!(body["max_tokens"], 2048);
        // Should include stop sequences
        assert_eq!(body["stop"][0], "STOP");
    }

    #[test]
    fn test_build_request_body_streaming() {
        let provider = create_test_provider("llama-3.3-70b-versatile");

        let request = CompletionRequest {
            messages: vec![Message::user("Hello!")],
            model: None,
            temperature: None,
            max_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
            stream: true,
        };

        let body = provider.build_request_body(&request);

        assert_eq!(body["stream"], true);
        assert!(body["stream_options"]["include_usage"].as_bool().unwrap());
    }

    #[test]
    fn test_build_request_body_with_tools() {
        let provider = create_test_provider("llama-3.3-70b-versatile");

        let request = CompletionRequest {
            messages: vec![Message::user("What's the weather?")],
            model: None,
            temperature: None,
            max_tokens: None,
            tools: Some(vec![ToolDefinition {
                name: "get_weather".to_string(),
                description: "Get current weather".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "location": { "type": "string" }
                    },
                    "required": ["location"]
                }),
            }]),
            tool_choice: None,
            stop: None,
            stream: false,
        };

        let body = provider.build_request_body(&request);

        assert!(body["tools"].is_array());
        let tools = body["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0]["type"], "function");
        assert_eq!(tools[0]["function"]["name"], "get_weather");
    }

    #[test]
    fn test_convert_messages_all_roles() {
        let provider = create_test_provider("llama-3.3-70b-versatile");

        let messages = vec![
            Message::system("System prompt"),
            Message::user("User message"),
            Message::assistant("Assistant response"),
        ];

        let converted = provider.convert_messages(&messages);

        assert_eq!(converted.len(), 3);
        assert_eq!(converted[0]["role"], "system");
        assert_eq!(converted[0]["content"], "System prompt");
        assert_eq!(converted[1]["role"], "user");
        assert_eq!(converted[1]["content"], "User message");
        assert_eq!(converted[2]["role"], "assistant");
        assert_eq!(converted[2]["content"], "Assistant response");
    }

    #[test]
    fn test_parse_response_basic() {
        let provider = create_test_provider("llama-3.3-70b-versatile");

        let groq_response = GroqResponse {
            choices: vec![GroqChoice {
                message: GroqMessage {
                    content: Some("Hello! How can I help you?".to_string()),
                    tool_calls: None,
                },
                finish_reason: Some("stop".to_string()),
            }],
            usage: Some(GroqUsage {
                prompt_tokens: 10,
                completion_tokens: 8,
                total_tokens: 18,
            }),
        };

        let response = provider.parse_response(&groq_response).unwrap();

        assert_eq!(response.message.content, "Hello! How can I help you?");
        assert_eq!(response.finish_reason, FinishReason::Stop);
        assert!(response.usage.is_some());
        let usage = response.usage.unwrap();
        assert_eq!(usage.prompt_tokens, 10);
        assert_eq!(usage.completion_tokens, 8);
        assert_eq!(usage.total_tokens, 18);
    }

    #[test]
    fn test_parse_response_with_tool_calls() {
        let provider = create_test_provider("llama-3.3-70b-versatile");

        let groq_response = GroqResponse {
            choices: vec![GroqChoice {
                message: GroqMessage {
                    content: None,
                    tool_calls: Some(vec![GroqToolCall {
                        id: "call_123".to_string(),
                        function: GroqFunction {
                            name: "get_weather".to_string(),
                            arguments: r#"{"location":"London"}"#.to_string(),
                        },
                    }]),
                },
                finish_reason: Some("tool_calls".to_string()),
            }],
            usage: None,
        };

        let response = provider.parse_response(&groq_response).unwrap();

        assert_eq!(response.finish_reason, FinishReason::ToolCalls);
        assert!(!response.message.tool_calls.as_ref().unwrap().is_empty());
        let tool_call = &response.message.tool_calls.as_ref().unwrap()[0];
        assert_eq!(tool_call.id, "call_123");
        assert_eq!(tool_call.name, "get_weather");
    }

    #[test]
    fn test_parse_response_empty_choices() {
        let provider = create_test_provider("llama-3.3-70b-versatile");

        let groq_response = GroqResponse {
            choices: vec![],
            usage: None,
        };

        let result = provider.parse_response(&groq_response);
        assert!(result.is_err());
    }

    #[test]
    fn test_count_tokens_approximation() {
        let provider = create_test_provider("llama-3.3-70b-versatile");

        // ~4 chars per token approximation
        let text = "Hello, world!"; // 13 chars -> 13/4 = 3
        let tokens = provider.count_tokens(text);
        assert_eq!(tokens, 3);

        // "This is a longer piece of text for testing." = 44 chars -> 44/4 = 11
        // But let's verify by actually counting
        let longer_text = "This is a longer piece of text for testing.";
        let expected = (longer_text.len() / 4) as u32;
        let tokens = provider.count_tokens(longer_text);
        assert_eq!(tokens, expected);
    }

    #[test]
    fn test_default_constants() {
        assert_eq!(GROQ_API_BASE_URL, "https://api.groq.com/openai/v1");
        assert_eq!(GROQ_API_KEY_ENV, "GROQ_API_KEY");
    }
}

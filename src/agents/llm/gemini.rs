//! Google Gemini LLM Provider with streaming support

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

/// Google Gemini LLM Provider
pub struct GeminiProvider {
    client: reqwest::Client,
    api_key: String,
    base_url: String,
    model: String,
    default_temperature: Option<f32>,
    default_max_tokens: Option<u32>,
}

impl GeminiProvider {
    /// Create a new Gemini provider from configuration
    pub fn new(config: &LlmProviderConfig) -> LlmResult<Self> {
        let api_key = if let Some(env_var) = &config.api_key_env {
            env::var(env_var).map_err(|_| {
                LlmError::Authentication(format!(
                    "Environment variable {} not set",
                    env_var
                ))
            })?
        } else {
            env::var("GEMINI_API_KEY").map_err(|_| {
                LlmError::Authentication("GEMINI_API_KEY environment variable not set".to_string())
            })?
        };

        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".to_string());

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            base_url,
            model: config.model.clone(),
            default_temperature: config.temperature,
            default_max_tokens: config.max_tokens,
        })
    }

    /// Create a new Gemini provider using secrets store for API key
    pub async fn new_with_secrets(
        config: &LlmProviderConfig,
        secrets: SharedSecretsStore,
    ) -> LlmResult<Self> {
        let env_var = config.api_key_env.as_deref().unwrap_or("GEMINI_API_KEY");

        let api_key = secrets.get_or_env(env_var).await.ok_or_else(|| {
            LlmError::Authentication(format!(
                "API key not found in secrets store or environment variable {}",
                env_var
            ))
        })?;

        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "https://generativelanguage.googleapis.com/v1beta".to_string());

        Ok(Self {
            client: reqwest::Client::new(),
            api_key,
            base_url,
            model: config.model.clone(),
            default_temperature: config.temperature,
            default_max_tokens: config.max_tokens,
        })
    }

    /// Build the request body for Gemini API
    fn build_request_body(&self, request: &CompletionRequest) -> Value {
        let mut body = json!({
            "contents": self.convert_messages(&request.messages),
        });

        // Generation config
        let mut generation_config = json!({});

        if let Some(temp) = request.temperature.or(self.default_temperature) {
            generation_config["temperature"] = json!(temp);
        }

        if let Some(max_tokens) = request.max_tokens.or(self.default_max_tokens) {
            generation_config["maxOutputTokens"] = json!(max_tokens);
        }

        if let Some(stop) = &request.stop {
            generation_config["stopSequences"] = json!(stop);
        }

        if generation_config.as_object().map_or(false, |o| !o.is_empty()) {
            body["generationConfig"] = generation_config;
        }

        // Tools (function declarations)
        if let Some(tools) = &request.tools {
            if !tools.is_empty() {
                body["tools"] = json!([{
                    "function_declarations": tools.iter().map(|t| {
                        json!({
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters
                        })
                    }).collect::<Vec<_>>()
                }]);
            }
        }

        // Tool choice
        if let Some(tool_choice) = &request.tool_choice {
            body["tool_config"] = match tool_choice {
                super::ToolChoice::Auto => json!({
                    "function_calling_config": { "mode": "AUTO" }
                }),
                super::ToolChoice::None => json!({
                    "function_calling_config": { "mode": "NONE" }
                }),
                super::ToolChoice::Required => json!({
                    "function_calling_config": { "mode": "ANY" }
                }),
                super::ToolChoice::Tool { name } => json!({
                    "function_calling_config": {
                        "mode": "ANY",
                        "allowed_function_names": [name]
                    }
                }),
            };
        }

        body
    }

    /// Convert internal messages to Gemini format
    fn convert_messages(&self, messages: &[Message]) -> Vec<Value> {
        let mut contents = Vec::new();
        let mut system_instruction: Option<String> = None;

        for m in messages {
            match m.role {
                Role::System => {
                    // Gemini handles system prompts differently - prepend to first user message
                    // or use systemInstruction field
                    system_instruction = Some(m.content.clone());
                }
                Role::User => {
                    let mut parts = vec![json!({ "text": m.content })];

                    // Include system instruction in first user message if present
                    if let Some(sys) = system_instruction.take() {
                        parts.insert(0, json!({ "text": format!("[System Instructions]\n{}\n\n", sys) }));
                    }

                    contents.push(json!({
                        "role": "user",
                        "parts": parts
                    }));
                }
                Role::Assistant => {
                    let mut parts = Vec::new();

                    if !m.content.is_empty() {
                        parts.push(json!({ "text": m.content }));
                    }

                    // Handle tool calls (function calls in Gemini)
                    if let Some(tool_calls) = &m.tool_calls {
                        for tc in tool_calls {
                            parts.push(json!({
                                "functionCall": {
                                    "name": tc.name,
                                    "args": tc.arguments
                                }
                            }));
                        }
                    }

                    if !parts.is_empty() {
                        contents.push(json!({
                            "role": "model",
                            "parts": parts
                        }));
                    }
                }
                Role::Tool => {
                    // Tool results in Gemini format
                    let tool_name = m.name.clone().unwrap_or_else(|| "tool".to_string());
                    let response_value: Value = serde_json::from_str(&m.content)
                        .unwrap_or_else(|_| json!({ "result": m.content }));

                    contents.push(json!({
                        "role": "user",
                        "parts": [{
                            "functionResponse": {
                                "name": tool_name,
                                "response": response_value
                            }
                        }]
                    }));
                }
            }
        }

        contents
    }

    /// Parse a non-streaming response
    fn parse_response(&self, response: &GeminiResponse) -> LlmResult<CompletionResponse> {
        let candidate = response.candidates.first().ok_or_else(|| {
            LlmError::Parse("No candidates in response".to_string())
        })?;

        let mut content = String::new();
        let mut tool_calls = Vec::new();

        if let Some(parts) = &candidate.content.parts {
            for (index, part) in parts.iter().enumerate() {
                if let Some(text) = &part.text {
                    content.push_str(text);
                }
                if let Some(fc) = &part.function_call {
                    tool_calls.push(ToolCall {
                        id: format!("call_{}", index),
                        name: fc.name.clone(),
                        arguments: fc.args.clone().unwrap_or(Value::Object(Default::default())),
                    });
                }
            }
        }

        let message = if tool_calls.is_empty() {
            Message::assistant(content)
        } else {
            Message::assistant_with_tools(content, tool_calls)
        };

        let finish_reason = match candidate.finish_reason.as_deref() {
            Some("STOP") => FinishReason::Stop,
            Some("MAX_TOKENS") => FinishReason::Length,
            Some("SAFETY") => FinishReason::ContentFilter,
            Some("RECITATION") => FinishReason::ContentFilter,
            Some("OTHER") => FinishReason::Stop,
            _ => FinishReason::Stop,
        };

        let usage = response.usage_metadata.as_ref().map(|u| TokenUsage {
            prompt_tokens: u.prompt_token_count.unwrap_or(0),
            completion_tokens: u.candidates_token_count.unwrap_or(0),
            total_tokens: u.total_token_count.unwrap_or(0),
        });

        Ok(CompletionResponse {
            message,
            finish_reason,
            usage,
        })
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    fn name(&self) -> &str {
        "gemini"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn supports_tools(&self) -> bool {
        true
    }

    async fn complete(&self, request: CompletionRequest) -> LlmResult<CompletionResponse> {
        let body = self.build_request_body(&request);
        let model = request.model.as_ref().unwrap_or(&self.model);

        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url, model, self.api_key
        );

        let response = self
            .client
            .post(&url)
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

        let gemini_response: GeminiResponse = response.json().await.map_err(|e| {
            LlmError::Parse(format!("Failed to parse response: {}", e))
        })?;

        self.parse_response(&gemini_response)
    }

    fn complete_stream(&self, request: CompletionRequest) -> LlmStream {
        let (sender, stream) = LlmStream::channel(64);

        let client = self.client.clone();
        let api_key = self.api_key.clone();
        let base_url = self.base_url.clone();
        let model = request.model.clone().unwrap_or_else(|| self.model.clone());
        let body = self.build_request_body(&request);

        tokio::spawn(async move {
            let result = Self::stream_completion(client, api_key, base_url, model, body, sender.clone()).await;
            if let Err(e) = result {
                let _ = sender.send_error(e).await;
            }
        });

        stream
    }

    fn count_tokens(&self, text: &str) -> u32 {
        // Gemini uses a similar tokenization to other models
        // Approximate: ~4 characters per token
        (text.len() / 4) as u32
    }

    fn context_window(&self) -> u32 {
        match self.model.as_str() {
            // Gemini 3.0 models (1M input)
            m if m.contains("gemini-3-pro") => 1048576,
            // Gemini 2.0 models
            m if m.contains("gemini-2.0-flash") => 1048576,
            // Gemini 1.5 models
            m if m.contains("gemini-1.5-pro") => 2097152, // 2M tokens
            m if m.contains("gemini-1.5-flash") => 1048576, // 1M tokens
            // Legacy
            m if m.contains("gemini-pro") => 32768,
            _ => 32768,
        }
    }

    fn max_output_tokens(&self) -> u32 {
        match self.model.as_str() {
            // Gemini 3.0 models (64K output)
            m if m.contains("gemini-3-pro") => 65536,
            // Gemini 2.0 and 1.5 models
            m if m.contains("gemini-2.0-flash") => 8192,
            m if m.contains("gemini-1.5-pro") => 8192,
            m if m.contains("gemini-1.5-flash") => 8192,
            _ => 8192,
        }
    }
}

impl GeminiProvider {
    async fn stream_completion(
        client: reqwest::Client,
        api_key: String,
        base_url: String,
        model: String,
        body: Value,
        sender: LlmStreamSender,
    ) -> LlmResult<()> {
        let url = format!(
            "{}/models/{}:streamGenerateContent?key={}&alt=sse",
            base_url, model, api_key
        );

        let response = client
            .post(&url)
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
        let mut tool_call_index = 0usize;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| LlmError::Streaming(e.to_string()))?;
            let text = String::from_utf8_lossy(&chunk);
            buffer.push_str(&text);

            // Process complete lines (SSE format)
            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                // Gemini SSE format: data: {...}
                if line.starts_with("data: ") {
                    let data = &line[6..];

                    if let Ok(parsed) = serde_json::from_str::<GeminiStreamResponse>(data) {
                        if let Some(candidate) = parsed.candidates.and_then(|c| c.into_iter().next()) {
                            let mut stream_chunk = StreamChunk {
                                content: String::new(),
                                tool_calls: Vec::new(),
                                finish_reason: None,
                                usage: None,
                            };

                            // Process parts
                            if let Some(content) = candidate.content {
                                if let Some(parts) = content.parts {
                                    for part in parts {
                                        // Handle text content
                                        if let Some(text) = part.text {
                                            stream_chunk.content.push_str(&text);
                                        }

                                        // Handle function calls
                                        if let Some(fc) = part.function_call {
                                            let delta = ToolCallDelta::new(tool_call_index)
                                                .with_id(&format!("call_{}", tool_call_index))
                                                .with_name(&fc.name)
                                                .with_arguments(&serde_json::to_string(&fc.args.unwrap_or(Value::Null)).unwrap_or_default());
                                            stream_chunk.tool_calls.push(delta);
                                            tool_call_index += 1;
                                        }
                                    }
                                }
                            }

                            // Handle finish reason
                            if let Some(reason) = candidate.finish_reason {
                                stream_chunk.finish_reason = Some(match reason.as_str() {
                                    "STOP" => FinishReason::Stop,
                                    "MAX_TOKENS" => FinishReason::Length,
                                    "SAFETY" => FinishReason::ContentFilter,
                                    "RECITATION" => FinishReason::ContentFilter,
                                    _ => FinishReason::Stop,
                                });
                            }

                            // Handle usage metadata
                            if let Some(usage) = parsed.usage_metadata {
                                stream_chunk.usage = Some(TokenUsage {
                                    prompt_tokens: usage.prompt_token_count.unwrap_or(0),
                                    completion_tokens: usage.candidates_token_count.unwrap_or(0),
                                    total_tokens: usage.total_token_count.unwrap_or(0),
                                });
                            }

                            // Send chunk if it has content or is a finish
                            if !stream_chunk.content.is_empty()
                                || !stream_chunk.tool_calls.is_empty()
                                || stream_chunk.finish_reason.is_some()
                                || stream_chunk.usage.is_some()
                            {
                                if sender.send(stream_chunk).await.is_err() {
                                    return Ok(()); // Receiver dropped
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

// Gemini API response types

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiCandidate {
    content: GeminiContent,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiContent {
    parts: Option<Vec<GeminiPart>>,
    #[allow(dead_code)]
    role: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiPart {
    text: Option<String>,
    function_call: Option<GeminiFunctionCall>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiFunctionCall {
    name: String,
    args: Option<Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiUsageMetadata {
    prompt_token_count: Option<u32>,
    candidates_token_count: Option<u32>,
    total_token_count: Option<u32>,
}

// Streaming response types

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiStreamResponse {
    candidates: Option<Vec<GeminiStreamCandidate>>,
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeminiStreamCandidate {
    content: Option<GeminiContent>,
    finish_reason: Option<String>,
}

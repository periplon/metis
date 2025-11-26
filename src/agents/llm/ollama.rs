//! Ollama LLM Provider with streaming support (for local models)

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::{json, Value};

use super::{
    CompletionRequest, CompletionResponse, FinishReason, LlmProvider, LlmStream, LlmStreamSender,
    StreamChunk, TokenUsage,
};
use crate::agents::config::LlmProviderConfig;
use crate::agents::domain::{Message, Role};
use crate::agents::error::{LlmError, LlmResult};

/// Ollama LLM Provider (for local models)
pub struct OllamaProvider {
    client: reqwest::Client,
    base_url: String,
    model: String,
    default_temperature: Option<f32>,
    default_max_tokens: Option<u32>,
}

impl OllamaProvider {
    /// Create a new Ollama provider from configuration
    pub fn new(config: &LlmProviderConfig) -> LlmResult<Self> {
        let base_url = config
            .base_url
            .clone()
            .unwrap_or_else(|| "http://localhost:11434".to_string());

        Ok(Self {
            client: reqwest::Client::new(),
            base_url,
            model: config.model.clone(),
            default_temperature: config.temperature,
            default_max_tokens: config.max_tokens,
        })
    }

    fn convert_messages(&self, messages: &[Message]) -> Vec<Value> {
        messages
            .iter()
            .map(|m| {
                json!({
                    "role": match m.role {
                        Role::System => "system",
                        Role::User => "user",
                        Role::Assistant => "assistant",
                        Role::Tool => "tool",
                    },
                    "content": m.content
                })
            })
            .collect()
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn supports_tools(&self) -> bool {
        // Ollama tool support depends on the model
        false
    }

    async fn complete(&self, request: CompletionRequest) -> LlmResult<CompletionResponse> {
        let body = json!({
            "model": request.model.as_ref().unwrap_or(&self.model),
            "messages": self.convert_messages(&request.messages),
            "stream": false,
            "options": {
                "temperature": request.temperature.or(self.default_temperature),
                "num_predict": request.max_tokens.or(self.default_max_tokens),
            }
        });

        let response = self
            .client
            .post(format!("{}/api/chat", self.base_url))
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

        let ollama_response: OllamaResponse = response.json().await.map_err(|e| {
            LlmError::Parse(format!("Failed to parse response: {}", e))
        })?;

        let message = Message::assistant(ollama_response.message.content);

        Ok(CompletionResponse {
            message,
            finish_reason: if ollama_response.done {
                FinishReason::Stop
            } else {
                FinishReason::Length
            },
            usage: Some(TokenUsage {
                prompt_tokens: ollama_response.prompt_eval_count.unwrap_or(0),
                completion_tokens: ollama_response.eval_count.unwrap_or(0),
                total_tokens: ollama_response.prompt_eval_count.unwrap_or(0)
                    + ollama_response.eval_count.unwrap_or(0),
            }),
        })
    }

    fn complete_stream(&self, request: CompletionRequest) -> LlmStream {
        let (sender, stream) = LlmStream::channel(64);

        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let model = request.model.clone().unwrap_or_else(|| self.model.clone());
        let messages = self.convert_messages(&request.messages);
        let temperature = request.temperature.or(self.default_temperature);
        let max_tokens = request.max_tokens.or(self.default_max_tokens);

        tokio::spawn(async move {
            let result = Self::stream_completion(
                client, base_url, model, messages, temperature, max_tokens, sender.clone()
            ).await;
            if let Err(e) = result {
                let _ = sender.send_error(e).await;
            }
        });

        stream
    }

    fn count_tokens(&self, text: &str) -> u32 {
        // Approximate token count
        (text.len() / 4) as u32
    }

    fn context_window(&self) -> u32 {
        // Depends on the model, default to a common value
        4096
    }

    fn max_output_tokens(&self) -> u32 {
        2048
    }
}

impl OllamaProvider {
    async fn stream_completion(
        client: reqwest::Client,
        base_url: String,
        model: String,
        messages: Vec<Value>,
        temperature: Option<f32>,
        max_tokens: Option<u32>,
        sender: LlmStreamSender,
    ) -> LlmResult<()> {
        let body = json!({
            "model": model,
            "messages": messages,
            "stream": true,
            "options": {
                "temperature": temperature,
                "num_predict": max_tokens,
            }
        });

        let response = client
            .post(format!("{}/api/chat", base_url))
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

            // Ollama streams NDJSON (one JSON object per line)
            while let Some(pos) = buffer.find('\n') {
                let line = buffer[..pos].trim().to_string();
                buffer = buffer[pos + 1..].to_string();

                if line.is_empty() {
                    continue;
                }

                if let Ok(stream_response) = serde_json::from_str::<OllamaStreamResponse>(&line) {
                    if let Some(content) = &stream_response.message.content {
                        if !content.is_empty() {
                            if sender.send(StreamChunk::text(content)).await.is_err() {
                                return Ok(());
                            }
                        }
                    }

                    if stream_response.done {
                        let usage = Some(TokenUsage {
                            prompt_tokens: stream_response.prompt_eval_count.unwrap_or(0),
                            completion_tokens: stream_response.eval_count.unwrap_or(0),
                            total_tokens: stream_response.prompt_eval_count.unwrap_or(0)
                                + stream_response.eval_count.unwrap_or(0),
                        });

                        if sender.send(StreamChunk::finish(FinishReason::Stop, usage)).await.is_err() {
                            return Ok(());
                        }
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
    done: bool,
    prompt_eval_count: Option<u32>,
    eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OllamaMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaStreamResponse {
    message: OllamaStreamMessage,
    done: bool,
    prompt_eval_count: Option<u32>,
    eval_count: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct OllamaStreamMessage {
    content: Option<String>,
}

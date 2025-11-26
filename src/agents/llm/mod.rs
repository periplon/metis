//! LLM Provider implementations with streaming support
//!
//! This module provides a unified interface for interacting with various LLM providers:
//! - OpenAI (GPT-4, GPT-3.5)
//! - Anthropic (Sonnet)
//! - Google Gemini
//! - Ollama (local models)
//! - Azure OpenAI

mod stream;
mod openai;
mod anthropic;
mod gemini;
mod ollama;
mod azure;

pub use stream::*;
pub use openai::OpenAiProvider;
pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;
pub use ollama::OllamaProvider;
pub use azure::AzureOpenAiProvider;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::agents::config::{LlmProviderConfig, LlmProviderType};
use crate::agents::domain::{Message, ToolDefinition};
use crate::agents::error::LlmResult;

/// Trait for LLM providers
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Get the provider name
    fn name(&self) -> &str;

    /// Get the model being used
    fn model(&self) -> &str;

    /// Check if streaming is supported
    fn supports_streaming(&self) -> bool {
        true
    }

    /// Check if tool/function calling is supported
    fn supports_tools(&self) -> bool {
        true
    }

    /// Complete a request (non-streaming)
    async fn complete(&self, request: CompletionRequest) -> LlmResult<CompletionResponse>;

    /// Complete a request with streaming
    fn complete_stream(&self, request: CompletionRequest) -> LlmStream;

    /// Count tokens in a text string
    fn count_tokens(&self, text: &str) -> u32;

    /// Count tokens in messages
    fn count_message_tokens(&self, messages: &[Message]) -> u32 {
        messages.iter().map(|m| self.count_tokens(&m.content) + 4).sum::<u32>() + 3
    }

    /// Get the context window size for the model
    fn context_window(&self) -> u32;

    /// Get the maximum output tokens for the model
    fn max_output_tokens(&self) -> u32;
}

/// Request for LLM completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionRequest {
    /// Messages in the conversation
    pub messages: Vec<Message>,
    /// Model to use (overrides provider default)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Temperature for sampling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Maximum tokens to generate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// Tools available for calling
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<ToolDefinition>>,
    /// Tool choice mode
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,
    /// Stop sequences
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// Whether to stream the response
    #[serde(default)]
    pub stream: bool,
}

impl Default for CompletionRequest {
    fn default() -> Self {
        Self {
            messages: Vec::new(),
            model: None,
            temperature: None,
            max_tokens: None,
            tools: None,
            tool_choice: None,
            stop: None,
            stream: false,
        }
    }
}

/// Tool choice mode
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    /// Let the model decide
    Auto,
    /// Don't use tools
    None,
    /// Must use a tool
    Required,
    /// Use a specific tool
    Tool { name: String },
}

/// Response from LLM completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionResponse {
    /// Generated message
    pub message: Message,
    /// Reason the completion stopped
    pub finish_reason: FinishReason,
    /// Token usage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
}

/// Reason completion stopped
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FinishReason {
    /// Natural stop
    Stop,
    /// Hit max tokens
    Length,
    /// Tool call requested
    ToolCalls,
    /// Content filtered
    ContentFilter,
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

use crate::adapters::secrets::SharedSecretsStore;

/// Create an LLM provider from configuration
pub fn create_provider(config: &LlmProviderConfig) -> LlmResult<Arc<dyn LlmProvider>> {
    match config.provider {
        LlmProviderType::OpenAI => {
            let provider = OpenAiProvider::new(config)?;
            Ok(Arc::new(provider))
        }
        LlmProviderType::Anthropic => {
            let provider = AnthropicProvider::new(config)?;
            Ok(Arc::new(provider))
        }
        LlmProviderType::Gemini => {
            let provider = GeminiProvider::new(config)?;
            Ok(Arc::new(provider))
        }
        LlmProviderType::Ollama => {
            let provider = OllamaProvider::new(config)?;
            Ok(Arc::new(provider))
        }
        LlmProviderType::AzureOpenAI => {
            let provider = AzureOpenAiProvider::new(config)?;
            Ok(Arc::new(provider))
        }
    }
}

/// Create an LLM provider from configuration, using secrets store for API keys
///
/// This function checks the secrets store first for API keys, then falls back to
/// environment variables if not found. This allows keys to be set via the UI
/// and stored in memory without persisting to disk.
pub async fn create_provider_with_secrets(
    config: &LlmProviderConfig,
    secrets: SharedSecretsStore,
) -> LlmResult<Arc<dyn LlmProvider>> {
    match config.provider {
        LlmProviderType::OpenAI => {
            let provider = OpenAiProvider::new_with_secrets(config, secrets).await?;
            Ok(Arc::new(provider))
        }
        LlmProviderType::Anthropic => {
            let provider = AnthropicProvider::new_with_secrets(config, secrets).await?;
            Ok(Arc::new(provider))
        }
        LlmProviderType::Gemini => {
            let provider = GeminiProvider::new_with_secrets(config, secrets).await?;
            Ok(Arc::new(provider))
        }
        LlmProviderType::Ollama => {
            // Ollama doesn't require an API key
            let provider = OllamaProvider::new(config)?;
            Ok(Arc::new(provider))
        }
        LlmProviderType::AzureOpenAI => {
            let provider = AzureOpenAiProvider::new_with_secrets(config, secrets).await?;
            Ok(Arc::new(provider))
        }
    }
}

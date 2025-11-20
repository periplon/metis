use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Deserialize)]
pub struct SamplingRequest {
    pub method: String,
    pub params: SamplingParams,
}

#[derive(Debug, Deserialize)]
pub struct SamplingParams {
    pub messages: Vec<SamplingMessage>,
    pub model_preferences: Option<ModelPreferences>,
    pub system_prompt: Option<String>,
    pub include_context: Option<String>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub stop_sequences: Option<Vec<String>>,
    pub metadata: Option<Value>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SamplingMessage {
    pub role: String,
    pub content: SamplingContent,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum SamplingContent {
    Text(String),
    MultiPart(Vec<ContentPart>),
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ContentPart {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: Option<String>,
    pub data: Option<String>,
    pub mime_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ModelPreferences {
    pub hints: Option<Vec<ModelHint>>,
    pub cost_priority: Option<f32>,
    pub speed_priority: Option<f32>,
    pub intelligence_priority: Option<f32>,
}

#[derive(Debug, Deserialize)]
pub struct ModelHint {
    pub name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SamplingResult {
    pub role: String,
    pub content: SamplingContent,
    pub model: String,
    pub stop_reason: Option<String>,
}

#[async_trait]
pub trait SamplingPort: Send + Sync {
    async fn create_message(&self, params: SamplingParams) -> anyhow::Result<SamplingResult>;
}

use crate::domain::sampling::{SamplingContent, SamplingParams, SamplingPort, SamplingResult};
use async_trait::async_trait;

pub struct MockSamplingHandler;

impl MockSamplingHandler {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MockSamplingHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SamplingPort for MockSamplingHandler {
    async fn create_message(&self, params: SamplingParams) -> anyhow::Result<SamplingResult> {
        // Mock implementation: echo back the last user message with a prefix
        let last_message = params.messages.last()
            .ok_or_else(|| anyhow::anyhow!("No messages provided"))?;
        
        let response_text = match &last_message.content {
            SamplingContent::Text(text) => {
                format!("Mock response to: {}", text)
            }
            SamplingContent::MultiPart(parts) => {
                let text_parts: Vec<String> = parts.iter()
                    .filter_map(|p| p.text.clone())
                    .collect();
                format!("Mock response to: {}", text_parts.join(" "))
            }
        };

        Ok(SamplingResult {
            role: "assistant".to_string(),
            content: SamplingContent::Text(response_text),
            model: "mock-model-1.0".to_string(),
            stop_reason: Some("end_turn".to_string()),
        })
    }
}

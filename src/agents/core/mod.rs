//! Core agent implementations
//!
//! Provides different agent types:
//! - SingleTurnAgent: One request → one response
//! - MultiTurnAgent: Maintains conversation history
//! - ReActAgent: Reasoning + Action loop with tool calling

mod single_turn;
mod multi_turn;
mod react;

pub use single_turn::SingleTurnAgent;
pub use multi_turn::MultiTurnAgent;
pub use react::ReActAgent;

use std::sync::Arc;

use serde_json::Value;
use tera::{Context, Tera};

use crate::agents::config::AgentConfig;
use crate::agents::domain::AgentType;
use crate::agents::error::AgentResult;
use crate::agents::llm::LlmProvider;
use crate::agents::memory::ConversationStore;
use crate::domain::ToolPort;

/// Render the system prompt as a Tera template with the input values
///
/// This allows system prompts to use template variables like:
/// ```text
/// You are a {{role}} assistant helping with {{task_type}}.
/// The user's name is {{user_name}}.
/// ```
///
/// Input values are available as template variables. Falls back to
/// the original system prompt if rendering fails.
pub fn render_system_prompt(system_prompt: &str, input: &Value) -> String {
    // If there's nothing that looks like a template, return as-is
    if !system_prompt.contains("{{") {
        return system_prompt.to_string();
    }

    // Build Tera context from input JSON
    let mut context = Context::new();

    if let Some(obj) = input.as_object() {
        for (key, value) in obj {
            // Insert values based on their type
            match value {
                Value::String(s) => {
                    context.insert(key, s);
                }
                Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        context.insert(key, &i);
                    } else if let Some(f) = n.as_f64() {
                        context.insert(key, &f);
                    }
                }
                Value::Bool(b) => {
                    context.insert(key, b);
                }
                Value::Array(_) | Value::Object(_) => {
                    // For complex types, insert as JSON string
                    context.insert(key, &value.to_string());
                }
                Value::Null => {
                    context.insert(key, &"");
                }
            }
        }
    }

    // Render the template
    match Tera::one_off(system_prompt, &context, false) {
        Ok(rendered) => rendered,
        Err(e) => {
            tracing::warn!("Failed to render system prompt template: {}", e);
            system_prompt.to_string()
        }
    }
}

/// Render the user prompt from a template and input values
///
/// If `prompt_template` is provided, renders it with input values as template variables.
/// Otherwise, extracts the "prompt" field from input, or auto-generates a prompt from
/// structured input fields.
///
/// Example template: "Analyze the topic '{{topic}}' for {{audience}} audience."
pub fn render_user_prompt(prompt_template: Option<&str>, input: &Value) -> String {
    match prompt_template {
        Some(template) if !template.is_empty() => {
            // Build Tera context from input JSON
            let mut context = Context::new();

            if let Some(obj) = input.as_object() {
                for (key, value) in obj {
                    match value {
                        Value::String(s) => {
                            context.insert(key, s);
                        }
                        Value::Number(n) => {
                            if let Some(i) = n.as_i64() {
                                context.insert(key, &i);
                            } else if let Some(f) = n.as_f64() {
                                context.insert(key, &f);
                            }
                        }
                        Value::Bool(b) => {
                            context.insert(key, b);
                        }
                        Value::Array(_) | Value::Object(_) => {
                            context.insert(key, &value.to_string());
                        }
                        Value::Null => {
                            context.insert(key, &"");
                        }
                    }
                }
            }

            // Render the template
            match Tera::one_off(template, &context, false) {
                Ok(rendered) => rendered,
                Err(e) => {
                    tracing::warn!("Failed to render prompt template: {}", e);
                    // Fallback to prompt field or auto-generate
                    fallback_prompt(input)
                }
            }
        }
        _ => {
            // No template - try prompt field first, then auto-generate from structured input
            fallback_prompt(input)
        }
    }
}

/// Generate a fallback prompt from input
/// First tries the "prompt" field, then auto-generates from structured fields
fn fallback_prompt(input: &Value) -> String {
    // First try the prompt field
    if let Some(prompt) = input.get("prompt").and_then(|v| v.as_str()) {
        if !prompt.is_empty() {
            return prompt.to_string();
        }
    }

    // Auto-generate from structured input fields
    if let Some(obj) = input.as_object() {
        let fields: Vec<String> = obj
            .iter()
            .filter(|(k, _)| *k != "session_id") // Skip session_id
            .map(|(k, v)| {
                let value_str = match v {
                    Value::String(s) => s.clone(),
                    Value::Null => String::new(),
                    _ => serde_json::to_string(v).unwrap_or_default(),
                };
                format!("{}: {}", k, value_str)
            })
            .collect();

        if !fields.is_empty() {
            return fields.join("\n");
        }
    }

    String::new()
}

/// Trait for executable agents
pub trait Agent: Send + Sync {
    /// Get the agent's configuration
    fn config(&self) -> &AgentConfig;

    /// Execute the agent
    fn execute(
        &self,
        input: serde_json::Value,
        session_id: Option<String>,
    ) -> crate::agents::domain::AgentStream;
}

/// Create an agent from configuration
pub fn create_agent(
    config: AgentConfig,
    llm_provider: Arc<dyn LlmProvider>,
    memory_store: Arc<dyn ConversationStore>,
    tool_handler: Arc<dyn ToolPort>,
) -> AgentResult<Arc<dyn Agent>> {
    match config.agent_type {
        AgentType::SingleTurn => {
            let agent = SingleTurnAgent::new(config, llm_provider);
            Ok(Arc::new(agent))
        }
        AgentType::MultiTurn => {
            let agent = MultiTurnAgent::new(config, llm_provider, memory_store);
            Ok(Arc::new(agent))
        }
        AgentType::ReAct => {
            let agent = ReActAgent::new(config, llm_provider, memory_store, tool_handler);
            Ok(Arc::new(agent))
        }
    }
}

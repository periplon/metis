//! Message and conversation types

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use super::ToolCall;

/// Message role in a conversation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// System message (instructions to the LLM)
    System,
    /// User message
    User,
    /// Assistant (LLM) message
    Assistant,
    /// Tool result message
    Tool,
}

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::System => write!(f, "system"),
            Role::User => write!(f, "user"),
            Role::Assistant => write!(f, "assistant"),
            Role::Tool => write!(f, "tool"),
        }
    }
}

/// A message in a conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role of the message sender
    pub role: Role,
    /// Message content (text)
    pub content: String,
    /// Tool calls made by the assistant (if any)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// ID of the tool call this message is responding to
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Optional name for the message sender
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl Message {
    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    /// Create an assistant message with tool calls
    pub fn assistant_with_tools(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            tool_calls: if tool_calls.is_empty() {
                None
            } else {
                Some(tool_calls)
            },
            tool_call_id: None,
            name: None,
        }
    }

    /// Create a tool result message
    pub fn tool_result(tool_call_id: impl Into<String>, result: &Value) -> Self {
        Self {
            role: Role::Tool,
            content: serde_json::to_string(result).unwrap_or_else(|_| "{}".to_string()),
            tool_calls: None,
            tool_call_id: Some(tool_call_id.into()),
            name: None,
        }
    }
}

/// A conversation session containing message history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSession {
    /// Unique session identifier
    pub session_id: String,
    /// Agent name this session belongs to
    pub agent_name: String,
    /// Message history
    pub messages: Vec<Message>,
    /// Arbitrary metadata attached to the session
    #[serde(default)]
    pub metadata: HashMap<String, Value>,
    /// Session creation timestamp (Unix epoch milliseconds)
    pub created_at: u64,
    /// Last update timestamp (Unix epoch milliseconds)
    pub updated_at: u64,
}

impl ConversationSession {
    /// Create a new conversation session
    pub fn new(session_id: String, agent_name: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            session_id,
            agent_name,
            messages: Vec::new(),
            metadata: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Add a message to the session
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
    }

    /// Get the number of messages
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get a preview of the last message
    pub fn last_message_preview(&self, max_len: usize) -> Option<String> {
        self.messages.last().map(|m| {
            if m.content.len() > max_len {
                format!("{}...", &m.content[..max_len])
            } else {
                m.content.clone()
            }
        })
    }

    /// Convert to a session summary
    pub fn to_summary(&self) -> super::SessionSummary {
        super::SessionSummary {
            session_id: self.session_id.clone(),
            agent_name: self.agent_name.clone(),
            message_count: self.messages.len(),
            created_at: self.created_at,
            updated_at: self.updated_at,
            last_message_preview: self.last_message_preview(100),
        }
    }
}

//! Agent response and streaming types

use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

use super::{ToolCall, ToolCallResult};

/// Final response from an agent execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentResponse {
    /// Output content/result
    pub output: Value,
    /// Tool calls made during execution (if any)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCallResult>,
    /// Reasoning steps (for ReAct agents)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasoning_steps: Vec<String>,
    /// Session ID (for multi-turn agents)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Number of iterations (for ReAct agents)
    #[serde(default)]
    pub iterations: u32,
    /// Token usage information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
}

impl Default for AgentResponse {
    fn default() -> Self {
        Self {
            output: Value::Null,
            tool_calls: Vec::new(),
            reasoning_steps: Vec::new(),
            session_id: None,
            iterations: 0,
            usage: None,
            execution_time_ms: 0,
        }
    }
}

/// Token usage information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Tokens used in the prompt/input
    pub prompt_tokens: u32,
    /// Tokens generated in the response
    pub completion_tokens: u32,
    /// Total tokens used
    pub total_tokens: u32,
}

/// Agent execution status
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    /// Agent is starting
    Starting,
    /// Agent is thinking/processing
    Thinking,
    /// Agent is calling a tool
    CallingTool { tool_name: String },
    /// Agent received tool result
    ToolResultReceived { tool_name: String },
    /// Agent is generating response
    Generating,
    /// Agent execution completed
    Completed,
    /// Agent execution failed
    Failed { error: String },
}

/// A chunk of streaming output from an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentChunk {
    /// Streamed text content
    Text { content: String },
    /// Tool call being initiated
    ToolCall {
        id: String,
        name: String,
        arguments: Value,
    },
    /// Tool execution result
    ToolResult {
        tool_call_id: String,
        name: String,
        result: Value,
        success: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<String>,
    },
    /// Reasoning/thought step (for ReAct)
    Thought { content: String },
    /// Status update
    Status { status: AgentStatus },
    /// Final complete response
    Complete { response: AgentResponse },
    /// Error occurred
    Error { message: String },
    /// Token usage update
    Usage { usage: TokenUsage },
}

impl AgentChunk {
    /// Create a text chunk
    pub fn text(content: impl Into<String>) -> Self {
        Self::Text {
            content: content.into(),
        }
    }

    /// Create a tool call chunk
    pub fn tool_call(tool_call: &ToolCall) -> Self {
        Self::ToolCall {
            id: tool_call.id.clone(),
            name: tool_call.name.clone(),
            arguments: tool_call.arguments.clone(),
        }
    }

    /// Create a tool result chunk
    pub fn tool_result(result: &ToolCallResult) -> Self {
        Self::ToolResult {
            tool_call_id: result.tool_call_id.clone(),
            name: result.tool_name.clone(),
            result: result.output.clone(),
            success: result.success,
            error: result.error.clone(),
        }
    }

    /// Create a thought chunk
    pub fn thought(content: impl Into<String>) -> Self {
        Self::Thought {
            content: content.into(),
        }
    }

    /// Create a status chunk
    pub fn status(status: AgentStatus) -> Self {
        Self::Status { status }
    }

    /// Create a complete chunk
    pub fn complete(response: AgentResponse) -> Self {
        Self::Complete { response }
    }

    /// Create an error chunk
    pub fn error(message: impl Into<String>) -> Self {
        Self::Error {
            message: message.into(),
        }
    }

    /// Create a usage chunk
    pub fn usage(usage: TokenUsage) -> Self {
        Self::Usage { usage }
    }
}

/// Streaming response from an agent
pub struct AgentStream {
    receiver: mpsc::Receiver<Result<AgentChunk, crate::agents::error::AgentError>>,
}

impl AgentStream {
    /// Create a new agent stream from a channel receiver
    pub fn new(receiver: mpsc::Receiver<Result<AgentChunk, crate::agents::error::AgentError>>) -> Self {
        Self { receiver }
    }

    /// Create a channel pair for building an agent stream
    pub fn channel(buffer: usize) -> (AgentStreamSender, Self) {
        let (tx, rx) = mpsc::channel(buffer);
        (AgentStreamSender { sender: tx }, Self { receiver: rx })
    }

    /// Collect all chunks into a final response
    pub async fn collect(mut self) -> Result<AgentResponse, crate::agents::error::AgentError> {
        let mut final_response: Option<AgentResponse> = None;
        let mut text_content = String::new();
        let mut tool_calls = Vec::new();
        let mut reasoning_steps = Vec::new();

        while let Some(result) = self.receiver.recv().await {
            match result {
                Ok(chunk) => match chunk {
                    AgentChunk::Text { content } => {
                        text_content.push_str(&content);
                    }
                    AgentChunk::ToolResult {
                        tool_call_id,
                        name,
                        result,
                        success,
                        error,
                    } => {
                        tool_calls.push(ToolCallResult {
                            tool_call_id,
                            tool_name: name,
                            input: Value::Null,
                            output: result,
                            execution_time_ms: 0,
                            success,
                            error,
                        });
                    }
                    AgentChunk::Thought { content } => {
                        reasoning_steps.push(content);
                    }
                    AgentChunk::Complete { response } => {
                        final_response = Some(response);
                    }
                    AgentChunk::Error { message } => {
                        return Err(crate::agents::error::AgentError::Execution(message));
                    }
                    _ => {}
                },
                Err(e) => return Err(e),
            }
        }

        if let Some(response) = final_response {
            Ok(response)
        } else {
            // Build response from accumulated chunks
            Ok(AgentResponse {
                output: serde_json::json!({ "content": text_content }),
                tool_calls,
                reasoning_steps,
                session_id: None,
                iterations: 0,
                usage: None,
                execution_time_ms: 0,
            })
        }
    }
}

impl Stream for AgentStream {
    type Item = Result<AgentChunk, crate::agents::error::AgentError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.receiver).poll_recv(cx)
    }
}

/// Sender half for building an agent stream
pub struct AgentStreamSender {
    sender: mpsc::Sender<Result<AgentChunk, crate::agents::error::AgentError>>,
}

impl AgentStreamSender {
    /// Send a chunk
    pub async fn send(&self, chunk: AgentChunk) -> Result<(), mpsc::error::SendError<Result<AgentChunk, crate::agents::error::AgentError>>> {
        self.sender.send(Ok(chunk)).await
    }

    /// Send an error
    pub async fn send_error(&self, error: crate::agents::error::AgentError) -> Result<(), mpsc::error::SendError<Result<AgentChunk, crate::agents::error::AgentError>>> {
        self.sender.send(Err(error)).await
    }

    /// Check if the receiver is closed
    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

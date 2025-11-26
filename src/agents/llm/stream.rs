//! Streaming types for LLM responses

use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::pin::Pin;
use std::task::{Context, Poll};
use tokio::sync::mpsc;

use crate::agents::domain::ToolCall;
use crate::agents::error::LlmError;

/// A chunk of streamed LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamChunk {
    /// Delta content (text being generated)
    #[serde(default)]
    pub content: String,
    /// Tool calls being made (partial or complete)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCallDelta>,
    /// Finish reason (if this is the final chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<super::FinishReason>,
    /// Token usage (usually only in final chunk)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<super::TokenUsage>,
}

impl StreamChunk {
    /// Create a text content chunk
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            tool_calls: Vec::new(),
            finish_reason: None,
            usage: None,
        }
    }

    /// Create a tool call chunk
    pub fn tool_call(delta: ToolCallDelta) -> Self {
        Self {
            content: String::new(),
            tool_calls: vec![delta],
            finish_reason: None,
            usage: None,
        }
    }

    /// Create a finish chunk
    pub fn finish(reason: super::FinishReason, usage: Option<super::TokenUsage>) -> Self {
        Self {
            content: String::new(),
            tool_calls: Vec::new(),
            finish_reason: Some(reason),
            usage,
        }
    }

    /// Check if this chunk has content
    pub fn has_content(&self) -> bool {
        !self.content.is_empty()
    }

    /// Check if this is a final chunk
    pub fn is_final(&self) -> bool {
        self.finish_reason.is_some()
    }
}

/// Delta update for a tool call (streaming tool calls)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallDelta {
    /// Index of the tool call being updated
    pub index: usize,
    /// Tool call ID (may be partial)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Tool name (may be partial)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Arguments JSON string (partial, accumulated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

impl ToolCallDelta {
    /// Create a new tool call delta
    pub fn new(index: usize) -> Self {
        Self {
            index,
            id: None,
            name: None,
            arguments: None,
        }
    }

    /// Set the ID
    pub fn with_id(mut self, id: impl Into<String>) -> Self {
        self.id = Some(id.into());
        self
    }

    /// Set the name
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// Set the arguments
    pub fn with_arguments(mut self, args: impl Into<String>) -> Self {
        self.arguments = Some(args.into());
        self
    }
}

/// Accumulator for building tool calls from streaming deltas
#[derive(Debug, Default)]
pub struct ToolCallAccumulator {
    tool_calls: Vec<ToolCallBuilder>,
}

#[derive(Debug, Default)]
struct ToolCallBuilder {
    id: String,
    name: String,
    arguments: String,
}

impl ToolCallAccumulator {
    /// Create a new accumulator
    pub fn new() -> Self {
        Self {
            tool_calls: Vec::new(),
        }
    }

    /// Apply a delta update
    pub fn apply_delta(&mut self, delta: &ToolCallDelta) {
        // Ensure we have enough builders
        while self.tool_calls.len() <= delta.index {
            self.tool_calls.push(ToolCallBuilder::default());
        }

        let builder = &mut self.tool_calls[delta.index];

        if let Some(id) = &delta.id {
            builder.id.push_str(id);
        }
        if let Some(name) = &delta.name {
            builder.name.push_str(name);
        }
        if let Some(args) = &delta.arguments {
            builder.arguments.push_str(args);
        }
    }

    /// Build the final tool calls
    pub fn build(self) -> Vec<ToolCall> {
        self.tool_calls
            .into_iter()
            .filter(|b| !b.id.is_empty() && !b.name.is_empty())
            .map(|b| ToolCall {
                id: b.id,
                name: b.name,
                arguments: serde_json::from_str(&b.arguments).unwrap_or(Value::Object(Default::default())),
            })
            .collect()
    }

    /// Check if any tool calls are being built
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}

/// Streaming response from an LLM provider
pub struct LlmStream {
    receiver: mpsc::Receiver<Result<StreamChunk, LlmError>>,
}

impl LlmStream {
    /// Create a new LLM stream from a channel receiver
    pub fn new(receiver: mpsc::Receiver<Result<StreamChunk, LlmError>>) -> Self {
        Self { receiver }
    }

    /// Create a channel pair for building an LLM stream
    pub fn channel(buffer: usize) -> (LlmStreamSender, Self) {
        let (tx, rx) = mpsc::channel(buffer);
        (LlmStreamSender { sender: tx }, Self { receiver: rx })
    }

    /// Collect all chunks into a complete response
    pub async fn collect(mut self) -> Result<super::CompletionResponse, LlmError> {
        let mut content = String::new();
        let mut tool_accumulator = ToolCallAccumulator::new();
        let mut finish_reason = None;
        let mut usage = None;

        while let Some(result) = self.receiver.recv().await {
            let chunk = result?;

            content.push_str(&chunk.content);

            for delta in &chunk.tool_calls {
                tool_accumulator.apply_delta(delta);
            }

            if let Some(reason) = chunk.finish_reason {
                finish_reason = Some(reason);
            }

            if chunk.usage.is_some() {
                usage = chunk.usage;
            }
        }

        let tool_calls = tool_accumulator.build();
        let message = if tool_calls.is_empty() {
            crate::agents::domain::Message::assistant(content)
        } else {
            crate::agents::domain::Message::assistant_with_tools(content, tool_calls)
        };

        Ok(super::CompletionResponse {
            message,
            finish_reason: finish_reason.unwrap_or(super::FinishReason::Stop),
            usage,
        })
    }
}

impl Stream for LlmStream {
    type Item = Result<StreamChunk, LlmError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.receiver).poll_recv(cx)
    }
}

/// Sender half for building an LLM stream
#[derive(Clone)]
pub struct LlmStreamSender {
    sender: mpsc::Sender<Result<StreamChunk, LlmError>>,
}

impl LlmStreamSender {
    /// Send a chunk
    pub async fn send(&self, chunk: StreamChunk) -> Result<(), mpsc::error::SendError<Result<StreamChunk, LlmError>>> {
        self.sender.send(Ok(chunk)).await
    }

    /// Send an error
    pub async fn send_error(&self, error: LlmError) -> Result<(), mpsc::error::SendError<Result<StreamChunk, LlmError>>> {
        self.sender.send(Err(error)).await
    }

    /// Send text content
    pub async fn send_text(&self, text: impl Into<String>) -> Result<(), mpsc::error::SendError<Result<StreamChunk, LlmError>>> {
        self.send(StreamChunk::text(text)).await
    }

    /// Send finish
    pub async fn send_finish(&self, reason: super::FinishReason, usage: Option<super::TokenUsage>) -> Result<(), mpsc::error::SendError<Result<StreamChunk, LlmError>>> {
        self.send(StreamChunk::finish(reason, usage)).await
    }

    /// Check if the receiver is closed
    pub fn is_closed(&self) -> bool {
        self.sender.is_closed()
    }
}

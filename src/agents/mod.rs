//! AI Agent System for Metis
//!
//! This module provides a sophisticated AI agent system supporting:
//! - Single-turn agents (one request → one response)
//! - Multi-turn conversational agents (maintain history)
//! - ReAct agents (reasoning + action loops with tool calling)
//! - Multi-agent orchestration (sequential, hierarchical, collaborative)
//!
//! ## Architecture
//!
//! - `domain/` - Core types (Message, Conversation, AgentChunk)
//! - `llm/` - LLM provider implementations with streaming
//! - `core/` - Agent implementations (SingleTurn, MultiTurn, ReAct)
//! - `orchestration/` - Multi-agent patterns
//! - `memory/` - Persistence backends

pub mod config;
pub mod domain;
pub mod error;
pub mod handler;
pub mod llm;
pub mod memory;
pub mod core;
pub mod orchestration;
pub mod token;

// Re-export commonly used types
pub use config::*;
pub use domain::*;
pub use error::*;
pub use handler::AgentHandler;

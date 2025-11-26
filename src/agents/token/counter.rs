//! Token counting utilities

use std::collections::HashMap;
use std::sync::RwLock;

use crate::agents::domain::Message;

/// Token counter with caching
pub struct TokenCounter {
    /// Cache of text hash -> token count
    cache: RwLock<HashMap<u64, u32>>,
    /// Approximate chars per token (varies by model)
    chars_per_token: f32,
}

impl TokenCounter {
    /// Create a new token counter
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            chars_per_token: 4.0, // Default approximation
        }
    }

    /// Create with a specific chars-per-token ratio
    pub fn with_ratio(chars_per_token: f32) -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
            chars_per_token,
        }
    }

    /// Count tokens in text (approximate)
    pub fn count(&self, text: &str) -> u32 {
        let hash = Self::hash_text(text);

        // Check cache
        if let Some(&count) = self.cache.read().unwrap().get(&hash) {
            return count;
        }

        // Calculate (approximate)
        let count = (text.len() as f32 / self.chars_per_token).ceil() as u32;

        // Cache result
        self.cache.write().unwrap().insert(hash, count);

        count
    }

    /// Count tokens in a message (including role overhead)
    pub fn count_message(&self, message: &Message) -> u32 {
        let content_tokens = self.count(&message.content);

        // Add overhead for role and formatting
        // This varies by model, using OpenAI's typical overhead
        let role_overhead = 4u32; // ~4 tokens for role markers

        // Add tool call tokens if present
        let tool_tokens = message.tool_calls.as_ref().map_or(0, |calls| {
            calls.iter().map(|tc| {
                self.count(&tc.name) + self.count(&tc.arguments.to_string()) + 10
            }).sum()
        });

        content_tokens + role_overhead + tool_tokens
    }

    /// Count tokens in multiple messages
    pub fn count_messages(&self, messages: &[Message]) -> u32 {
        let message_tokens: u32 = messages.iter().map(|m| self.count_message(m)).sum();

        // Add conversation overhead (~3 tokens)
        message_tokens + 3
    }

    /// Hash text for caching
    fn hash_text(text: &str) -> u64 {
        use std::hash::{Hash, Hasher};
        use std::collections::hash_map::DefaultHasher;

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        hasher.finish()
    }

    /// Clear the cache
    pub fn clear_cache(&self) {
        self.cache.write().unwrap().clear();
    }
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new()
    }
}

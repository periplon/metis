//! Memory management strategies for conversation history

use crate::agents::config::MemoryStrategy;
use crate::agents::domain::Message;

/// Apply a memory strategy to a list of messages
pub fn apply_strategy(messages: &[Message], strategy: &MemoryStrategy, budget_tokens: Option<u32>) -> Vec<Message> {
    match strategy {
        MemoryStrategy::Full => {
            // Keep all messages (up to budget if specified)
            if let Some(_budget) = budget_tokens {
                // TODO: Trim based on token count
                messages.to_vec()
            } else {
                messages.to_vec()
            }
        }
        MemoryStrategy::SlidingWindow { size } => {
            apply_sliding_window(messages, *size)
        }
        MemoryStrategy::FirstLast { first, last } => {
            apply_first_last(messages, *first, *last)
        }
    }
}

/// Keep only the last N messages (plus system message if present)
fn apply_sliding_window(messages: &[Message], window_size: usize) -> Vec<Message> {
    if messages.is_empty() {
        return Vec::new();
    }

    let mut result = Vec::new();

    // Always keep system message if it's first
    let start_idx = if !messages.is_empty() && matches!(messages[0].role, crate::agents::domain::Role::System) {
        result.push(messages[0].clone());
        1
    } else {
        0
    };

    // Get the last `window_size` messages (excluding system)
    let remaining = &messages[start_idx..];
    let take_from = remaining.len().saturating_sub(window_size);

    result.extend(remaining[take_from..].iter().cloned());

    result
}

/// Keep first N messages and last M messages
fn apply_first_last(messages: &[Message], first_count: usize, last_count: usize) -> Vec<Message> {
    if messages.is_empty() {
        return Vec::new();
    }

    let total = first_count + last_count;

    // If we have fewer messages than total, just return all
    if messages.len() <= total {
        return messages.to_vec();
    }

    let mut result = Vec::new();

    // Add first N messages
    result.extend(messages[..first_count].iter().cloned());

    // Add last M messages (avoiding overlap)
    let last_start = messages.len().saturating_sub(last_count);
    if last_start > first_count {
        result.extend(messages[last_start..].iter().cloned());
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agents::domain::Role;

    fn make_msg(role: Role, content: &str) -> Message {
        Message {
            role,
            content: content.to_string(),
            tool_calls: None,
            tool_call_id: None,
            name: None,
        }
    }

    #[test]
    fn test_sliding_window_basic() {
        let messages = vec![
            make_msg(Role::User, "1"),
            make_msg(Role::Assistant, "2"),
            make_msg(Role::User, "3"),
            make_msg(Role::Assistant, "4"),
            make_msg(Role::User, "5"),
        ];

        let result = apply_sliding_window(&messages, 3);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].content, "3");
        assert_eq!(result[2].content, "5");
    }

    #[test]
    fn test_sliding_window_preserves_system() {
        let messages = vec![
            make_msg(Role::System, "system"),
            make_msg(Role::User, "1"),
            make_msg(Role::Assistant, "2"),
            make_msg(Role::User, "3"),
            make_msg(Role::Assistant, "4"),
        ];

        let result = apply_sliding_window(&messages, 2);
        assert_eq!(result.len(), 3); // system + 2
        assert_eq!(result[0].role, Role::System);
        assert_eq!(result[1].content, "3");
        assert_eq!(result[2].content, "4");
    }

    #[test]
    fn test_first_last() {
        let messages = vec![
            make_msg(Role::User, "1"),
            make_msg(Role::Assistant, "2"),
            make_msg(Role::User, "3"),
            make_msg(Role::Assistant, "4"),
            make_msg(Role::User, "5"),
            make_msg(Role::Assistant, "6"),
        ];

        let result = apply_first_last(&messages, 2, 2);
        assert_eq!(result.len(), 4);
        assert_eq!(result[0].content, "1");
        assert_eq!(result[1].content, "2");
        assert_eq!(result[2].content, "5");
        assert_eq!(result[3].content, "6");
    }
}

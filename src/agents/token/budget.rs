//! Token budget management

use std::sync::atomic::{AtomicU32, Ordering};

/// Manages token budgets for agent execution
pub struct BudgetManager {
    /// Total token budget
    total: u32,
    /// Tokens consumed so far
    consumed: AtomicU32,
    /// Tokens reserved for response
    reserved: u32,
}

impl BudgetManager {
    /// Create a new budget manager
    pub fn new(total: u32, reserved_for_response: u32) -> Self {
        Self {
            total,
            consumed: AtomicU32::new(0),
            reserved: reserved_for_response,
        }
    }

    /// Create with default response reservation (4096 tokens)
    pub fn with_total(total: u32) -> Self {
        Self::new(total, 4096)
    }

    /// Check if we can afford a certain number of tokens
    pub fn can_afford(&self, tokens: u32) -> bool {
        let consumed = self.consumed.load(Ordering::Relaxed);
        consumed + tokens + self.reserved <= self.total
    }

    /// Try to consume tokens, returns false if over budget
    pub fn consume(&self, tokens: u32) -> bool {
        loop {
            let current = self.consumed.load(Ordering::Relaxed);

            if current + tokens + self.reserved > self.total {
                return false;
            }

            if self.consumed.compare_exchange(
                current,
                current + tokens,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ).is_ok() {
                return true;
            }
        }
    }

    /// Get remaining available tokens (excluding reserved)
    pub fn remaining(&self) -> u32 {
        let consumed = self.consumed.load(Ordering::Relaxed);
        self.total.saturating_sub(consumed + self.reserved)
    }

    /// Get total consumed tokens
    pub fn consumed(&self) -> u32 {
        self.consumed.load(Ordering::Relaxed)
    }

    /// Get the total budget
    pub fn total(&self) -> u32 {
        self.total
    }

    /// Reset the consumed tokens
    pub fn reset(&self) {
        self.consumed.store(0, Ordering::Relaxed);
    }

    /// Get utilization as a percentage
    pub fn utilization(&self) -> f32 {
        let consumed = self.consumed.load(Ordering::Relaxed) as f32;
        consumed / self.total as f32 * 100.0
    }
}

impl Clone for BudgetManager {
    fn clone(&self) -> Self {
        Self {
            total: self.total,
            consumed: AtomicU32::new(self.consumed.load(Ordering::Relaxed)),
            reserved: self.reserved,
        }
    }
}

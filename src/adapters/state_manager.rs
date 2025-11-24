use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct StateManager {
    state: Arc<RwLock<HashMap<String, Value>>>,
}

impl StateManager {
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn get(&self, key: &str) -> Option<Value> {
        let state = self.state.read().await;
        state.get(key).cloned()
    }

    pub async fn set(&self, key: String, value: Value) {
        let mut state = self.state.write().await;
        state.insert(key, value);
    }

    pub async fn increment(&self, key: &str) -> i64 {
        let mut state = self.state.write().await;
        let current = state
            .get(key)
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let new_value = current + 1;
        state.insert(key.to_string(), Value::Number(new_value.into()));
        new_value
    }

    pub async fn reset(&self, key: &str) {
        let mut state = self.state.write().await;
        state.remove(key);
    }

    pub async fn reset_all(&self) {
        let mut state = self.state.write().await;
        state.clear();
    }

    /// Get all state entries (for API/UI inspection)
    pub async fn get_all(&self) -> HashMap<String, Value> {
        let state = self.state.read().await;
        state.clone()
    }

    /// Clear all state (alias for reset_all)
    pub async fn clear(&self) {
        self.reset_all().await;
    }

    /// Delete a specific key
    pub async fn delete(&self, key: &str) {
        self.reset(key).await;
    }
}

impl Default for StateManager {
    fn default() -> Self {
        Self::new()
    }
}

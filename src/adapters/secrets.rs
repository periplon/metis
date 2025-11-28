//! In-memory secrets storage for API keys and credentials.
//!
//! This module provides ephemeral storage for sensitive credentials like API keys.
//! Secrets are stored in memory only and are lost when the server restarts.
//! Values are write-only - they can be set and deleted but never read back through the API.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Known secret key identifiers
pub mod keys {
    pub const OPENAI_API_KEY: &str = "OPENAI_API_KEY";
    pub const ANTHROPIC_API_KEY: &str = "ANTHROPIC_API_KEY";
    pub const GEMINI_API_KEY: &str = "GEMINI_API_KEY";
    pub const AWS_ACCESS_KEY_ID: &str = "AWS_ACCESS_KEY_ID";
    pub const AWS_SECRET_ACCESS_KEY: &str = "AWS_SECRET_ACCESS_KEY";
    pub const AWS_REGION: &str = "AWS_REGION";
}

/// In-memory secrets store
#[derive(Debug, Default)]
pub struct SecretsStore {
    secrets: RwLock<HashMap<String, String>>,
}

impl SecretsStore {
    /// Create a new empty secrets store
    pub fn new() -> Self {
        Self {
            secrets: RwLock::new(HashMap::new()),
        }
    }

    /// Set a secret value
    pub async fn set(&self, key: impl Into<String>, value: impl Into<String>) {
        let mut secrets = self.secrets.write().await;
        secrets.insert(key.into(), value.into());
    }

    /// Get a secret value (internal use only)
    pub async fn get(&self, key: &str) -> Option<String> {
        let secrets = self.secrets.read().await;
        secrets.get(key).cloned()
    }

    /// Delete a secret
    pub async fn delete(&self, key: &str) -> bool {
        let mut secrets = self.secrets.write().await;
        secrets.remove(key).is_some()
    }

    /// List all secret keys (not values)
    pub async fn list_keys(&self) -> Vec<String> {
        let secrets = self.secrets.read().await;
        secrets.keys().cloned().collect()
    }

    /// Check if a secret exists
    pub async fn exists(&self, key: &str) -> bool {
        let secrets = self.secrets.read().await;
        secrets.contains_key(key)
    }

    /// Clear all secrets
    pub async fn clear(&self) {
        let mut secrets = self.secrets.write().await;
        secrets.clear();
    }

    /// Get secret with fallback to environment variable
    pub async fn get_or_env(&self, key: &str) -> Option<String> {
        // First check in-memory store
        if let Some(value) = self.get(key).await {
            return Some(value);
        }
        // Fall back to environment variable
        std::env::var(key).ok()
    }
}

/// Thread-safe shared secrets store
pub type SharedSecretsStore = Arc<SecretsStore>;

/// Create a new shared secrets store
pub fn create_secrets_store() -> SharedSecretsStore {
    Arc::new(SecretsStore::new())
}

/// In-memory passphrase store for encrypting secrets when saving config
#[derive(Debug, Default)]
pub struct PassphraseStore {
    passphrase: RwLock<Option<String>>,
}

impl PassphraseStore {
    /// Create a new empty passphrase store
    pub fn new() -> Self {
        Self {
            passphrase: RwLock::new(None),
        }
    }

    /// Set the passphrase
    pub async fn set(&self, passphrase: impl Into<String>) {
        let mut p = self.passphrase.write().await;
        *p = Some(passphrase.into());
    }

    /// Get the passphrase
    pub async fn get(&self) -> Option<String> {
        let p = self.passphrase.read().await;
        p.clone()
    }

    /// Clear the passphrase
    pub async fn clear(&self) {
        let mut p = self.passphrase.write().await;
        *p = None;
    }

    /// Check if passphrase is set
    pub async fn is_set(&self) -> bool {
        let p = self.passphrase.read().await;
        p.is_some()
    }
}

/// Thread-safe shared passphrase store
pub type SharedPassphraseStore = Arc<PassphraseStore>;

/// Create a new shared passphrase store
pub fn create_passphrase_store() -> SharedPassphraseStore {
    Arc::new(PassphraseStore::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_set_and_get() {
        let store = SecretsStore::new();
        store.set("TEST_KEY", "test_value").await;
        assert_eq!(store.get("TEST_KEY").await, Some("test_value".to_string()));
    }

    #[tokio::test]
    async fn test_delete() {
        let store = SecretsStore::new();
        store.set("TEST_KEY", "test_value").await;
        assert!(store.delete("TEST_KEY").await);
        assert_eq!(store.get("TEST_KEY").await, None);
    }

    #[tokio::test]
    async fn test_list_keys() {
        let store = SecretsStore::new();
        store.set("KEY1", "value1").await;
        store.set("KEY2", "value2").await;
        let keys = store.list_keys().await;
        assert!(keys.contains(&"KEY1".to_string()));
        assert!(keys.contains(&"KEY2".to_string()));
    }

    #[tokio::test]
    async fn test_exists() {
        let store = SecretsStore::new();
        store.set("TEST_KEY", "test_value").await;
        assert!(store.exists("TEST_KEY").await);
        assert!(!store.exists("NONEXISTENT").await);
    }

    #[tokio::test]
    async fn test_passphrase_store() {
        let store = PassphraseStore::new();

        // Initially no passphrase
        assert!(!store.is_set().await);
        assert_eq!(store.get().await, None);

        // Set passphrase
        store.set("my-secret-passphrase").await;
        assert!(store.is_set().await);
        assert_eq!(store.get().await, Some("my-secret-passphrase".to_string()));

        // Clear passphrase
        store.clear().await;
        assert!(!store.is_set().await);
        assert_eq!(store.get().await, None);
    }
}

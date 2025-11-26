//! Secret storage abstraction.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::Result;

/// A secret value (can be a single string or key-value pairs).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SecretValue {
    /// A single secret string.
    String(String),
    /// A map of key-value pairs.
    Map(HashMap<String, String>),
}

impl SecretValue {
    /// Get the value as a string (returns first/only value for maps).
    pub fn as_string(&self) -> Option<&str> {
        match self {
            SecretValue::String(s) => Some(s),
            SecretValue::Map(m) => m.values().next().map(|s| s.as_str()),
        }
    }

    /// Get a specific key from a map secret.
    pub fn get(&self, key: &str) -> Option<&str> {
        match self {
            SecretValue::String(s) if key.is_empty() => Some(s),
            SecretValue::Map(m) => m.get(key).map(|s| s.as_str()),
            _ => None,
        }
    }
}

/// Trait for secret storage backends.
#[async_trait]
pub trait SecretStore: Send + Sync {
    /// Get a secret by path.
    async fn get(&self, path: &str) -> Result<SecretValue>;

    /// Get a specific key from a secret.
    async fn get_key(&self, path: &str, key: &str) -> Result<String>;

    /// List secret paths with a prefix.
    async fn list(&self, prefix: &str) -> Result<Vec<String>>;

    /// Set/create a secret.
    async fn set(&self, path: &str, value: SecretValue) -> Result<()>;

    /// Delete a secret.
    async fn delete(&self, path: &str) -> Result<()>;
}

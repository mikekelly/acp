//! Centralized registry for tokens, plugins, and credentials
//!
//! The registry is the authoritative record of what exists in the system.
//! It's stored as a single JSON document in the SecretStore at key "_registry".
//! This solves the problem of listing items on platforms where enumeration is
//! difficult (e.g., macOS Keychain).
//!
//! The actual values (token strings, plugin code, credential values) are still
//! stored at their individual keys. The registry only tracks metadata.

use crate::{storage::SecretStore, AcpError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Token metadata entry in the registry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenEntry {
    pub id: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub prefix: String,
}

/// Plugin metadata entry in the registry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PluginEntry {
    pub name: String,
    pub hosts: Vec<String>,
    pub credential_schema: Vec<String>,
}

/// Credential metadata entry in the registry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CredentialEntry {
    pub plugin: String,
    pub field: String,
}

/// The complete registry data structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RegistryData {
    pub version: u32,
    pub tokens: Vec<TokenEntry>,
    pub plugins: Vec<PluginEntry>,
    pub credentials: Vec<CredentialEntry>,
}

impl Default for RegistryData {
    fn default() -> Self {
        Self {
            version: 1,
            tokens: Vec::new(),
            plugins: Vec::new(),
            credentials: Vec::new(),
        }
    }
}

/// Registry manager for centralized metadata storage
///
/// The Registry wraps a SecretStore and provides load/save operations
/// for the registry data. The registry is stored at key "_registry".
pub struct Registry {
    store: Arc<dyn SecretStore>,
}

impl Registry {
    /// Storage key for the registry
    const KEY: &'static str = "_registry";

    /// Create a new Registry with the given store
    pub fn new(store: Arc<dyn SecretStore>) -> Self {
        Self { store }
    }

    /// Load the registry from storage
    ///
    /// Returns an empty RegistryData if the registry doesn't exist yet.
    /// This is not an error - it's the expected state for a fresh installation.
    pub async fn load(&self) -> Result<RegistryData> {
        match self.store.get(Self::KEY).await? {
            Some(bytes) => {
                let data = serde_json::from_slice(&bytes).map_err(|e| {
                    AcpError::storage(format!("Failed to parse registry JSON: {}", e))
                })?;
                Ok(data)
            }
            None => {
                // Registry doesn't exist yet - return empty
                Ok(RegistryData::default())
            }
        }
    }

    /// Save the registry to storage
    ///
    /// Serializes the RegistryData to JSON and stores it at the registry key.
    pub async fn save(&self, data: &RegistryData) -> Result<()> {
        let bytes = serde_json::to_vec(data)
            .map_err(|e| AcpError::storage(format!("Failed to serialize registry: {}", e)))?;
        self.store.set(Self::KEY, &bytes).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_data_serialization() {
        let data = RegistryData {
            version: 1,
            tokens: vec![TokenEntry {
                id: "abc123".to_string(),
                name: "test-token".to_string(),
                created_at: DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
                prefix: "acp_abc123".to_string(),
            }],
            plugins: vec![PluginEntry {
                name: "exa".to_string(),
                hosts: vec!["api.exa.ai".to_string()],
                credential_schema: vec!["api_key".to_string()],
            }],
            credentials: vec![CredentialEntry {
                plugin: "exa".to_string(),
                field: "api_key".to_string(),
            }],
        };

        // Serialize to JSON
        let json = serde_json::to_string(&data).expect("serialization should succeed");
        assert!(json.contains("\"version\":1"));
        assert!(json.contains("\"id\":\"abc123\""));
        assert!(json.contains("\"name\":\"exa\""));

        // Deserialize back
        let parsed: RegistryData =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.tokens.len(), 1);
        assert_eq!(parsed.tokens[0].id, "abc123");
        assert_eq!(parsed.plugins.len(), 1);
        assert_eq!(parsed.plugins[0].name, "exa");
        assert_eq!(parsed.credentials.len(), 1);
        assert_eq!(parsed.credentials[0].plugin, "exa");
    }

    #[test]
    fn test_registry_data_empty() {
        let data = RegistryData::default();

        assert_eq!(data.version, 1);
        assert_eq!(data.tokens.len(), 0);
        assert_eq!(data.plugins.len(), 0);
        assert_eq!(data.credentials.len(), 0);

        // Should serialize/deserialize empty structures
        let json = serde_json::to_string(&data).expect("serialization should succeed");
        let parsed: RegistryData =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(parsed.version, 1);
    }

    #[test]
    fn test_token_entry_fields() {
        let token = TokenEntry {
            id: "test123".to_string(),
            name: "my-agent".to_string(),
            created_at: Utc::now(),
            prefix: "acp_test123".to_string(),
        };

        assert_eq!(token.id, "test123");
        assert_eq!(token.name, "my-agent");
        assert_eq!(token.prefix, "acp_test123");
    }

    #[test]
    fn test_plugin_entry_fields() {
        let plugin = PluginEntry {
            name: "aws-s3".to_string(),
            hosts: vec!["*.s3.amazonaws.com".to_string()],
            credential_schema: vec!["access_key".to_string(), "secret_key".to_string()],
        };

        assert_eq!(plugin.name, "aws-s3");
        assert_eq!(plugin.hosts.len(), 1);
        assert_eq!(plugin.credential_schema.len(), 2);
    }

    #[test]
    fn test_credential_entry_fields() {
        let cred = CredentialEntry {
            plugin: "exa".to_string(),
            field: "api_key".to_string(),
        };

        assert_eq!(cred.plugin, "exa");
        assert_eq!(cred.field, "api_key");
    }

    #[tokio::test]
    async fn test_registry_load_empty() {
        use crate::storage::FileStore;
        use std::sync::Arc;

        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = FileStore::new(temp_dir.path().to_path_buf())
            .await
            .expect("create FileStore");
        let registry = Registry::new(Arc::new(store));

        // Load when no registry exists yet - should return empty RegistryData
        let data = registry.load().await.expect("load should succeed");
        assert_eq!(data.version, 1);
        assert_eq!(data.tokens.len(), 0);
        assert_eq!(data.plugins.len(), 0);
        assert_eq!(data.credentials.len(), 0);
    }

    #[tokio::test]
    async fn test_registry_save_and_load() {
        use crate::storage::FileStore;
        use std::sync::Arc;

        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = FileStore::new(temp_dir.path().to_path_buf())
            .await
            .expect("create FileStore");
        let registry = Registry::new(Arc::new(store));

        // Create test data
        let data = RegistryData {
            version: 1,
            tokens: vec![TokenEntry {
                id: "test123".to_string(),
                name: "test-token".to_string(),
                created_at: Utc::now(),
                prefix: "acp_test123".to_string(),
            }],
            plugins: vec![PluginEntry {
                name: "exa".to_string(),
                hosts: vec!["api.exa.ai".to_string()],
                credential_schema: vec!["api_key".to_string()],
            }],
            credentials: vec![CredentialEntry {
                plugin: "exa".to_string(),
                field: "api_key".to_string(),
            }],
        };

        // Save
        registry
            .save(&data)
            .await
            .expect("save should succeed");

        // Load back
        let loaded = registry.load().await.expect("load should succeed");
        assert_eq!(loaded.version, data.version);
        assert_eq!(loaded.tokens.len(), 1);
        assert_eq!(loaded.tokens[0].id, "test123");
        assert_eq!(loaded.plugins.len(), 1);
        assert_eq!(loaded.plugins[0].name, "exa");
        assert_eq!(loaded.credentials.len(), 1);
        assert_eq!(loaded.credentials[0].plugin, "exa");
    }

    #[tokio::test]
    async fn test_registry_overwrite() {
        use crate::storage::FileStore;
        use std::sync::Arc;

        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = FileStore::new(temp_dir.path().to_path_buf())
            .await
            .expect("create FileStore");
        let registry = Registry::new(Arc::new(store));

        // Save initial data
        let data1 = RegistryData {
            version: 1,
            tokens: vec![TokenEntry {
                id: "token1".to_string(),
                name: "first".to_string(),
                created_at: Utc::now(),
                prefix: "acp_token1".to_string(),
            }],
            plugins: vec![],
            credentials: vec![],
        };
        registry.save(&data1).await.expect("save should succeed");

        // Overwrite with new data
        let data2 = RegistryData {
            version: 1,
            tokens: vec![
                TokenEntry {
                    id: "token1".to_string(),
                    name: "first".to_string(),
                    created_at: Utc::now(),
                    prefix: "acp_token1".to_string(),
                },
                TokenEntry {
                    id: "token2".to_string(),
                    name: "second".to_string(),
                    created_at: Utc::now(),
                    prefix: "acp_token2".to_string(),
                },
            ],
            plugins: vec![],
            credentials: vec![],
        };
        registry.save(&data2).await.expect("save should succeed");

        // Load and verify it was overwritten
        let loaded = registry.load().await.expect("load should succeed");
        assert_eq!(loaded.tokens.len(), 2);
        assert_eq!(loaded.tokens[1].id, "token2");
    }

    #[tokio::test]
    async fn test_registry_uses_correct_key() {
        use crate::storage::{FileStore, SecretStore};
        use std::sync::Arc;

        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = Arc::new(
            FileStore::new(temp_dir.path().to_path_buf())
                .await
                .expect("create FileStore"),
        );
        let registry = Registry::new(store.clone());

        // Save some data
        let data = RegistryData::default();
        registry.save(&data).await.expect("save should succeed");

        // Verify it was stored at the correct key
        let raw_value = store
            .get("_registry")
            .await
            .expect("get should succeed")
            .expect("value should exist");

        // Verify it's valid JSON
        let parsed: RegistryData =
            serde_json::from_slice(&raw_value).expect("should deserialize");
        assert_eq!(parsed.version, 1);
    }
}

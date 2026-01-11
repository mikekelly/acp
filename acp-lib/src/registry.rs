//! Centralized registry for tokens, plugins, and credentials
//!
//! The registry is the authoritative record of what exists in the system.
//! It's stored as a single JSON document in the SecretStore at key "_registry".
//! This solves the problem of listing items on platforms where enumeration is
//! difficult (e.g., macOS Keychain).
//!
//! The actual values (token strings, plugin code, credential values) are still
//! stored at their individual keys. The registry only tracks metadata.

use crate::{
    storage::{FileStore, SecretStore},
    AcpError, AgentToken, PluginRuntime, Result,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::sync::Arc;

/// Token metadata entry in the registry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenEntry {
    pub token_value: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
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

    // Token CRUD operations

    /// Add a token to the registry
    ///
    /// Loads the registry, adds the token to the tokens vec, and saves.
    pub async fn add_token(&self, token: &TokenEntry) -> Result<()> {
        let mut data = self.load().await?;
        data.tokens.push(token.clone());
        self.save(&data).await
    }

    /// Remove a token from the registry by token value
    ///
    /// Loads the registry, removes the token with matching token_value, and saves.
    pub async fn remove_token(&self, token_value: &str) -> Result<()> {
        let mut data = self.load().await?;
        data.tokens.retain(|t| t.token_value != token_value);
        self.save(&data).await
    }

    /// List all tokens in the registry
    ///
    /// Returns the tokens vec from the loaded registry.
    pub async fn list_tokens(&self) -> Result<Vec<TokenEntry>> {
        let data = self.load().await?;
        Ok(data.tokens)
    }

    // Plugin CRUD operations

    /// Add a plugin to the registry
    ///
    /// Loads the registry, adds the plugin to the plugins vec, and saves.
    pub async fn add_plugin(&self, plugin: &PluginEntry) -> Result<()> {
        let mut data = self.load().await?;
        data.plugins.push(plugin.clone());
        self.save(&data).await
    }

    /// Remove a plugin from the registry by name
    ///
    /// Loads the registry, removes the plugin with matching name, and saves.
    pub async fn remove_plugin(&self, name: &str) -> Result<()> {
        let mut data = self.load().await?;
        data.plugins.retain(|p| p.name != name);
        self.save(&data).await
    }

    /// List all plugins in the registry
    ///
    /// Returns the plugins vec from the loaded registry.
    pub async fn list_plugins(&self) -> Result<Vec<PluginEntry>> {
        let data = self.load().await?;
        Ok(data.plugins)
    }

    // Credential CRUD operations

    /// Add a credential to the registry
    ///
    /// Loads the registry, adds the credential to the credentials vec, and saves.
    pub async fn add_credential(&self, credential: &CredentialEntry) -> Result<()> {
        let mut data = self.load().await?;
        data.credentials.push(credential.clone());
        self.save(&data).await
    }

    /// Remove a credential from the registry by plugin and field
    ///
    /// Loads the registry, removes the credential with matching plugin and field, and saves.
    pub async fn remove_credential(&self, plugin: &str, field: &str) -> Result<()> {
        let mut data = self.load().await?;
        data.credentials
            .retain(|c| !(c.plugin == plugin && c.field == field));
        self.save(&data).await
    }

    /// List all credentials in the registry
    ///
    /// Returns the credentials vec from the loaded registry.
    pub async fn list_credentials(&self) -> Result<Vec<CredentialEntry>> {
        let data = self.load().await?;
        Ok(data.credentials)
    }

    // Migration support

    /// Migrate tokens from old format (token:{id}) to new format (token:{value})
    ///
    /// This method scans for tokens stored with old UUID-based keys and migrates them
    /// to the new token-value-based keys. It also updates the registry.
    ///
    /// Old format: `token:{uuid}` where uuid is the token.id field
    /// New format: `token:{token_value}` where token_value is the actual token string (acp_...)
    ///
    /// Migration process:
    /// 1. List all keys with prefix "token:"
    /// 2. For each key, load the token and check format
    /// 3. If old format (key != "token:{token.token}"), migrate:
    ///    - Store at new key "token:{token.token}"
    ///    - Add to registry if not already present
    ///    - Delete old key
    ///
    /// # Returns
    /// * Ok(()) if migration succeeded
    /// * Err if migration failed
    pub async fn migrate_old_token_format(&self) -> Result<()> {
        // We need FileStore to list keys
        // If store is not FileStore, we can't migrate (return Ok to skip)
        let file_store = match self.store.as_any().downcast_ref::<FileStore>() {
            Some(fs) => fs,
            None => return Ok(()), // Not a FileStore, skip migration
        };

        // Get all token keys
        let token_keys = file_store.list_internal("token:").await?;

        // Load existing registry to check for duplicates
        let mut registry_data = self.load().await?;
        let existing_token_values: HashSet<String> =
            registry_data.tokens.iter().map(|t| t.token_value.clone()).collect();

        for key in token_keys {
            // Load the token
            if let Some(token_bytes) = self.store.get(&key).await? {
                if let Ok(token) = serde_json::from_slice::<AgentToken>(&token_bytes) {
                    let new_key = format!("token:{}", token.token);

                    // Check if this is an old-format key
                    if key != new_key {
                        // This is old format - migrate it

                        // Store at new key
                        self.store.set(&new_key, &token_bytes).await?;

                        // Add to registry if not already present
                        if !existing_token_values.contains(&token.token) {
                            let entry = TokenEntry {
                                token_value: token.token.clone(),
                                name: token.name.clone(),
                                created_at: token.created_at,
                            };
                            registry_data.tokens.push(entry);
                        }

                        // Delete old key
                        self.store.delete(&key).await?;
                    }
                }
            }
        }

        // Save updated registry
        self.save(&registry_data).await?;

        Ok(())
    }

    /// Migrate existing FileStore data to registry
    ///
    /// This method is used during server startup to migrate from old installations
    /// that have tokens, plugins, and credentials stored but no _registry key.
    ///
    /// It only runs if the registry doesn't exist yet. If a registry already exists,
    /// this method does nothing (returns Ok immediately).
    ///
    /// # Arguments
    /// * `file_store` - Reference to the FileStore to migrate from
    ///
    /// # Returns
    /// * Ok(()) if migration succeeded or was skipped (registry already exists)
    /// * Err if migration failed
    pub async fn migrate_from_file_store(&self, file_store: &FileStore) -> Result<()> {
        // Check if registry already exists
        if self.store.get(Self::KEY).await?.is_some() {
            // Registry already exists, skip migration
            return Ok(());
        }

        // Build registry from existing keys
        let mut data = RegistryData::default();

        // Migrate tokens: keys like "token:abc123"
        let token_keys = file_store.list_internal("token:").await?;
        for key in token_keys {
            // Load the token to get metadata
            if let Some(token_bytes) = self.store.get(&key).await? {
                if let Ok(token) = serde_json::from_slice::<AgentToken>(&token_bytes) {
                    let entry = TokenEntry {
                        token_value: token.token.clone(),
                        name: token.name.clone(),
                        created_at: token.created_at,
                    };
                    data.tokens.push(entry);
                }
            }
        }

        // Migrate plugins: keys like "plugin:exa"
        let plugin_keys = file_store.list_internal("plugin:").await?;
        for key in plugin_keys {
            // Extract plugin name from key "plugin:name" -> "name"
            let plugin_name = key.strip_prefix("plugin:").unwrap_or(&key);

            // Load the plugin code to extract metadata
            if let Some(plugin_code) = self.store.get(&key).await? {
                if let Ok(code_str) = String::from_utf8(plugin_code) {
                    // Use PluginRuntime to extract metadata
                    if let Ok(mut runtime) = PluginRuntime::new() {
                        // Execute the plugin code first to set up the global plugin object
                        if runtime.execute(&code_str).is_ok() {
                            if let Ok(metadata) =
                                runtime.extract_plugin_metadata(plugin_name)
                            {
                                let entry = PluginEntry {
                                    name: plugin_name.to_string(),
                                    hosts: metadata.match_patterns,
                                    credential_schema: metadata.credential_schema,
                                };
                                data.plugins.push(entry);
                            }
                        }
                    }
                }
            }
        }

        // Migrate credentials: keys like "credential:plugin:field"
        let credential_keys = file_store.list_internal("credential:").await?;
        for key in credential_keys {
            // Parse key "credential:plugin:field" -> plugin="plugin", field="field"
            let parts: Vec<&str> = key.split(':').collect();
            if parts.len() == 3 && parts[0] == "credential" {
                let entry = CredentialEntry {
                    plugin: parts[1].to_string(),
                    field: parts[2].to_string(),
                };
                data.credentials.push(entry);
            }
        }

        // Save the registry
        self.save(&data).await?;

        Ok(())
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
                token_value: "acp_abc123".to_string(),
                name: "test-token".to_string(),
                created_at: DateTime::parse_from_rfc3339("2024-01-15T10:30:00Z")
                    .unwrap()
                    .with_timezone(&Utc),
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
        assert!(json.contains("\"token_value\":\"acp_abc123\""));
        assert!(json.contains("\"name\":\"exa\""));

        // Deserialize back
        let parsed: RegistryData =
            serde_json::from_str(&json).expect("deserialization should succeed");
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.tokens.len(), 1);
        assert_eq!(parsed.tokens[0].token_value, "acp_abc123");
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
            token_value: "acp_test123".to_string(),
            name: "my-agent".to_string(),
            created_at: Utc::now(),
        };

        assert_eq!(token.token_value, "acp_test123");
        assert_eq!(token.name, "my-agent");
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
                token_value: "acp_test123".to_string(),
                name: "test-token".to_string(),
                created_at: Utc::now(),
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
        assert_eq!(loaded.tokens[0].token_value, "acp_test123");
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
                token_value: "acp_token1".to_string(),
                name: "first".to_string(),
                created_at: Utc::now(),
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
                    token_value: "acp_token1".to_string(),
                    name: "first".to_string(),
                    created_at: Utc::now(),
                },
                TokenEntry {
                    token_value: "acp_token2".to_string(),
                    name: "second".to_string(),
                    created_at: Utc::now(),
                },
            ],
            plugins: vec![],
            credentials: vec![],
        };
        registry.save(&data2).await.expect("save should succeed");

        // Load and verify it was overwritten
        let loaded = registry.load().await.expect("load should succeed");
        assert_eq!(loaded.tokens.len(), 2);
        assert_eq!(loaded.tokens[1].token_value, "acp_token2");
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

    // RED: Tests for token CRUD operations
    #[tokio::test]
    async fn test_add_token() {
        use crate::storage::FileStore;
        use std::sync::Arc;

        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = FileStore::new(temp_dir.path().to_path_buf())
            .await
            .expect("create FileStore");
        let registry = Registry::new(Arc::new(store));

        let token = TokenEntry {
            token_value: "acp_abc123".to_string(),
            name: "test-token".to_string(),
            created_at: Utc::now(),
        };

        // Add token should succeed
        registry.add_token(&token).await.expect("add should succeed");

        // Verify token is in registry
        let tokens = registry.list_tokens().await.expect("list should succeed");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_value, "acp_abc123");
        assert_eq!(tokens[0].name, "test-token");
    }

    #[tokio::test]
    async fn test_remove_token() {
        use crate::storage::FileStore;
        use std::sync::Arc;

        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = FileStore::new(temp_dir.path().to_path_buf())
            .await
            .expect("create FileStore");
        let registry = Registry::new(Arc::new(store));

        // Add two tokens
        let token1 = TokenEntry {
            token_value: "acp_abc123".to_string(),
            name: "token1".to_string(),
            created_at: Utc::now(),
        };
        let token2 = TokenEntry {
            token_value: "acp_def456".to_string(),
            name: "token2".to_string(),
            created_at: Utc::now(),
        };
        registry.add_token(&token1).await.expect("add should succeed");
        registry.add_token(&token2).await.expect("add should succeed");

        // Remove first token by value
        registry
            .remove_token("acp_abc123")
            .await
            .expect("remove should succeed");

        // Verify only second token remains
        let tokens = registry.list_tokens().await.expect("list should succeed");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_value, "acp_def456");
    }

    #[tokio::test]
    async fn test_list_tokens() {
        use crate::storage::FileStore;
        use std::sync::Arc;

        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = FileStore::new(temp_dir.path().to_path_buf())
            .await
            .expect("create FileStore");
        let registry = Registry::new(Arc::new(store));

        // List should return empty on fresh registry
        let tokens = registry.list_tokens().await.expect("list should succeed");
        assert_eq!(tokens.len(), 0);

        // Add tokens
        let token1 = TokenEntry {
            token_value: "acp_abc123".to_string(),
            name: "token1".to_string(),
            created_at: Utc::now(),
        };
        let token2 = TokenEntry {
            token_value: "acp_def456".to_string(),
            name: "token2".to_string(),
            created_at: Utc::now(),
        };
        registry.add_token(&token1).await.expect("add should succeed");
        registry.add_token(&token2).await.expect("add should succeed");

        // List should return both
        let tokens = registry.list_tokens().await.expect("list should succeed");
        assert_eq!(tokens.len(), 2);
    }

    // RED: Tests for plugin CRUD operations
    #[tokio::test]
    async fn test_add_plugin() {
        use crate::storage::FileStore;
        use std::sync::Arc;

        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = FileStore::new(temp_dir.path().to_path_buf())
            .await
            .expect("create FileStore");
        let registry = Registry::new(Arc::new(store));

        let plugin = PluginEntry {
            name: "exa".to_string(),
            hosts: vec!["api.exa.ai".to_string()],
            credential_schema: vec!["api_key".to_string()],
        };

        // Add plugin should succeed
        registry
            .add_plugin(&plugin)
            .await
            .expect("add should succeed");

        // Verify plugin is in registry
        let plugins = registry.list_plugins().await.expect("list should succeed");
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "exa");
        assert_eq!(plugins[0].hosts, vec!["api.exa.ai"]);
    }

    #[tokio::test]
    async fn test_remove_plugin() {
        use crate::storage::FileStore;
        use std::sync::Arc;

        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = FileStore::new(temp_dir.path().to_path_buf())
            .await
            .expect("create FileStore");
        let registry = Registry::new(Arc::new(store));

        // Add two plugins
        let plugin1 = PluginEntry {
            name: "exa".to_string(),
            hosts: vec!["api.exa.ai".to_string()],
            credential_schema: vec!["api_key".to_string()],
        };
        let plugin2 = PluginEntry {
            name: "github".to_string(),
            hosts: vec!["api.github.com".to_string()],
            credential_schema: vec!["token".to_string()],
        };
        registry
            .add_plugin(&plugin1)
            .await
            .expect("add should succeed");
        registry
            .add_plugin(&plugin2)
            .await
            .expect("add should succeed");

        // Remove first plugin
        registry
            .remove_plugin("exa")
            .await
            .expect("remove should succeed");

        // Verify only second plugin remains
        let plugins = registry.list_plugins().await.expect("list should succeed");
        assert_eq!(plugins.len(), 1);
        assert_eq!(plugins[0].name, "github");
    }

    #[tokio::test]
    async fn test_list_plugins() {
        use crate::storage::FileStore;
        use std::sync::Arc;

        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = FileStore::new(temp_dir.path().to_path_buf())
            .await
            .expect("create FileStore");
        let registry = Registry::new(Arc::new(store));

        // List should return empty on fresh registry
        let plugins = registry.list_plugins().await.expect("list should succeed");
        assert_eq!(plugins.len(), 0);

        // Add plugins
        let plugin1 = PluginEntry {
            name: "exa".to_string(),
            hosts: vec!["api.exa.ai".to_string()],
            credential_schema: vec!["api_key".to_string()],
        };
        let plugin2 = PluginEntry {
            name: "github".to_string(),
            hosts: vec!["api.github.com".to_string()],
            credential_schema: vec!["token".to_string()],
        };
        registry
            .add_plugin(&plugin1)
            .await
            .expect("add should succeed");
        registry
            .add_plugin(&plugin2)
            .await
            .expect("add should succeed");

        // List should return both
        let plugins = registry.list_plugins().await.expect("list should succeed");
        assert_eq!(plugins.len(), 2);
    }

    // RED: Tests for credential CRUD operations
    #[tokio::test]
    async fn test_add_credential() {
        use crate::storage::FileStore;
        use std::sync::Arc;

        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = FileStore::new(temp_dir.path().to_path_buf())
            .await
            .expect("create FileStore");
        let registry = Registry::new(Arc::new(store));

        let cred = CredentialEntry {
            plugin: "exa".to_string(),
            field: "api_key".to_string(),
        };

        // Add credential should succeed
        registry
            .add_credential(&cred)
            .await
            .expect("add should succeed");

        // Verify credential is in registry
        let creds = registry
            .list_credentials()
            .await
            .expect("list should succeed");
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].plugin, "exa");
        assert_eq!(creds[0].field, "api_key");
    }

    #[tokio::test]
    async fn test_remove_credential() {
        use crate::storage::FileStore;
        use std::sync::Arc;

        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = FileStore::new(temp_dir.path().to_path_buf())
            .await
            .expect("create FileStore");
        let registry = Registry::new(Arc::new(store));

        // Add two credentials
        let cred1 = CredentialEntry {
            plugin: "exa".to_string(),
            field: "api_key".to_string(),
        };
        let cred2 = CredentialEntry {
            plugin: "exa".to_string(),
            field: "secret".to_string(),
        };
        let cred3 = CredentialEntry {
            plugin: "github".to_string(),
            field: "token".to_string(),
        };
        registry
            .add_credential(&cred1)
            .await
            .expect("add should succeed");
        registry
            .add_credential(&cred2)
            .await
            .expect("add should succeed");
        registry
            .add_credential(&cred3)
            .await
            .expect("add should succeed");

        // Remove exa api_key credential
        registry
            .remove_credential("exa", "api_key")
            .await
            .expect("remove should succeed");

        // Verify only exa/secret and github/token remain
        let creds = registry
            .list_credentials()
            .await
            .expect("list should succeed");
        assert_eq!(creds.len(), 2);
        assert!(creds.iter().any(|c| c.plugin == "exa" && c.field == "secret"));
        assert!(creds.iter().any(|c| c.plugin == "github" && c.field == "token"));
        assert!(!creds
            .iter()
            .any(|c| c.plugin == "exa" && c.field == "api_key"));
    }

    #[tokio::test]
    async fn test_list_credentials() {
        use crate::storage::FileStore;
        use std::sync::Arc;

        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = FileStore::new(temp_dir.path().to_path_buf())
            .await
            .expect("create FileStore");
        let registry = Registry::new(Arc::new(store));

        // List should return empty on fresh registry
        let creds = registry
            .list_credentials()
            .await
            .expect("list should succeed");
        assert_eq!(creds.len(), 0);

        // Add credentials
        let cred1 = CredentialEntry {
            plugin: "exa".to_string(),
            field: "api_key".to_string(),
        };
        let cred2 = CredentialEntry {
            plugin: "github".to_string(),
            field: "token".to_string(),
        };
        registry
            .add_credential(&cred1)
            .await
            .expect("add should succeed");
        registry
            .add_credential(&cred2)
            .await
            .expect("add should succeed");

        // List should return both
        let creds = registry
            .list_credentials()
            .await
            .expect("list should succeed");
        assert_eq!(creds.len(), 2);
    }

    // RED: Test that token value IS the ID (no separate id field)
    #[tokio::test]
    async fn test_token_entry_uses_value_as_key() {
        use crate::storage::FileStore;
        use std::sync::Arc;

        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = FileStore::new(temp_dir.path().to_path_buf())
            .await
            .expect("create FileStore");
        let registry = Registry::new(Arc::new(store));

        let token = TokenEntry {
            token_value: "acp_test123".to_string(),
            name: "test-token".to_string(),
            created_at: Utc::now(),
        };

        // Add token
        registry.add_token(&token).await.expect("add should succeed");

        // Verify token is in registry
        let tokens = registry.list_tokens().await.expect("list should succeed");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_value, "acp_test123");
        assert_eq!(tokens[0].name, "test-token");
    }

    // RED: Test that remove_token works with value as key
    #[tokio::test]
    async fn test_remove_token_by_value() {
        use crate::storage::FileStore;
        use std::sync::Arc;

        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = FileStore::new(temp_dir.path().to_path_buf())
            .await
            .expect("create FileStore");
        let registry = Registry::new(Arc::new(store));

        // Add token
        let token = TokenEntry {
            token_value: "acp_test123".to_string(),
            name: "test-token".to_string(),
            created_at: Utc::now(),
        };
        registry.add_token(&token).await.expect("add should succeed");

        // Remove by value
        registry
            .remove_token("acp_test123")
            .await
            .expect("remove should succeed");

        // Verify token is gone
        let tokens = registry.list_tokens().await.expect("list should succeed");
        assert_eq!(tokens.len(), 0);
    }

    // RED: Test migration from old token format (token:{id}) to new format (token:{value})
    #[tokio::test]
    async fn test_migrate_old_token_format() {
        use crate::storage::{FileStore, SecretStore};
        use std::sync::Arc;

        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = Arc::new(
            FileStore::new(temp_dir.path().to_path_buf())
                .await
                .expect("create FileStore"),
        );
        let registry = Registry::new(Arc::clone(&store) as Arc<dyn SecretStore>);

        // Create a token using OLD format: token:{id}
        let old_token = AgentToken::new("old-token");
        let old_token_json = serde_json::to_vec(&old_token).expect("serialize token");
        let old_key = format!("token:{}", old_token.id); // OLD FORMAT
        store.set(&old_key, &old_token_json).await.expect("store old token");

        // Run migration
        registry.migrate_old_token_format().await.expect("migration should succeed");

        // Verify token is now in registry with new format
        let tokens = registry.list_tokens().await.expect("list should succeed");
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].token_value, old_token.token);
        assert_eq!(tokens[0].name, "old-token");

        // Verify token is accessible via new format key
        let new_key = format!("token:{}", old_token.token);
        let token_json = store.get(&new_key).await.expect("get should succeed").expect("token should exist");
        let migrated_token: AgentToken = serde_json::from_slice(&token_json).expect("deserialize should succeed");
        assert_eq!(migrated_token.token, old_token.token);
        assert_eq!(migrated_token.name, "old-token");

        // Verify old key is deleted
        let old_value = store.get(&old_key).await.expect("get should succeed");
        assert!(old_value.is_none(), "old format key should be deleted");
    }

    // RED: Test that migration handles mixed old and new formats
    #[tokio::test]
    async fn test_migrate_mixed_token_formats() {
        use crate::storage::{FileStore, SecretStore};
        use std::sync::Arc;

        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = Arc::new(
            FileStore::new(temp_dir.path().to_path_buf())
                .await
                .expect("create FileStore"),
        );
        let registry = Registry::new(Arc::clone(&store) as Arc<dyn SecretStore>);

        // Create one token in OLD format
        let old_token = AgentToken::new("old-token");
        let old_token_json = serde_json::to_vec(&old_token).expect("serialize token");
        let old_key = format!("token:{}", old_token.id);
        store.set(&old_key, &old_token_json).await.expect("store old token");

        // Create one token in NEW format with registry entry
        let new_token = AgentToken::new("new-token");
        let new_token_json = serde_json::to_vec(&new_token).expect("serialize token");
        let new_key = format!("token:{}", new_token.token);
        store.set(&new_key, &new_token_json).await.expect("store new token");

        let new_entry = TokenEntry {
            token_value: new_token.token.clone(),
            name: new_token.name.clone(),
            created_at: new_token.created_at,
        };
        registry.add_token(&new_entry).await.expect("add new token to registry");

        // Run migration
        registry.migrate_old_token_format().await.expect("migration should succeed");

        // Verify both tokens are in registry
        let tokens = registry.list_tokens().await.expect("list should succeed");
        assert_eq!(tokens.len(), 2);

        let token_values: Vec<String> = tokens.iter().map(|t| t.token_value.clone()).collect();
        assert!(token_values.contains(&old_token.token));
        assert!(token_values.contains(&new_token.token));

        // Verify old key is deleted, new key still exists
        let old_value = store.get(&old_key).await.expect("get should succeed");
        assert!(old_value.is_none(), "old format key should be deleted");

        let new_value = store.get(&new_key).await.expect("get should succeed");
        assert!(new_value.is_some(), "new format key should still exist");
    }
}

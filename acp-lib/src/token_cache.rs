//! Token Cache - Invalidate-on-write cache for agent tokens
//!
//! Provides a caching layer over SecretStore for agent tokens.
//! - Read path: Check in-memory cache → if miss, load ALL tokens from Registry
//! - Write path: Modify storage → invalidate cache
//! - Storage is the single source of truth

use crate::error::{AcpError, Result};
use crate::registry::Registry;
use crate::storage::SecretStore;
use crate::types::AgentToken;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Token cache with invalidate-on-write pattern
///
/// Wraps a SecretStore and Registry, providing in-memory caching for tokens.
/// Cache is invalidated (set to None) on writes, ensuring storage is always the source of truth.
pub struct TokenCache {
    store: Arc<dyn SecretStore>,
    registry: Arc<Registry>,
    cache: RwLock<Option<HashMap<String, AgentToken>>>, // None = invalidated
}

impl TokenCache {
    /// Create a new TokenCache
    pub fn new(store: Arc<dyn SecretStore>, registry: Arc<Registry>) -> Self {
        Self {
            store,
            registry,
            cache: RwLock::new(None), // Start invalidated
        }
    }

    /// Get token by bearer token value
    ///
    /// Loads from Registry on cache miss.
    pub async fn get_by_token(&self, token: &str) -> Result<Option<AgentToken>> {
        // Try cache first
        {
            let cache_guard = self.cache.read().await;
            if let Some(ref token_map) = *cache_guard {
                return Ok(token_map.get(token).cloned());
            }
        }

        // Cache miss - load all tokens from Registry
        self.load_cache().await?;

        // Try again
        let cache_guard = self.cache.read().await;
        if let Some(ref token_map) = *cache_guard {
            Ok(token_map.get(token).cloned())
        } else {
            // Should never happen since we just loaded
            Ok(None)
        }
    }

    /// List all tokens
    ///
    /// Loads from Registry on cache miss.
    pub async fn list(&self) -> Result<Vec<AgentToken>> {
        // Try cache first
        {
            let cache_guard = self.cache.read().await;
            if let Some(ref token_map) = *cache_guard {
                return Ok(token_map.values().cloned().collect());
            }
        }

        // Cache miss - load all tokens from Registry
        self.load_cache().await?;

        // Try again
        let cache_guard = self.cache.read().await;
        if let Some(ref token_map) = *cache_guard {
            Ok(token_map.values().cloned().collect())
        } else {
            // Should never happen since we just loaded
            Ok(Vec::new())
        }
    }

    /// Create a new token
    ///
    /// Writes to disk, updates Registry, and invalidates cache.
    pub async fn create(&self, name: &str) -> Result<AgentToken> {
        let token = AgentToken::new(name);

        // Persist to storage
        let token_json = serde_json::to_vec(&token)
            .map_err(|e| AcpError::storage(format!("Failed to serialize token: {}", e)))?;

        let store_key = format!("token:{}", token.id);
        self.store.set(&store_key, &token_json).await?;

        // Add to registry
        use crate::registry::TokenEntry;
        let token_entry = TokenEntry {
            id: token.id.clone(),
            name: token.name.clone(),
            created_at: token.created_at,
            prefix: token.prefix.clone(),
        };
        self.registry.add_token(&token_entry).await?;

        // Invalidate cache
        self.invalidate().await;

        Ok(token)
    }

    /// Delete token by ID
    ///
    /// Writes to disk, updates Registry, and invalidates cache. Returns true if token existed, false otherwise.
    pub async fn delete(&self, id: &str) -> Result<bool> {
        // Delete from storage
        let store_key = format!("token:{}", id);

        // Check if it exists first by loading cache
        let existed = {
            let cache_guard = self.cache.read().await;
            if let Some(ref token_map) = *cache_guard {
                token_map.values().any(|t| t.id == id)
            } else {
                // Cache miss - need to load to know if it exists
                drop(cache_guard);
                self.load_cache().await?;
                let cache_guard = self.cache.read().await;
                if let Some(ref token_map) = *cache_guard {
                    token_map.values().any(|t| t.id == id)
                } else {
                    false
                }
            }
        };

        self.store.delete(&store_key).await?;

        // Remove from registry
        self.registry.remove_token(id).await?;

        // Invalidate cache
        self.invalidate().await;

        Ok(existed)
    }

    /// Invalidate cache
    ///
    /// Forces next read to reload from storage.
    pub async fn invalidate(&self) {
        *self.cache.write().await = None;
    }

    /// Load all tokens from Registry into cache
    ///
    /// Uses Registry to get token metadata, then loads token values from storage.
    async fn load_cache(&self) -> Result<()> {
        let mut token_map = HashMap::new();

        // Get token list from Registry
        let token_entries = self.registry.list_tokens().await?;

        // Load each token value from storage
        for entry in token_entries {
            let key = format!("token:{}", entry.id);
            if let Some(token_json) = self.store.get(&key).await? {
                match serde_json::from_slice::<AgentToken>(&token_json) {
                    Ok(token) => {
                        token_map.insert(token.token.clone(), token);
                    }
                    Err(e) => {
                        tracing::warn!("Failed to deserialize token {}: {}", key, e);
                    }
                }
            }
        }

        // Update cache
        *self.cache.write().await = Some(token_map);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::Registry;
    use crate::storage::FileStore;

    #[tokio::test]
    async fn test_create_and_get_token() {
        let _temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = Arc::new(
            FileStore::new(_temp_dir.path().to_path_buf())
                .await
                .expect("create FileStore"),
        );
        let registry = Arc::new(Registry::new(Arc::clone(&store) as Arc<dyn SecretStore>));

        let cache = TokenCache::new(store, registry);

        // Create a token
        let token = cache.create("test-agent").await.expect("create token");
        assert_eq!(token.name, "test-agent");

        // Get the token by value
        let retrieved = cache
            .get_by_token(&token.token)
            .await
            .expect("get token")
            .expect("token should exist");

        assert_eq!(retrieved.id, token.id);
        assert_eq!(retrieved.name, "test-agent");
    }

    #[tokio::test]
    async fn test_list_tokens() {
        let _temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = Arc::new(
            FileStore::new(_temp_dir.path().to_path_buf())
                .await
                .expect("create FileStore"),
        );
        let registry = Arc::new(Registry::new(Arc::clone(&store) as Arc<dyn SecretStore>));

        let cache = TokenCache::new(store, registry);

        // Initially empty
        let tokens = cache.list().await.expect("list tokens");
        assert_eq!(tokens.len(), 0);

        // Create some tokens
        let token1 = cache.create("agent-1").await.expect("create token");
        let token2 = cache.create("agent-2").await.expect("create token");

        // List should return both
        let tokens = cache.list().await.expect("list tokens");
        assert_eq!(tokens.len(), 2);

        let ids: Vec<String> = tokens.iter().map(|t| t.id.clone()).collect();
        assert!(ids.contains(&token1.id));
        assert!(ids.contains(&token2.id));
    }

    #[tokio::test]
    async fn test_delete_token() {
        let _temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = Arc::new(
            FileStore::new(_temp_dir.path().to_path_buf())
                .await
                .expect("create FileStore"),
        );
        let registry = Arc::new(Registry::new(Arc::clone(&store) as Arc<dyn SecretStore>));

        let cache = TokenCache::new(store, registry);

        // Create a token
        let token = cache.create("test-agent").await.expect("create token");

        // Delete it
        let existed = cache.delete(&token.id).await.expect("delete token");
        assert!(existed);

        // Should not be found
        let retrieved = cache
            .get_by_token(&token.token)
            .await
            .expect("get token");
        assert!(retrieved.is_none());

        // List should be empty
        let tokens = cache.list().await.expect("list tokens");
        assert_eq!(tokens.len(), 0);
    }

    #[tokio::test]
    async fn test_delete_nonexistent_token() {
        let _temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = Arc::new(
            FileStore::new(_temp_dir.path().to_path_buf())
                .await
                .expect("create FileStore"),
        );
        let registry = Arc::new(Registry::new(Arc::clone(&store) as Arc<dyn SecretStore>));

        let cache = TokenCache::new(store, registry);

        // Delete non-existent token
        let existed = cache.delete("nonexistent").await.expect("delete token");
        assert!(!existed);
    }

    #[tokio::test]
    async fn test_cache_invalidation_on_write() {
        let _temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = Arc::new(
            FileStore::new(_temp_dir.path().to_path_buf())
                .await
                .expect("create FileStore"),
        );
        let registry = Arc::new(Registry::new(Arc::clone(&store) as Arc<dyn SecretStore>));

        let cache = TokenCache::new(store, registry);

        // Create token 1
        let token1 = cache.create("agent-1").await.expect("create token");

        // Prime the cache by listing
        let tokens = cache.list().await.expect("list tokens");
        assert_eq!(tokens.len(), 1);

        // Create token 2 (should invalidate cache)
        let token2 = cache.create("agent-2").await.expect("create token");

        // List should show both (cache was invalidated and reloaded)
        let tokens = cache.list().await.expect("list tokens");
        assert_eq!(tokens.len(), 2);

        let ids: Vec<String> = tokens.iter().map(|t| t.id.clone()).collect();
        assert!(ids.contains(&token1.id));
        assert!(ids.contains(&token2.id));
    }

    #[tokio::test]
    async fn test_get_nonexistent_token() {
        let _temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = Arc::new(
            FileStore::new(_temp_dir.path().to_path_buf())
                .await
                .expect("create FileStore"),
        );
        let registry = Arc::new(Registry::new(Arc::clone(&store) as Arc<dyn SecretStore>));

        let cache = TokenCache::new(store, registry);

        // Get non-existent token
        let retrieved = cache
            .get_by_token("nonexistent")
            .await
            .expect("get token");
        assert!(retrieved.is_none());
    }

    #[tokio::test]
    async fn test_manual_invalidation() {
        let _temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = Arc::new(
            FileStore::new(_temp_dir.path().to_path_buf())
                .await
                .expect("create FileStore"),
        );
        let registry = Arc::new(Registry::new(Arc::clone(&store) as Arc<dyn SecretStore>));

        let cache = TokenCache::new(Arc::clone(&store) as Arc<dyn SecretStore>, Arc::clone(&registry));

        // Create a token
        let token = cache.create("agent-1").await.expect("create token");

        // Prime the cache
        let tokens = cache.list().await.expect("list tokens");
        assert_eq!(tokens.len(), 1);

        // Manually add a token to storage AND registry (bypassing cache)
        let token2 = AgentToken::new("agent-2");
        let token_json = serde_json::to_vec(&token2).expect("serialize token");
        let store_key = format!("token:{}", token2.id);
        store.set(&store_key, &token_json).await.expect("store token");

        // Also add to registry
        use crate::registry::TokenEntry;
        let token_entry = TokenEntry {
            id: token2.id.clone(),
            name: token2.name.clone(),
            created_at: token2.created_at,
            prefix: token2.prefix.clone(),
        };
        registry.add_token(&token_entry).await.expect("add to registry");

        // List should still show only 1 (cache is stale)
        let tokens = cache.list().await.expect("list tokens");
        assert_eq!(tokens.len(), 1);

        // Invalidate cache
        cache.invalidate().await;

        // Now list should show 2
        let tokens = cache.list().await.expect("list tokens");
        assert_eq!(tokens.len(), 2);

        let ids: Vec<String> = tokens.iter().map(|t| t.id.clone()).collect();
        assert!(ids.contains(&token.id));
        assert!(ids.contains(&token2.id));
    }
}

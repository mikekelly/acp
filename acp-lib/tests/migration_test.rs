//! Migration tests for upgrading existing installations
//!
//! Tests that verify the migration logic for existing installations
//! that have tokens, plugins, and credentials stored but no _registry key.

use acp_lib::{
    registry::{Registry, TokenEntry},
    storage::{FileStore, SecretStore},
    AgentToken,
};
use chrono::Utc;
use std::sync::Arc;

/// Test that migration builds registry from existing FileStore data
#[tokio::test]
async fn test_migrate_from_file_store() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let store = Arc::new(
        FileStore::new(temp_dir.path().to_path_buf())
            .await
            .expect("create FileStore"),
    );

    // Simulate an old installation by directly setting keys WITHOUT creating _registry
    // Add a token
    let token = AgentToken::new("test-token");
    let token_key = format!("token:{}", token.id);
    let token_json = serde_json::to_vec(&token).expect("serialize token");
    store.set(&token_key, &token_json).await.expect("set token");

    // Add a plugin
    let plugin_code = r#"
        var plugin = {
            name: "exa",
            matchPatterns: ["api.exa.ai"],
            credentialSchema: ["api_key"],
            transform: function(request, credentials) { return request; }
        };
    "#;
    store
        .set("plugin:exa", plugin_code.as_bytes())
        .await
        .expect("set plugin");

    // Add credentials for the plugin
    store
        .set("credential:exa:api_key", b"secret123")
        .await
        .expect("set credential");

    // Verify _registry doesn't exist yet
    let registry_exists = store.get("_registry").await.expect("check registry");
    assert!(registry_exists.is_none(), "registry should not exist yet");

    // Create Registry and call migration
    let registry = Registry::new(store.clone() as Arc<dyn acp_lib::storage::SecretStore>);
    registry
        .migrate_from_file_store(&store)
        .await
        .expect("migration should succeed");

    // Verify registry was created and contains the migrated data
    let tokens = registry.list_tokens().await.expect("list tokens");
    assert_eq!(tokens.len(), 1, "should have migrated 1 token");
    assert_eq!(tokens[0].id, token.id);
    assert_eq!(tokens[0].name, "test-token");

    let plugins = registry.list_plugins().await.expect("list plugins");
    assert_eq!(plugins.len(), 1, "should have migrated 1 plugin");
    assert_eq!(plugins[0].name, "exa");
    assert_eq!(plugins[0].hosts, vec!["api.exa.ai"]);
    assert_eq!(plugins[0].credential_schema, vec!["api_key"]);

    let credentials = registry.list_credentials().await.expect("list credentials");
    assert_eq!(credentials.len(), 1, "should have migrated 1 credential");
    assert_eq!(credentials[0].plugin, "exa");
    assert_eq!(credentials[0].field, "api_key");
}

/// Test that migration only runs when _registry doesn't exist
#[tokio::test]
async fn test_migrate_skips_when_registry_exists() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let store = Arc::new(
        FileStore::new(temp_dir.path().to_path_buf())
            .await
            .expect("create FileStore"),
    );

    // Create registry with existing data
    let registry = Registry::new(store.clone() as Arc<dyn acp_lib::storage::SecretStore>);
    let existing_token = TokenEntry {
        id: "existing123".to_string(),
        name: "existing-token".to_string(),
        created_at: Utc::now(),
        prefix: "acp_existing123".to_string(),
    };
    registry
        .add_token(&existing_token)
        .await
        .expect("add existing token");

    // Add a raw token key that would be migrated if registry didn't exist
    let new_token = AgentToken::new("new-token");
    let token_key = format!("token:{}", new_token.id);
    let token_json = serde_json::to_vec(&new_token).expect("serialize token");
    store
        .set(&token_key, &token_json)
        .await
        .expect("set token");

    // Call migration - should skip because registry exists
    registry
        .migrate_from_file_store(&store)
        .await
        .expect("migration should succeed");

    // Verify only the original token exists in registry
    let tokens = registry.list_tokens().await.expect("list tokens");
    assert_eq!(tokens.len(), 1, "should still have only 1 token");
    assert_eq!(tokens[0].id, "existing123", "should be the original token");
}

/// Test that migration handles empty FileStore gracefully
#[tokio::test]
async fn test_migrate_from_empty_file_store() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let store = Arc::new(
        FileStore::new(temp_dir.path().to_path_buf())
            .await
            .expect("create FileStore"),
    );

    // Create registry and call migration on empty store
    let registry = Registry::new(store.clone() as Arc<dyn acp_lib::storage::SecretStore>);
    registry
        .migrate_from_file_store(&store)
        .await
        .expect("migration should succeed");

    // Verify empty registry was created
    let tokens = registry.list_tokens().await.expect("list tokens");
    assert_eq!(tokens.len(), 0);

    let plugins = registry.list_plugins().await.expect("list plugins");
    assert_eq!(plugins.len(), 0);

    let credentials = registry.list_credentials().await.expect("list credentials");
    assert_eq!(credentials.len(), 0);
}

/// Test that migration handles multiple tokens, plugins, and credentials
#[tokio::test]
async fn test_migrate_multiple_items() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let store = Arc::new(
        FileStore::new(temp_dir.path().to_path_buf())
            .await
            .expect("create FileStore"),
    );

    // Add multiple tokens
    for i in 0..3 {
        let token = AgentToken::new(&format!("token-{}", i));
        let token_key = format!("token:{}", token.id);
        let token_json = serde_json::to_vec(&token).expect("serialize token");
        store.set(&token_key, &token_json).await.expect("set token");
    }

    // Add multiple plugins
    for plugin_name in ["exa", "github"] {
        let plugin_code = format!(
            r#"
            var plugin = {{
                name: "{}",
                matchPatterns: ["api.{}.com"],
                credentialSchema: ["api_key"],
                transform: function(request, credentials) {{ return request; }}
            }};
        "#,
            plugin_name, plugin_name
        );
        store
            .set(&format!("plugin:{}", plugin_name), plugin_code.as_bytes())
            .await
            .expect("set plugin");

        // Add credentials for each plugin
        store
            .set(
                &format!("credential:{}:api_key", plugin_name),
                b"secret123",
            )
            .await
            .expect("set credential");
    }

    // Create registry and call migration
    let registry = Registry::new(store.clone() as Arc<dyn acp_lib::storage::SecretStore>);
    registry
        .migrate_from_file_store(&store)
        .await
        .expect("migration should succeed");

    // Verify all items were migrated
    let tokens = registry.list_tokens().await.expect("list tokens");
    assert_eq!(tokens.len(), 3, "should have migrated 3 tokens");

    let plugins = registry.list_plugins().await.expect("list plugins");
    assert_eq!(plugins.len(), 2, "should have migrated 2 plugins");

    let credentials = registry.list_credentials().await.expect("list credentials");
    assert_eq!(credentials.len(), 2, "should have migrated 2 credentials");
}

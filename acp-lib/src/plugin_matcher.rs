//! Plugin matching utilities
//!
//! Provides functions to find plugins that match a given host.

use crate::error::Result;
use crate::plugin_runtime::PluginRuntime;
use crate::registry::Registry;
use crate::storage::SecretStore;
use crate::types::ACPPlugin;

/// Find a plugin that matches the given host
///
/// Uses the Registry to list plugin metadata, then loads and checks match patterns.
/// Returns the first matching plugin, or None if no match is found.
///
/// # Arguments
/// * `host` - The hostname to match against (e.g., "api.example.com")
/// * `store` - SecretStore to load plugin code from
/// * `registry` - Registry to list available plugins
///
/// # Returns
/// Option containing the matching plugin, or None
pub async fn find_matching_plugin<S: SecretStore + ?Sized>(
    host: &str,
    store: &S,
    registry: &Registry,
) -> Result<Option<ACPPlugin>> {
    // Get all plugin entries from registry
    let plugin_entries = registry.list_plugins().await?;

    // Load and check each plugin
    for entry in plugin_entries {
        let key = format!("plugin:{}", entry.name);

        // Load plugin code from storage
        let plugin_code = store.get(&key).await?;
        if let Some(code_bytes) = plugin_code {
            let code = String::from_utf8_lossy(&code_bytes);

            // Create a runtime to extract metadata
            let mut runtime = PluginRuntime::new()?;
            if let Ok(plugin) = runtime.load_plugin_from_code(&entry.name, &code) {
                // Check if this plugin matches the host
                if plugin.matches_host(host) {
                    return Ok(Some(plugin));
                }
            }
        }
    }

    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{PluginEntry, Registry};
    use crate::storage::FileStore;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_find_matching_plugin_exact_match() {
        let temp_dir = std::env::temp_dir().join(format!(
            "acp_matcher_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        let store = Arc::new(FileStore::new(temp_dir.clone()).await.unwrap());
        let registry = Registry::new(Arc::clone(&store) as Arc<dyn SecretStore>);

        let plugin_code = r#"
        var plugin = {
            name: "test",
            matchPatterns: ["api.example.com"],
            credentialSchema: [],
            transform: function(request, credentials) { return request; }
        };
        "#;

        store.set("plugin:test", plugin_code.as_bytes()).await.unwrap();

        // Add to registry
        let entry = PluginEntry {
            name: "test".to_string(),
            hosts: vec!["api.example.com".to_string()],
            credential_schema: vec![],
        };
        registry.add_plugin(&entry).await.unwrap();

        let result = find_matching_plugin("api.example.com", &*store, &registry).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "test");

        tokio::fs::remove_dir_all(temp_dir).await.ok();
    }

    #[tokio::test]
    async fn test_find_matching_plugin_wildcard() {
        let temp_dir = std::env::temp_dir().join(format!(
            "acp_matcher_wildcard_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        let store = Arc::new(FileStore::new(temp_dir.clone()).await.unwrap());
        let registry = Registry::new(Arc::clone(&store) as Arc<dyn SecretStore>);

        let plugin_code = r#"
        var plugin = {
            name: "s3",
            matchPatterns: ["*.s3.amazonaws.com"],
            credentialSchema: [],
            transform: function(request, credentials) { return request; }
        };
        "#;

        store.set("plugin:s3", plugin_code.as_bytes()).await.unwrap();

        // Add to registry
        let entry = PluginEntry {
            name: "s3".to_string(),
            hosts: vec!["*.s3.amazonaws.com".to_string()],
            credential_schema: vec![],
        };
        registry.add_plugin(&entry).await.unwrap();

        let result = find_matching_plugin("bucket.s3.amazonaws.com", &*store, &registry).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().name, "s3");

        tokio::fs::remove_dir_all(temp_dir).await.ok();
    }

    #[tokio::test]
    async fn test_find_matching_plugin_no_match() {
        let temp_dir = std::env::temp_dir().join(format!(
            "acp_matcher_nomatch_test_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        let store = Arc::new(FileStore::new(temp_dir.clone()).await.unwrap());
        let registry = Registry::new(Arc::clone(&store) as Arc<dyn SecretStore>);

        let plugin_code = r#"
        var plugin = {
            name: "test",
            matchPatterns: ["api.example.com"],
            credentialSchema: [],
            transform: function(request, credentials) { return request; }
        };
        "#;

        store.set("plugin:test", plugin_code.as_bytes()).await.unwrap();

        // Add to registry
        let entry = PluginEntry {
            name: "test".to_string(),
            hosts: vec!["api.example.com".to_string()],
            credential_schema: vec![],
        };
        registry.add_plugin(&entry).await.unwrap();

        let result = find_matching_plugin("api.other.com", &*store, &registry).await.unwrap();
        assert!(result.is_none());

        tokio::fs::remove_dir_all(temp_dir).await.ok();
    }
}

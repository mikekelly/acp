//! Secure storage abstraction for secrets
//!
//! Provides a trait-based abstraction over platform-specific secret storage
//! mechanisms. Implementations include:
//! - FileStore: File-based storage with proper permissions (all platforms)
//! - KeychainStore: macOS Keychain integration (macOS only)
//!
//! The storage is used to persist:
//! - Plugin credentials (scoped by plugin name)
//! - Agent tokens
//! - CA private keys
//! - Password hashes

use crate::Result;
use async_trait::async_trait;
use std::path::PathBuf;

/// Trait for secure secret storage operations
///
/// All implementations must be async and support binary data.
/// Keys use namespacing with format: `type:name:key`
/// Examples: `credential:aws-s3:access_key`, `token:abc123`, `ca:private_key`
#[async_trait]
pub trait SecretStore: Send + Sync {
    /// Store a secret value
    ///
    /// # Arguments
    /// * `key` - Namespaced key (e.g., "credential:plugin:field")
    /// * `value` - Binary secret data
    async fn set(&self, key: &str, value: &[u8]) -> Result<()>;

    /// Retrieve a secret value
    ///
    /// Returns None if the key doesn't exist.
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>>;

    /// Delete a secret
    ///
    /// Returns Ok(()) even if the key doesn't exist (idempotent).
    async fn delete(&self, key: &str) -> Result<()>;
}

/// File-based secret storage implementation
///
/// Stores secrets as individual files in a directory with restrictive permissions.
/// Works on all platforms. Each secret is stored in a file named after its key
/// with directory separators encoded.
pub struct FileStore {
    base_path: PathBuf,
}

impl FileStore {
    /// Create a new FileStore at the given path
    ///
    /// The directory will be created if it doesn't exist, with mode 0700.
    pub async fn new(base_path: PathBuf) -> Result<Self> {
        tokio::fs::create_dir_all(&base_path).await?;

        // Set restrictive permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o700);
            std::fs::set_permissions(&base_path, perms)?;
        }

        Ok(Self { base_path })
    }

    /// Convert a key to a safe filename using base64url encoding
    fn key_to_filename(&self, key: &str) -> PathBuf {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;

        let encoded = URL_SAFE_NO_PAD.encode(key.as_bytes());
        self.base_path.join(encoded)
    }

    /// Convert a filename back to a key
    fn filename_to_key(&self, path: &std::path::Path) -> Option<String> {
        use base64::engine::general_purpose::URL_SAFE_NO_PAD;
        use base64::Engine;

        path.file_name()
            .and_then(|name| name.to_str())
            .and_then(|encoded| URL_SAFE_NO_PAD.decode(encoded).ok())
            .and_then(|bytes| String::from_utf8(bytes).ok())
    }
}

#[async_trait]
impl SecretStore for FileStore {
    async fn set(&self, key: &str, value: &[u8]) -> Result<()> {
        let path = self.key_to_filename(key);

        // Write to temp file first, then rename (atomic on Unix)
        let temp_path = path.with_extension("tmp");
        tokio::fs::write(&temp_path, value).await?;

        // Set restrictive permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            std::fs::set_permissions(&temp_path, perms)?;
        }

        tokio::fs::rename(&temp_path, &path).await?;
        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let path = self.key_to_filename(key);

        match tokio::fs::read(&path).await {
            Ok(data) => Ok(Some(data)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn delete(&self, key: &str) -> Result<()> {
        let path = self.key_to_filename(key);

        match tokio::fs::remove_file(&path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

impl FileStore {
    /// List all keys matching a prefix (internal use only)
    ///
    /// This method is kept for Phase 5 migration purposes (building registry from existing keys).
    /// It is not part of the SecretStore trait.
    ///
    /// # Arguments
    /// * `prefix` - Key prefix to match (e.g., "credential:aws-s3:")
    ///
    /// Returns sorted list of matching keys.
    #[allow(dead_code)]
    async fn list_internal(&self, prefix: &str) -> Result<Vec<String>> {
        let mut keys = Vec::new();

        let mut entries = tokio::fs::read_dir(&self.base_path).await?;
        while let Some(entry) = entries.next_entry().await? {
            if let Some(key) = self.filename_to_key(&entry.path()) {
                if key.starts_with(prefix) {
                    keys.push(key);
                }
            }
        }

        keys.sort();
        Ok(keys)
    }
}

/// macOS Keychain secret storage implementation
///
/// Uses the macOS Keychain to securely store secrets.
/// Only available on macOS.
#[cfg(target_os = "macos")]
pub struct KeychainStore {
    service_name: String,
}

#[cfg(target_os = "macos")]
impl KeychainStore {
    /// Create a new KeychainStore with the given service name
    ///
    /// The service name is used as a namespace for all keychain items.
    pub fn new(service_name: impl Into<String>) -> Result<Self> {
        Ok(Self {
            service_name: service_name.into(),
        })
    }
}

#[cfg(target_os = "macos")]
#[async_trait]
impl SecretStore for KeychainStore {
    async fn set(&self, key: &str, value: &[u8]) -> Result<()> {
        use security_framework::passwords::{delete_generic_password, set_generic_password};

        // Delete existing entry first (if any) to avoid conflicts
        let _ = delete_generic_password(&self.service_name, key);

        // Set the new password
        set_generic_password(&self.service_name, key, value)
            .map_err(|e| crate::AcpError::storage(format!("Keychain set failed: {}", e)))?;

        Ok(())
    }

    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>> {
        use security_framework::passwords::get_generic_password;

        match get_generic_password(&self.service_name, key) {
            Ok(password) => Ok(Some(password)),
            Err(e) => {
                // Check if it's a "not found" error
                let err_str = format!("{:?}", e);
                if err_str.contains("ItemNotFound") || err_str.contains("-25300") {
                    Ok(None)
                } else {
                    Err(crate::AcpError::storage(format!(
                        "Keychain get failed: {}",
                        e
                    )))
                }
            }
        }
    }

    async fn delete(&self, key: &str) -> Result<()> {
        use security_framework::passwords::delete_generic_password;

        match delete_generic_password(&self.service_name, key) {
            Ok(()) => Ok(()),
            Err(e) => {
                // Check if it's a "not found" error (idempotent)
                let err_str = format!("{:?}", e);
                if err_str.contains("ItemNotFound") || err_str.contains("-25300") {
                    Ok(())
                } else {
                    Err(crate::AcpError::storage(format!(
                        "Keychain delete failed: {}",
                        e
                    )))
                }
            }
        }
    }
}

/// Factory function to create the appropriate SecretStore implementation
///
/// On macOS, returns a KeychainStore by default. If `data_dir` is provided,
/// returns a FileStore instead (useful for containers/testing).
///
/// On other platforms, always returns a FileStore.
///
/// # Arguments
/// * `data_dir` - Optional directory for FileStore. If None on macOS, uses Keychain.
///   If None on other platforms, uses a default location.
pub async fn create_store(data_dir: Option<PathBuf>) -> Result<Box<dyn SecretStore>> {
    // Check for ACP_DATA_DIR environment variable first (useful for testing)
    if let Ok(env_path) = std::env::var("ACP_DATA_DIR") {
        let store = FileStore::new(PathBuf::from(env_path)).await?;
        return Ok(Box::new(store));
    }

    match data_dir {
        Some(path) => {
            // Explicit file storage requested
            let store = FileStore::new(path).await?;
            Ok(Box::new(store))
        }
        None => {
            // Platform-specific default
            #[cfg(target_os = "macos")]
            {
                let store = KeychainStore::new("com.acp.credentials")?;
                Ok(Box::new(store))
            }

            #[cfg(not(target_os = "macos"))]
            {
                // Use default location: ~/.acp/secrets
                let home = std::env::var("HOME")
                    .or_else(|_| std::env::var("USERPROFILE"))
                    .map_err(|_| crate::AcpError::storage("Cannot determine home directory"))?;
                let path = PathBuf::from(home).join(".acp").join("secrets");
                let store = FileStore::new(path).await?;
                Ok(Box::new(store))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test helper to verify SecretStore implementation
    async fn test_store_implementation<S: SecretStore>(store: S) {
        // Test set and get
        store
            .set("test:key1", b"value1")
            .await
            .expect("set should succeed");

        let value = store
            .get("test:key1")
            .await
            .expect("get should succeed")
            .expect("value should exist");
        assert_eq!(value, b"value1");

        // Test get non-existent key
        let missing = store
            .get("test:missing")
            .await
            .expect("get should succeed");
        assert!(missing.is_none(), "missing key should return None");

        // Test overwrite
        store
            .set("test:key1", b"value2")
            .await
            .expect("overwrite should succeed");
        let value = store
            .get("test:key1")
            .await
            .expect("get should succeed")
            .expect("value should exist");
        assert_eq!(value, b"value2");

        // Test binary data
        let binary_data = vec![0u8, 1, 2, 255, 128];
        store
            .set("test:binary", &binary_data)
            .await
            .expect("binary set should succeed");
        let retrieved = store
            .get("test:binary")
            .await
            .expect("get should succeed")
            .expect("value should exist");
        assert_eq!(retrieved, binary_data);

        // Test delete
        store
            .delete("test:key1")
            .await
            .expect("delete should succeed");
        let deleted = store
            .get("test:key1")
            .await
            .expect("get should succeed");
        assert!(deleted.is_none(), "deleted key should not exist");

        // Test delete idempotency
        store
            .delete("test:key1")
            .await
            .expect("second delete should succeed");

        // Cleanup
        store.delete("test:binary").await.ok();
    }

    #[tokio::test]
    async fn test_file_store() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let store = FileStore::new(temp_dir.path().to_path_buf())
            .await
            .expect("create FileStore");

        test_store_implementation(store).await;
    }

    #[tokio::test]
    async fn test_file_store_permissions() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;

            let temp_dir = tempfile::tempdir().expect("create temp dir");
            let store = FileStore::new(temp_dir.path().to_path_buf())
                .await
                .expect("create FileStore");

            // Check directory permissions
            let metadata = std::fs::metadata(temp_dir.path()).expect("get metadata");
            let mode = metadata.permissions().mode();
            assert_eq!(mode & 0o777, 0o700, "directory should have mode 0700");

            // Write a file and check permissions
            store
                .set("test:perm", b"value")
                .await
                .expect("set should succeed");

            let file_path = store.key_to_filename("test:perm");
            let file_metadata = std::fs::metadata(&file_path).expect("get file metadata");
            let file_mode = file_metadata.permissions().mode();
            assert_eq!(
                file_mode & 0o777,
                0o600,
                "file should have mode 0600"
            );

            store.delete("test:perm").await.ok();
        }
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn test_keychain_store() {
        // Use a unique service name for testing
        let service_name = format!("com.acp.test.{}", std::process::id());
        let store = KeychainStore::new(&service_name).expect("create KeychainStore");

        // Test basic operations (not list, since KeychainStore.list() returns empty)
        store
            .set("test:key1", b"value1")
            .await
            .expect("set should succeed");

        let value = store
            .get("test:key1")
            .await
            .expect("get should succeed")
            .expect("value should exist");
        assert_eq!(value, b"value1");

        // Test get non-existent key
        let missing = store
            .get("test:missing")
            .await
            .expect("get should succeed");
        assert!(missing.is_none(), "missing key should return None");

        // Test overwrite
        store
            .set("test:key1", b"value2")
            .await
            .expect("overwrite should succeed");
        let value = store
            .get("test:key1")
            .await
            .expect("get should succeed")
            .expect("value should exist");
        assert_eq!(value, b"value2");

        // Test binary data
        let binary_data = vec![0u8, 1, 2, 255, 128];
        store
            .set("test:binary", &binary_data)
            .await
            .expect("binary set should succeed");
        let retrieved = store
            .get("test:binary")
            .await
            .expect("get should succeed")
            .expect("value should exist");
        assert_eq!(retrieved, binary_data);

        // Test delete
        store
            .delete("test:key1")
            .await
            .expect("delete should succeed");
        let deleted = store
            .get("test:key1")
            .await
            .expect("get should succeed");
        assert!(deleted.is_none(), "deleted key should not exist");

        // Test delete idempotency
        store
            .delete("test:key1")
            .await
            .expect("second delete should succeed");

        // Cleanup
        let _ = store.delete("test:binary").await;
    }
}

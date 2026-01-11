//! Key-Value Store primitive
//!
//! Stateless facade over Database engine.
//! Provides working memory for agents: scratchpads, tool outputs, intermediate results.
//!
//! # Design
//!
//! KVStore is a stateless facade - it holds only an Arc<Database> reference.
//! Multiple KVStore instances sharing the same Database see the same data.
//! Clone is cheap (just Arc clone).
//!
//! # Example
//!
//! ```ignore
//! let db = Arc::new(Database::open(path)?);
//! let kv = KVStore::new(db.clone());
//!
//! // Begin a run
//! db.begin_run(run_id, vec![])?;
//!
//! // Use KV store
//! kv.put(run_id, "key", b"value")?;
//! let value = kv.get(run_id, "key")?;
//! ```

use in_mem_core::{error::Result, types::RunId, value::Value};
use in_mem_engine::Database;
use std::sync::Arc;
use std::time::Duration;

/// Key-Value Store primitive
///
/// Stateless facade over Database engine.
/// Provides working memory for agents: scratchpads, tool outputs, intermediate results.
///
/// # Thread Safety
///
/// KVStore is Clone and Send + Sync. Multiple instances sharing the same
/// Database reference will see the same data (no local state).
#[derive(Clone)]
pub struct KVStore {
    /// Database reference (shared)
    db: Arc<Database>,
}

impl KVStore {
    /// Create a new KV store facade
    ///
    /// # Arguments
    ///
    /// * `db` - Shared database reference
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    /// Get value by key
    ///
    /// Returns the raw bytes if the key exists and contains a Bytes value.
    /// Returns None if the key doesn't exist or has expired (TTL).
    ///
    /// # Arguments
    ///
    /// * `run_id` - The run this operation belongs to
    /// * `key` - The key to look up
    ///
    /// # Returns
    ///
    /// The value bytes if found, None otherwise
    pub fn get(&self, run_id: RunId, key: impl AsRef<[u8]>) -> Result<Option<Vec<u8>>> {
        if let Some(Value::Bytes(bytes)) = self.db.get(run_id, key)? {
            Ok(Some(bytes))
        } else {
            Ok(None)
        }
    }

    /// Put a key-value pair
    ///
    /// Stores the value as raw bytes. Overwrites any existing value.
    ///
    /// # Arguments
    ///
    /// * `run_id` - The run this operation belongs to
    /// * `key` - The key to store under
    /// * `value` - The value bytes to store
    ///
    /// # Returns
    ///
    /// The version number assigned to this write
    pub fn put(
        &self,
        run_id: RunId,
        key: impl AsRef<[u8]>,
        value: impl Into<Vec<u8>>,
    ) -> Result<u64> {
        self.db.put(run_id, key, Value::Bytes(value.into()))
    }

    /// Put a key-value pair with TTL
    ///
    /// Stores the value with a time-to-live. After the TTL expires,
    /// get() will return None.
    ///
    /// # Arguments
    ///
    /// * `run_id` - The run this operation belongs to
    /// * `key` - The key to store under
    /// * `value` - The value bytes to store
    /// * `ttl` - Time-to-live duration
    ///
    /// # Returns
    ///
    /// The version number assigned to this write
    pub fn put_with_ttl(
        &self,
        run_id: RunId,
        key: impl AsRef<[u8]>,
        value: impl Into<Vec<u8>>,
        ttl: Duration,
    ) -> Result<u64> {
        self.db
            .put_with_ttl(run_id, key, Value::Bytes(value.into()), Some(ttl))
    }

    /// Delete a key
    ///
    /// Removes the key and returns its previous value if it existed.
    ///
    /// # Arguments
    ///
    /// * `run_id` - The run this operation belongs to
    /// * `key` - The key to delete
    ///
    /// # Returns
    ///
    /// The previous value bytes if the key existed, None otherwise
    pub fn delete(&self, run_id: RunId, key: impl AsRef<[u8]>) -> Result<Option<Vec<u8>>> {
        if let Some(Value::Bytes(bytes)) = self.db.delete(run_id, key)? {
            Ok(Some(bytes))
        } else {
            Ok(None)
        }
    }

    /// List keys with a prefix
    ///
    /// Returns all key-value pairs where the key starts with the given prefix.
    /// Only returns entries with Bytes values.
    ///
    /// # Arguments
    ///
    /// * `run_id` - The run this operation belongs to
    /// * `prefix` - The key prefix to match
    ///
    /// # Returns
    ///
    /// Vector of (key, value) pairs matching the prefix
    pub fn list(&self, run_id: RunId, prefix: impl AsRef<[u8]>) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        let entries = self.db.list(run_id, prefix)?;

        Ok(entries
            .into_iter()
            .filter_map(|(k, v)| {
                if let Value::Bytes(bytes) = v {
                    Some((k, bytes))
                } else {
                    None
                }
            })
            .collect())
    }

    /// Check if a key exists
    ///
    /// # Arguments
    ///
    /// * `run_id` - The run this operation belongs to
    /// * `key` - The key to check
    ///
    /// # Returns
    ///
    /// true if the key exists and has not expired, false otherwise
    pub fn exists(&self, run_id: RunId, key: impl AsRef<[u8]>) -> Result<bool> {
        Ok(self.get(run_id, key)?.is_some())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_kv() -> (TempDir, KVStore, RunId) {
        let temp_dir = TempDir::new().unwrap();
        let db = Arc::new(Database::open(temp_dir.path()).unwrap());
        let kv = KVStore::new(db.clone());
        let run_id = RunId::new();
        db.begin_run(run_id, vec![]).unwrap();
        (temp_dir, kv, run_id)
    }

    #[test]
    fn test_kv_put_and_get() {
        let (_dir, kv, run_id) = setup_kv();

        kv.put(run_id, "key1", b"value1".to_vec()).unwrap();

        let value = kv.get(run_id, "key1").unwrap().unwrap();
        assert_eq!(value, b"value1");
    }

    #[test]
    fn test_kv_get_nonexistent() {
        let (_dir, kv, run_id) = setup_kv();

        let value = kv.get(run_id, "nonexistent").unwrap();
        assert!(value.is_none());
    }

    #[test]
    fn test_kv_delete() {
        let (_dir, kv, run_id) = setup_kv();

        kv.put(run_id, "key1", b"value1".to_vec()).unwrap();

        let deleted = kv.delete(run_id, "key1").unwrap();
        assert_eq!(deleted.unwrap(), b"value1");

        let value = kv.get(run_id, "key1").unwrap();
        assert!(value.is_none());
    }

    #[test]
    fn test_kv_put_with_ttl() {
        let (_dir, kv, run_id) = setup_kv();

        // Use 1 second TTL (minimum supported by second-level precision)
        kv.put_with_ttl(
            run_id,
            "temp",
            b"temporary".to_vec(),
            Duration::from_secs(1),
        )
        .unwrap();

        // Should exist initially
        assert!(kv.get(run_id, "temp").unwrap().is_some());

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(1100));

        // Should be expired
        assert!(kv.get(run_id, "temp").unwrap().is_none());
    }

    #[test]
    fn test_kv_list() {
        let (_dir, kv, run_id) = setup_kv();

        kv.put(run_id, "user:alice", b"alice_data".to_vec())
            .unwrap();
        kv.put(run_id, "user:bob", b"bob_data".to_vec()).unwrap();
        kv.put(run_id, "config:foo", b"config_data".to_vec())
            .unwrap();

        let entries = kv.list(run_id, "user:").unwrap();
        assert_eq!(entries.len(), 2);

        // Verify both user entries present
        let keys: Vec<Vec<u8>> = entries.iter().map(|(k, _)| k.clone()).collect();
        assert!(keys.contains(&b"user:alice".to_vec()));
        assert!(keys.contains(&b"user:bob".to_vec()));
    }

    #[test]
    fn test_kv_exists() {
        let (_dir, kv, run_id) = setup_kv();

        kv.put(run_id, "key1", b"value1".to_vec()).unwrap();

        assert!(kv.exists(run_id, "key1").unwrap());
        assert!(!kv.exists(run_id, "key2").unwrap());
    }

    #[test]
    fn test_kv_update() {
        let (_dir, kv, run_id) = setup_kv();

        kv.put(run_id, "counter", b"1".to_vec()).unwrap();
        kv.put(run_id, "counter", b"2".to_vec()).unwrap();
        kv.put(run_id, "counter", b"3".to_vec()).unwrap();

        let value = kv.get(run_id, "counter").unwrap().unwrap();
        assert_eq!(value, b"3");
    }

    #[test]
    fn test_kv_is_stateless() {
        let temp_dir = TempDir::new().unwrap();
        let db = Arc::new(Database::open(temp_dir.path()).unwrap());
        let run_id = RunId::new();
        db.begin_run(run_id, vec![]).unwrap();

        // Create two KV instances
        let kv1 = KVStore::new(db.clone());
        let kv2 = KVStore::new(db.clone());

        // Write via kv1
        kv1.put(run_id, "shared", b"data".to_vec()).unwrap();

        // Read via kv2 (should see same data - no state in KV)
        let value = kv2.get(run_id, "shared").unwrap().unwrap();
        assert_eq!(value, b"data");
    }

    #[test]
    fn test_kv_clone() {
        let (_dir, kv, run_id) = setup_kv();

        // Clone is cheap (just Arc clone)
        let kv_clone = kv.clone();

        kv.put(run_id, "key1", b"value1".to_vec()).unwrap();

        // Both see same data
        assert_eq!(kv.get(run_id, "key1").unwrap().unwrap(), b"value1");
        assert_eq!(kv_clone.get(run_id, "key1").unwrap().unwrap(), b"value1");
    }

    #[test]
    fn test_kv_different_runs() {
        let temp_dir = TempDir::new().unwrap();
        let db = Arc::new(Database::open(temp_dir.path()).unwrap());
        let kv = KVStore::new(db.clone());

        let run1 = RunId::new();
        let run2 = RunId::new();

        db.begin_run(run1, vec![]).unwrap();
        db.begin_run(run2, vec![]).unwrap();

        // Write to run1
        kv.put(run1, "shared_key", b"run1_value".to_vec()).unwrap();

        // Write to run2
        kv.put(run2, "shared_key", b"run2_value".to_vec()).unwrap();

        // Each run sees its own value
        assert_eq!(kv.get(run1, "shared_key").unwrap().unwrap(), b"run1_value");
        assert_eq!(kv.get(run2, "shared_key").unwrap().unwrap(), b"run2_value");
    }
}

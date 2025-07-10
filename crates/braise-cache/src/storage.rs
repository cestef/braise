use std::path::Path;
use std::sync::Arc;

use log::debug;
use sled::Db;

use crate::{CacheEntry, CacheResult};
use braise_errors::CacheError;

// Helper functions for error conversion
fn convert_sled_error(err: sled::Error) -> CacheError {
    CacheError::storage(err.to_string())
}

fn convert_serde_error(err: serde_json::Error) -> CacheError {
    CacheError::serialization(err.to_string())
}

pub trait CacheStorage: Send + Sync {
    fn get(&self, key: &str) -> CacheResult<Option<CacheEntry>>;
    fn set(&self, key: &str, entry: CacheEntry) -> CacheResult<()>;
    fn remove(&self, key: &str) -> CacheResult<bool>;
    fn clear(&self) -> CacheResult<()>;
    fn keys(&self) -> CacheResult<Vec<String>>;
    fn size(&self) -> CacheResult<u64>;
    fn cleanup_expired(&self) -> CacheResult<u64>;
}

pub struct SledStorage {
    db: Arc<Db>,
}

impl SledStorage {
    pub fn new<P: AsRef<Path>>(path: P) -> CacheResult<Self> {
        let db = sled::open(path).map_err(convert_sled_error)?;
        Ok(Self { db: Arc::new(db) })
    }

    pub fn in_memory() -> CacheResult<Self> {
        let config = sled::Config::new().temporary(true);
        let db = config.open().map_err(convert_sled_error)?;
        Ok(Self { db: Arc::new(db) })
    }

    pub fn flush(&self) -> CacheResult<()> {
        self.db.flush().map_err(convert_sled_error)?;
        Ok(())
    }

    pub fn compact(&self) -> CacheResult<()> {
        // Sled doesn't have explicit compaction, but we can trigger cleanup
        self.cleanup_expired()?;
        Ok(())
    }

    fn serialize_entry(&self, entry: &CacheEntry) -> CacheResult<Vec<u8>> {
        serde_json::to_vec(entry).map_err(convert_serde_error)
    }

    fn deserialize_entry(&self, data: &[u8]) -> CacheResult<CacheEntry> {
        serde_json::from_slice(data).map_err(convert_serde_error)
    }
}

impl CacheStorage for SledStorage {
    fn get(&self, key: &str) -> CacheResult<Option<CacheEntry>> {
        debug!("Cache GET: {}", key);

        match self.db.get(key.as_bytes()).map_err(convert_sled_error)? {
            Some(data) => {
                let entry = self.deserialize_entry(&data)?;

                // Check if entry is expired
                if entry.is_expired() {
                    debug!("Cache entry expired: {}", key);
                    // Remove expired entry
                    self.db.remove(key.as_bytes()).map_err(convert_sled_error)?;
                    return Ok(None);
                }

                debug!("Cache HIT: {}", key);
                Ok(Some(entry))
            }
            None => {
                debug!("Cache MISS: {}", key);
                Ok(None)
            }
        }
    }

    fn set(&self, key: &str, entry: CacheEntry) -> CacheResult<()> {
        debug!("Cache SET: {}", key);

        let data = self.serialize_entry(&entry)?;
        self.db
            .insert(key.as_bytes(), data)
            .map_err(convert_sled_error)?;

        // Ensure data is persisted
        self.db.flush().map_err(convert_sled_error)?;

        Ok(())
    }

    fn remove(&self, key: &str) -> CacheResult<bool> {
        debug!("Cache REMOVE: {}", key);

        match self.db.remove(key.as_bytes()).map_err(convert_sled_error)? {
            Some(_) => Ok(true),
            None => Ok(false),
        }
    }

    fn clear(&self) -> CacheResult<()> {
        debug!("Cache CLEAR");

        self.db.clear().map_err(convert_sled_error)?;
        self.db.flush().map_err(convert_sled_error)?;

        Ok(())
    }

    fn keys(&self) -> CacheResult<Vec<String>> {
        let mut keys = Vec::new();

        for result in self.db.iter() {
            let (key, _) = result.map_err(convert_sled_error)?;
            if let Ok(key_str) = String::from_utf8(key.to_vec()) {
                keys.push(key_str);
            }
        }

        Ok(keys)
    }

    fn size(&self) -> CacheResult<u64> {
        // Estimate size by summing key and value lengths
        let mut total_size = 0u64;

        for result in self.db.iter() {
            let (key, value) = result.map_err(convert_sled_error)?;
            total_size += key.len() as u64 + value.len() as u64;
        }

        Ok(total_size)
    }

    fn cleanup_expired(&self) -> CacheResult<u64> {
        debug!("Cleaning up expired cache entries");

        let mut removed_count = 0u64;
        let mut keys_to_remove = Vec::new();

        // First pass: identify expired entries
        for result in self.db.iter() {
            let (key, value) = result.map_err(convert_sled_error)?;

            if let Ok(entry) = self.deserialize_entry(&value)
                && entry.is_expired()
                && let Ok(key_str) = String::from_utf8(key.to_vec())
            {
                keys_to_remove.push(key_str);
            }
        }

        // Second pass: remove expired entries
        for key in keys_to_remove {
            if self.remove(&key)? {
                removed_count += 1;
            }
        }

        if removed_count > 0 {
            debug!("Removed {} expired cache entries", removed_count);
            self.db.flush().map_err(convert_sled_error)?;
        }

        Ok(removed_count)
    }
}

impl Clone for SledStorage {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CacheValue;
    use braise_types::TypedValue;
    use std::time::{Duration, SystemTime};
    use tempfile::tempdir;

    fn create_test_entry(key: &str, value: &str) -> CacheEntry {
        CacheEntry {
            key: key.to_string(),
            value: CacheValue::BuiltinResult(TypedValue::string(value)),
            created_at: SystemTime::now(),
            ttl: None,
            dependencies: vec![],
        }
    }

    fn create_test_entry_with_ttl(key: &str, value: &str, ttl: Duration) -> CacheEntry {
        CacheEntry {
            key: key.to_string(),
            value: CacheValue::BuiltinResult(TypedValue::string(value)),
            created_at: SystemTime::now(),
            ttl: Some(ttl),
            dependencies: vec![],
        }
    }

    #[test]
    fn test_in_memory_storage() {
        let storage = SledStorage::in_memory().unwrap();

        // Test set and get
        let entry = create_test_entry("test_key", "test_value");
        storage.set("test_key", entry).unwrap();

        let retrieved = storage.get("test_key").unwrap().unwrap();
        assert_eq!(retrieved.key, "test_key");
    }

    #[test]
    fn test_persistent_storage() {
        let temp_dir = tempdir().unwrap();
        let storage = SledStorage::new(temp_dir.path().join("test_cache")).unwrap();

        // Test set and get
        let entry = create_test_entry("persistent_key", "persistent_value");
        storage.set("persistent_key", entry).unwrap();

        let retrieved = storage.get("persistent_key").unwrap().unwrap();
        assert_eq!(retrieved.key, "persistent_key");
    }

    #[test]
    fn test_expiration() {
        let storage = SledStorage::in_memory().unwrap();

        // Create entry with very short TTL
        let entry =
            create_test_entry_with_ttl("expiring_key", "expiring_value", Duration::from_millis(1));
        storage.set("expiring_key", entry).unwrap();

        // Wait for expiration
        std::thread::sleep(Duration::from_millis(10));

        // Entry should be expired and removed
        let retrieved = storage.get("expiring_key").unwrap();
        assert!(retrieved.is_none());
    }

    #[test]
    fn test_cleanup_expired() {
        let storage = SledStorage::in_memory().unwrap();

        // Add some entries with different TTLs
        let entry1 = create_test_entry_with_ttl("key1", "value1", Duration::from_millis(1));
        let entry2 = create_test_entry_with_ttl("key2", "value2", Duration::from_secs(3600));
        let entry3 = create_test_entry("key3", "value3"); // No TTL

        storage.set("key1", entry1).unwrap();
        storage.set("key2", entry2).unwrap();
        storage.set("key3", entry3).unwrap();

        // Wait for first entry to expire
        std::thread::sleep(Duration::from_millis(10));

        // Cleanup should remove 1 expired entry
        let removed_count = storage.cleanup_expired().unwrap();
        assert_eq!(removed_count, 1);

        // Verify remaining entries
        assert!(storage.get("key1").unwrap().is_none());
        assert!(storage.get("key2").unwrap().is_some());
        assert!(storage.get("key3").unwrap().is_some());
    }

    #[test]
    fn test_clear() {
        let storage = SledStorage::in_memory().unwrap();

        // Add some entries
        storage
            .set("key1", create_test_entry("key1", "value1"))
            .unwrap();
        storage
            .set("key2", create_test_entry("key2", "value2"))
            .unwrap();

        // Clear all
        storage.clear().unwrap();

        // Verify all entries are removed
        assert!(storage.get("key1").unwrap().is_none());
        assert!(storage.get("key2").unwrap().is_none());
    }

    #[test]
    fn test_keys() {
        let storage = SledStorage::in_memory().unwrap();

        // Add some entries
        storage
            .set("key1", create_test_entry("key1", "value1"))
            .unwrap();
        storage
            .set("key2", create_test_entry("key2", "value2"))
            .unwrap();

        // Get all keys
        let keys = storage.keys().unwrap();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"key1".to_string()));
        assert!(keys.contains(&"key2".to_string()));
    }
}

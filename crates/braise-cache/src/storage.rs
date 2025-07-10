use std::{path::Path, sync::Arc};

use bincode::config::Configuration;
use log::debug;

use crate::{CacheEntry, CacheResult};
use braise_errors::CacheError;
use redb::{Database, ReadableTable, TableDefinition};

pub trait CacheStorage: Send + Sync {
    fn get(&self, key: &str) -> CacheResult<Option<CacheEntry>>;
    fn set(&self, key: &str, entry: CacheEntry) -> CacheResult<()>;
    fn remove(&self, key: &str) -> CacheResult<bool>;
    fn clear(&self) -> CacheResult<()>;
    fn keys(&self) -> CacheResult<Vec<String>>;
    fn size(&self) -> CacheResult<u64>;
    fn cleanup_expired(&self) -> CacheResult<u64>;
}

fn db_error(e: redb::DatabaseError) -> CacheError {
    CacheError::storage(e.to_string())
}

fn tx_error(e: redb::TransactionError) -> CacheError {
    CacheError::storage(e.to_string())
}

fn table_error(e: redb::TableError) -> CacheError {
    CacheError::storage(e.to_string())
}

fn st_error(e: redb::StorageError) -> CacheError {
    CacheError::storage(e.to_string())
}

fn commit_error(e: redb::CommitError) -> CacheError {
    CacheError::storage(e.to_string())
}

pub struct RedbStorage {
    db: Arc<Database>,
    config: Configuration,
}

impl RedbStorage {
    pub fn new<P: AsRef<Path>>(path: P) -> CacheResult<Self> {
        let db = Database::create(path).map_err(db_error)?.into();
        let config = Configuration::default();
        Ok(Self { db, config })
    }

    pub fn compact(&self) -> CacheResult<()> {
        // Sled doesn't have explicit compaction, but we can trigger cleanup
        self.cleanup_expired()?;
        Ok(())
    }

    fn serialize_entry(&self, entry: &CacheEntry) -> CacheResult<Vec<u8>> {
        bincode::encode_to_vec(entry, self.config).map_err(|e| {
            CacheError::serialization(format!("Failed to serialize cache entry: {}", e))
        })
    }

    fn deserialize_entry(&self, data: &[u8]) -> CacheResult<CacheEntry> {
        bincode::decode_from_slice(data, self.config)
            .map_err(|e| {
                CacheError::serialization(format!("Failed to deserialize cache entry: {}", e))
            })
            .map(|(entry, _)| entry)
    }
}

const TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("cache");

impl CacheStorage for RedbStorage {
    fn get(&self, key: &str) -> CacheResult<Option<CacheEntry>> {
        debug!("Cache GET: {}", key);

        let read_txn = self.db.begin_read().map_err(tx_error)?;
        let table = read_txn.open_table(TABLE).map_err(table_error)?;
        match table.get(key.as_bytes()).map_err(st_error)? {
            Some(data) => {
                let entry = self.deserialize_entry(data.value())?;
                if entry.is_expired() {
                    debug!("Cache entry expired: {}", key);
                    drop(read_txn); // End read txn before write
                    let write_txn = self.db.begin_write().map_err(tx_error)?;
                    let mut table = write_txn.open_table(TABLE).map_err(table_error)?;
                    table.remove(key.as_bytes()).map_err(st_error)?;
                    drop(table); // Release the mutable borrow before committing
                    write_txn.commit().map_err(commit_error)?;
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

        let write_txn = self.db.begin_write().map_err(tx_error)?;
        let mut table = write_txn.open_table(TABLE).map_err(table_error)?;
        let serialized_entry = self.serialize_entry(&entry)?;
        table
            .insert(key.as_bytes(), &*serialized_entry)
            .map_err(st_error)?;
        drop(table); // Release the mutable borrow before committing
        write_txn.commit().map_err(commit_error)?;

        debug!("Cache entry set: {}", key);
        Ok(())
    }

    fn remove(&self, key: &str) -> CacheResult<bool> {
        debug!("Cache REMOVE: {}", key);

        let write_txn = self.db.begin_write().map_err(tx_error)?;
        let mut table = write_txn.open_table(TABLE).map_err(table_error)?;
        let existed = table.remove(key.as_bytes()).map_err(st_error)?.is_some();
        drop(table);
        write_txn.commit().map_err(commit_error)?;

        debug!("Cache entry removed: {} (existed: {})", key, existed);
        Ok(existed)
    }

    fn clear(&self) -> CacheResult<()> {
        debug!("Cache CLEAR: clearing all entries");

        let write_txn = self.db.begin_write().map_err(tx_error)?;
        let mut table = write_txn.open_table(TABLE).map_err(table_error)?;
        
        // Get all keys first
        let entries = table.iter().map_err(st_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(st_error)?;
            
        let keys: Vec<Vec<u8>> = entries
            .into_iter()
            .map(|(k, _)| k.value().to_vec())
            .collect();
        
        // Remove all entries
        for key in keys {
            table.remove(&*key).map_err(st_error)?;
        }
        
        drop(table);
        write_txn.commit().map_err(commit_error)?;

        debug!("Cache cleared");
        Ok(())
    }

    fn keys(&self) -> CacheResult<Vec<String>> {
        debug!("Cache KEYS: getting all keys");

        let read_txn = self.db.begin_read().map_err(tx_error)?;
        let table = read_txn.open_table(TABLE).map_err(table_error)?;
        
        let entries = table.iter().map_err(st_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(st_error)?;
            
        let keys: Result<Vec<String>, _> = entries
            .into_iter()
            .map(|(k, _)| String::from_utf8(k.value().to_vec()).map_err(|e| {
                CacheError::storage(format!("Invalid UTF-8 key: {}", e))
            }))
            .collect();

        let keys = keys?;
        debug!("Cache keys retrieved: {} keys", keys.len());
        Ok(keys)
    }

    fn size(&self) -> CacheResult<u64> {
        debug!("Cache SIZE: getting cache size");

        let read_txn = self.db.begin_read().map_err(tx_error)?;
        let table = read_txn.open_table(TABLE).map_err(table_error)?;
        
        let entries = table.iter().map_err(st_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(st_error)?;
            
        let count = entries.len() as u64;

        debug!("Cache size: {} entries", count);
        Ok(count)
    }

    fn cleanup_expired(&self) -> CacheResult<u64> {
        debug!("Cache CLEANUP: removing expired entries");

        let read_txn = self.db.begin_read().map_err(tx_error)?;
        let table = read_txn.open_table(TABLE).map_err(table_error)?;
        
        // Find expired keys
        let entries = table.iter().map_err(st_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(st_error)?;
            
        let expired_keys: Result<Vec<Vec<u8>>, _> = entries
            .into_iter()
            .filter_map(|(k, v)| {
                match self.deserialize_entry(v.value()) {
                    Ok(entry) if entry.is_expired() => Some(Ok(k.value().to_vec())),
                    Ok(_) => None,
                    Err(e) => Some(Err(e)),
                }
            })
            .collect();

        let expired_keys = expired_keys?;
        let expired_count = expired_keys.len() as u64;
        
        drop(table);
        drop(read_txn);

        if !expired_keys.is_empty() {
            let write_txn = self.db.begin_write().map_err(tx_error)?;
            let mut table = write_txn.open_table(TABLE).map_err(table_error)?;
            
            for key in expired_keys {
                table.remove(&*key).map_err(st_error)?;
            }
            
            drop(table);
            write_txn.commit().map_err(commit_error)?;
        }

        debug!("Cache cleanup completed: {} expired entries removed", expired_count);
        Ok(expired_count)
    }
}

impl Clone for RedbStorage {
    fn clone(&self) -> Self {
        Self {
            db: self.db.clone(),
            config: self.config,
        }
    }
}

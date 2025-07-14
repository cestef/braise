use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use log::{debug, info, warn};

use crate::{
    CacheConfig, CacheDependency, CacheEntry, CacheKeyGenerator, CacheLevel, CacheResult,
    CacheStorage, CacheValue, RedbStorage,
};
use braise_errors::CacheError;
use braise_types::TypedValue;

pub struct CacheManager {
    storage: Arc<dyn CacheStorage>,
    config: CacheConfig,
    stats: Arc<Mutex<CacheStats>>,
}

#[derive(Debug, Default, Clone)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub evictions: u64,
    pub errors: u64,
    pub total_requests: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.hits as f64 / self.total_requests as f64
        }
    }

    pub fn miss_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            self.misses as f64 / self.total_requests as f64
        }
    }
}

impl CacheManager {
    pub fn new(config: CacheConfig) -> CacheResult<Self> {
        let storage = Arc::new(RedbStorage::new(&config.storage_path)?);
        let stats = Arc::new(Mutex::new(CacheStats::default()));

        Ok(Self {
            storage,
            config,
            stats,
        })
    }

    pub fn with_storage(storage: Arc<dyn CacheStorage>, config: CacheConfig) -> CacheResult<Self> {
        let stats = Arc::new(Mutex::new(CacheStats::default()));

        Ok(Self {
            storage,
            config,
            stats,
        })
    }

    pub fn get(&self, key: &str) -> CacheResult<Option<CacheValue>> {
        self.increment_total_requests();

        if !self.config.enabled {
            self.increment_misses();
            return Ok(None);
        }

        let level = CacheKeyGenerator::extract_level_from_key(key)
            .ok_or_else(|| CacheError::invalid_key(key))?;

        if !self.config.should_cache_level(&level) {
            self.increment_misses();
            return Ok(None);
        }

        if self.config.should_ignore_key(key) {
            self.increment_misses();
            return Ok(None);
        }

        match self.storage.get(key)? {
            Some(entry) => {
                debug!("Cache HIT for key: {key}");
                self.increment_hits();
                Ok(Some(entry.value))
            }
            None => {
                debug!("Cache MISS for key: {key}");
                self.increment_misses();
                Ok(None)
            }
        }
    }

    pub fn set(
        &self,
        key: &str,
        value: CacheValue,
        dependencies: Vec<CacheDependency>,
    ) -> CacheResult<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let level = CacheKeyGenerator::extract_level_from_key(key)
            .ok_or_else(|| CacheError::invalid_key(key))?;

        if !self.config.should_cache_level(&level) {
            return Ok(());
        }

        if self.config.should_ignore_key(key) {
            return Ok(());
        }

        // Check cache size limits
        self.enforce_size_limits()?;

        // Create cache entry
        let mut entry = CacheEntry::new(key.to_string(), value);

        // Set TTL based on configuration
        if let Some(ttl) = self.config.get_ttl_for_key(key) {
            entry = entry.with_ttl(ttl);
        }

        // Add dependencies
        entry = entry.with_dependencies(dependencies.clone());

        // Store the entry
        self.storage.set(key, entry)?;

        debug!("Cache SET for key: {key}");
        Ok(())
    }

    pub fn invalidate(&self, key: &str) -> CacheResult<bool> {
        debug!("Invalidating cache key: {key}");

        self.storage.remove(key)
    }

    pub fn clear(&self) -> CacheResult<()> {
        info!("Clearing entire cache");

        // Clear storage
        self.storage.clear()?;

        // Reset stats
        if let Ok(mut stats) = self.stats.lock() {
            *stats = CacheStats::default();
        }

        Ok(())
    }

    pub fn clear_by_pattern(&self, pattern: &str) -> CacheResult<u64> {
        debug!("Clearing cache entries matching pattern: {pattern}");

        let keys = self.storage.keys()?;
        let mut removed_count = 0;

        for key in keys {
            if key.contains(pattern) && self.invalidate(&key)? {
                removed_count += 1;
            }
        }

        info!(
            "Cleared {removed_count} cache entries matching pattern: {pattern}"
        );
        Ok(removed_count)
    }

    pub fn cleanup_expired(&self) -> CacheResult<u64> {
        debug!("Cleaning up expired cache entries");

        let removed_count = self.storage.cleanup_expired()?;

        if removed_count > 0 {
            info!("Cleaned up {removed_count} expired cache entries");

            if let Ok(mut stats) = self.stats.lock() {
                stats.evictions += removed_count;
            }
        }

        Ok(removed_count)
    }

    pub fn get_stats(&self) -> CacheStats {
        self.stats
            .lock()
            .map(|stats| stats.clone())
            .unwrap_or_default()
    }

    pub fn get_size(&self) -> CacheResult<u64> {
        self.storage.size()
    }

    pub fn get_config(&self) -> &CacheConfig {
        &self.config
    }

    pub fn update_config(&mut self, config: CacheConfig) {
        self.config = config;
    }

    pub fn get_cache_keys(&self) -> CacheResult<Vec<String>> {
        self.storage.keys()
    }

    pub fn get_cache_keys_by_level(&self, level: CacheLevel) -> CacheResult<Vec<String>> {
        let all_keys = self.storage.keys()?;
        let prefix = match level {
            CacheLevel::Recipe => "recipe:",
            CacheLevel::Command => "cmd:",
            CacheLevel::Builtin => "builtin:",
            CacheLevel::Expression => "expr:",
        };

        Ok(all_keys
            .into_iter()
            .filter(|key| key.starts_with(prefix))
            .collect())
    }

    fn enforce_size_limits(&self) -> CacheResult<()> {
        let current_size = self.storage.size()?;

        if current_size > self.config.max_size {
            warn!(
                "Cache size limit exceeded: {} bytes (limit: {} bytes)",
                current_size, self.config.max_size
            );

            // Simple eviction strategy: remove expired entries first
            let removed_count = self.cleanup_expired()?;

            if removed_count > 0 {
                debug!("Removed {removed_count} expired entries to free space");
            }

            // If still over limit, we could implement LRU or other strategies
            let new_size = self.storage.size()?;
            if new_size > self.config.max_size {
                return Err(CacheError::size_limit_exceeded(
                    new_size,
                    self.config.max_size,
                ));
            }
        }

        Ok(())
    }

    fn increment_total_requests(&self) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.total_requests += 1;
        }
    }

    fn increment_hits(&self) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.hits += 1;
        }
    }

    fn increment_misses(&self) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.misses += 1;
        }
    }
}

impl Clone for CacheManager {
    fn clone(&self) -> Self {
        Self {
            storage: Arc::clone(&self.storage),
            config: self.config.clone(),
            stats: Arc::clone(&self.stats),
        }
    }
}

// Convenience methods for different cache levels
impl CacheManager {
    pub fn get_recipe_result(
        &self,
        recipe_name: &str,
        params: &HashMap<String, TypedValue>,
        file_hashes: &HashMap<PathBuf, String>,
    ) -> CacheResult<Option<Result<(), String>>> {
        let key = CacheKeyGenerator::generate_recipe_key(recipe_name, params, file_hashes);

        match self.get(&key)? {
            Some(CacheValue::RecipeResult(result)) => Ok(Some(result)),
            Some(_) => Err(CacheError::invalid_key(format!(
                "Expected recipe result for key: {key}"
            ))),
            None => Ok(None),
        }
    }

    pub fn set_recipe_result(
        &self,
        recipe_name: &str,
        params: &HashMap<String, TypedValue>,
        file_hashes: &HashMap<PathBuf, String>,
        result: Result<(), String>,
        dependencies: Vec<CacheDependency>,
    ) -> CacheResult<()> {
        let key = CacheKeyGenerator::generate_recipe_key(recipe_name, params, file_hashes);
        let value = CacheValue::RecipeResult(result);

        self.set(&key, value, dependencies)
    }

    pub fn get_command_output(
        &self,
        command: &str,
        working_dir: &Path,
        env_vars: &HashMap<String, String>,
    ) -> CacheResult<Option<crate::CommandOutput>> {
        let key = CacheKeyGenerator::generate_command_key(command, working_dir, env_vars);

        match self.get(&key)? {
            Some(CacheValue::CommandOutput {
                stdout,
                stderr,
                exit_code,
            }) => Ok(Some(crate::CommandOutput {
                stdout,
                stderr,
                exit_code,
            })),
            Some(_) => Err(CacheError::invalid_key(format!(
                "Expected command output for key: {key}"
            ))),
            None => Ok(None),
        }
    }

    pub fn set_command_output(
        &self,
        command: &str,
        working_dir: &Path,
        env_vars: &HashMap<String, String>,
        output: crate::CommandOutput,
        dependencies: Vec<CacheDependency>,
    ) -> CacheResult<()> {
        let key = CacheKeyGenerator::generate_command_key(command, working_dir, env_vars);
        let value = CacheValue::CommandOutput {
            stdout: output.stdout,
            stderr: output.stderr,
            exit_code: output.exit_code,
        };

        self.set(&key, value, dependencies)
    }

    pub fn get_builtin_result(
        &self,
        module: &str,
        function: &str,
        args: &[TypedValue],
    ) -> CacheResult<Option<TypedValue>> {
        let key = CacheKeyGenerator::generate_builtin_key(module, function, args);

        match self.get(&key)? {
            Some(CacheValue::BuiltinResult(result)) => Ok(Some(result)),
            Some(_) => Err(CacheError::invalid_key(format!(
                "Expected builtin result for key: {key}"
            ))),
            None => Ok(None),
        }
    }

    pub fn set_builtin_result(
        &self,
        module: &str,
        function: &str,
        args: &[TypedValue],
        result: TypedValue,
        dependencies: Vec<CacheDependency>,
    ) -> CacheResult<()> {
        let key = CacheKeyGenerator::generate_builtin_key(module, function, args);
        let value = CacheValue::BuiltinResult(result);

        self.set(&key, value, dependencies)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use braise_types::TypedValue;
    use tempfile::NamedTempFile;

    fn create_test_manager() -> CacheManager {
        let temp = NamedTempFile::new().unwrap();
        let path = temp.path().to_path_buf();
        let config = CacheConfig::default().with_storage_path(path);
        CacheManager::new(config).unwrap()
    }

    #[test]
    fn test_cache_manager_creation() {
        let manager = create_test_manager();
        assert!(manager.get_config().enabled);
    }

    #[test]
    fn test_basic_cache_operations() {
        let manager = create_test_manager();

        // Test cache miss (use a valid cache key format)
        let result = manager.get("builtin:test:func:abcd1234").unwrap();
        assert!(result.is_none());

        // Test cache set and hit
        let value = CacheValue::BuiltinResult(TypedValue::string("test_value"));
        let key = "builtin:test:func:abcd1234";
        manager.set(key, value.clone(), vec![]).unwrap();

        let cached_result = manager.get(key).unwrap();
        assert!(cached_result.is_some());

        if let Some(CacheValue::BuiltinResult(typed_value)) = cached_result {
            if let braise_types::ValueData::String(s) = &typed_value.value {
                assert_eq!(s, "test_value");
            } else {
                panic!("Expected string value data");
            }
        } else {
            panic!("Expected builtin result value");
        }
    }

    #[test]
    fn test_cache_invalidation() {
        let manager = create_test_manager();

        // Set a value
        let key = "builtin:test:func:abcd1234";
        let value = CacheValue::BuiltinResult(TypedValue::string("test_value"));
        manager.set(key, value, vec![]).unwrap();

        // Verify it's cached
        assert!(manager.get(key).unwrap().is_some());

        // Invalidate
        let was_removed = manager.invalidate(key).unwrap();
        assert!(was_removed);

        // Verify it's gone
        assert!(manager.get(key).unwrap().is_none());
    }

    #[test]
    fn test_cache_stats() {
        let manager = create_test_manager();

        // Initial stats
        let stats = manager.get_stats();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.total_requests, 0);

        // Generate a miss
        manager.get("builtin:test:func:nonexistent").unwrap();
        let stats = manager.get_stats();
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.total_requests, 1);

        // Set a value and generate a hit
        let value = CacheValue::BuiltinResult(TypedValue::string("test"));
        let key = "builtin:test:func:abcd1234";
        manager.set(key, value, vec![]).unwrap();
        manager.get(key).unwrap();

        let stats = manager.get_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.hit_rate(), 0.5);
    }

    #[test]
    fn test_clear_cache() {
        let manager = create_test_manager();

        // Set multiple values
        let value1 = CacheValue::BuiltinResult(TypedValue::string("value1"));
        let value2 = CacheValue::BuiltinResult(TypedValue::string("value2"));

        let key1 = "builtin:test:func1:abcd1234";
        let key2 = "builtin:test:func2:abcd1234";
        manager.set(key1, value1, vec![]).unwrap();
        manager.set(key2, value2, vec![]).unwrap();

        // Verify they're cached
        assert!(manager.get(key1).unwrap().is_some());
        assert!(manager.get(key2).unwrap().is_some());

        // Clear cache
        manager.clear().unwrap();

        // Verify they're gone
        assert!(manager.get(key1).unwrap().is_none());
        assert!(manager.get(key2).unwrap().is_none());
    }

    #[test]
    fn test_convenience_methods() {
        let manager = create_test_manager();

        // Test builtin result caching
        let args = vec![TypedValue::string("test")];
        let result = TypedValue::number(42.0);

        // Cache miss
        let cached = manager
            .get_builtin_result("test_module", "test_func", &args)
            .unwrap();
        assert!(cached.is_none());

        // Set and get
        manager
            .set_builtin_result("test_module", "test_func", &args, result.clone(), vec![])
            .unwrap();
        let cached = manager
            .get_builtin_result("test_module", "test_func", &args)
            .unwrap();
        assert_eq!(cached, Some(result));
    }
}

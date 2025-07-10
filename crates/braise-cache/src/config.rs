use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use bincode::{Decode, Encode};

#[derive(Debug, Clone, Encode, Decode)]
pub struct CacheConfig {
    pub enabled: bool,
    pub storage_path: PathBuf,
    pub max_size: u64,
    pub default_ttl: Option<Duration>,
    pub recipe_cache: bool,
    pub command_cache: bool,
    pub builtin_cache: bool,
    pub expression_cache: bool,
    pub ttl_overrides: HashMap<String, Duration>,
    pub ignore_patterns: Vec<String>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            storage_path: default_cache_dir(),
            max_size: 1024 * 1024 * 100, // 100MB
            default_ttl: None,
            recipe_cache: true,
            command_cache: true,
            builtin_cache: true,
            expression_cache: false,
            ttl_overrides: default_ttl_overrides(),
            ignore_patterns: Vec::new(),
        }
    }
}

impl CacheConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_storage_path(mut self, path: PathBuf) -> Self {
        self.storage_path = path;
        self
    }

    pub fn with_max_size(mut self, size: u64) -> Self {
        self.max_size = size;
        self
    }

    pub fn with_default_ttl(mut self, ttl: Duration) -> Self {
        self.default_ttl = Some(ttl);
        self
    }

    pub fn disable_recipe_cache(mut self) -> Self {
        self.recipe_cache = false;
        self
    }

    pub fn disable_command_cache(mut self) -> Self {
        self.command_cache = false;
        self
    }

    pub fn disable_builtin_cache(mut self) -> Self {
        self.builtin_cache = false;
        self
    }

    pub fn enable_expression_cache(mut self) -> Self {
        self.expression_cache = true;
        self
    }

    pub fn add_ttl_override(mut self, pattern: String, ttl: Duration) -> Self {
        self.ttl_overrides.insert(pattern, ttl);
        self
    }

    pub fn add_ignore_pattern(mut self, pattern: String) -> Self {
        self.ignore_patterns.push(pattern);
        self
    }

    pub fn get_ttl_for_key(&self, key: &str) -> Option<Duration> {
        for (pattern, ttl) in &self.ttl_overrides {
            if key.starts_with(pattern) {
                return Some(*ttl);
            }
        }
        self.default_ttl
    }

    pub fn should_cache_level(&self, level: &crate::CacheLevel) -> bool {
        match level {
            crate::CacheLevel::Recipe => self.recipe_cache,
            crate::CacheLevel::Command => self.command_cache,
            crate::CacheLevel::Builtin => self.builtin_cache,
            crate::CacheLevel::Expression => self.expression_cache,
        }
    }

    pub fn should_ignore_key(&self, key: &str) -> bool {
        for pattern in &self.ignore_patterns {
            if key.contains(pattern) {
                return true;
            }
        }
        false
    }
}

fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("braise")
        .join("cache")
}

fn default_ttl_overrides() -> HashMap<String, Duration> {
    let mut overrides = HashMap::new();

    // Git operations cache for 5 minutes
    overrides.insert("builtin:git:".to_string(), Duration::from_secs(300));

    // HTTP operations cache for 1 hour
    overrides.insert("builtin:http:".to_string(), Duration::from_secs(3600));

    // CPU/OS info cache for 1 day (rarely changes)
    overrides.insert("builtin:cpu:".to_string(), Duration::from_secs(86400));
    overrides.insert("builtin:os:".to_string(), Duration::from_secs(86400));

    overrides
}

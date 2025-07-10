use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use bincode::{Decode, Encode};
use braise_errors::{CacheError, Result};
use braise_types::TypedValue;

pub mod config;
pub mod keys;
pub mod manager;
pub mod storage;

pub use config::CacheConfig;
pub use keys::CacheKeyGenerator;
pub use manager::CacheManager;
pub use storage::{CacheStorage, RedbStorage};

#[derive(Debug, Clone, Encode, Decode)]
pub struct CacheEntry {
    pub key: String,
    pub value: CacheValue,
    pub created_at: SystemTime,
    pub ttl: Option<Duration>,
    pub dependencies: Vec<CacheDependency>,
}

impl CacheEntry {
    pub fn new(key: String, value: CacheValue) -> Self {
        Self {
            key,
            value,
            created_at: SystemTime::now(),
            ttl: None,
            dependencies: Vec::new(),
        }
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    pub fn with_dependencies(mut self, dependencies: Vec<CacheDependency>) -> Self {
        self.dependencies = dependencies;
        self
    }

    pub fn is_expired(&self) -> bool {
        if let Some(ttl) = self.ttl
            && let Ok(elapsed) = self.created_at.elapsed()
        {
            return elapsed > ttl;
        }
        false
    }
}

#[derive(Debug, Clone, Encode, Decode)]
pub enum CacheValue {
    RecipeResult(Result<(), String>),
    CommandOutput {
        stdout: String,
        stderr: String,
        exit_code: i32,
    },
    BuiltinResult(TypedValue),
    ExpressionResult(TypedValue),
}

impl CacheValue {
    pub fn into_result(self) -> Result<(), String> {
        match self {
            CacheValue::RecipeResult(result) => result,
            _ => Err("Cache value is not a recipe result".to_string()),
        }
    }

    pub fn into_command_output(self) -> Result<CommandOutput, String> {
        match self {
            CacheValue::CommandOutput {
                stdout,
                stderr,
                exit_code,
            } => Ok(CommandOutput {
                stdout,
                stderr,
                exit_code,
            }),
            _ => Err("Cache value is not a command output".to_string()),
        }
    }

    pub fn into_typed_value(self) -> Result<TypedValue, String> {
        match self {
            CacheValue::BuiltinResult(value) | CacheValue::ExpressionResult(value) => Ok(value),
            _ => Err("Cache value is not a typed value".to_string()),
        }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

#[derive(Debug, Clone, Encode, Decode)]
pub struct CacheDependency {
    pub dependency_type: DependencyType,
    pub path: PathBuf,
    pub hash: String,
}

#[derive(Debug, Clone, Encode, Decode)]
pub enum DependencyType {
    File,
    Directory,
    GitCommit,
    Environment,
}

#[derive(Debug, Clone, Encode, Decode)]
pub enum CacheLevel {
    Recipe,
    Command,
    Builtin,
    Expression,
}

pub type CacheResult<T> = Result<T, CacheError>;

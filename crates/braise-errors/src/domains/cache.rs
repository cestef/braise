use miette::Diagnostic;
use thiserror::Error;

/// Cache-specific errors
#[derive(Error, Debug, Clone)]
pub enum CacheError {
    #[error("Storage error: {message}")]
    Storage { message: String },

    #[error("Serialization error: {message}")]
    Serialization { message: String },

    #[error("IO error: {message}")]
    Io { message: String },

    #[error("Cache entry not found: {key}")]
    NotFound { key: String },

    #[error("Cache entry expired: {key}")]
    Expired { key: String },

    #[error("Invalid cache key: {key}")]
    InvalidKey { key: String },

    #[error("Cache configuration error: {message}")]
    Configuration { message: String },

    #[error("Cache size limit exceeded: {current_size} bytes (limit: {max_size} bytes)")]
    SizeLimitExceeded { current_size: u64, max_size: u64 },

    #[error("Cache initialization failed: {message}")]
    InitializationFailed { message: String },
}

impl CacheError {
    pub fn storage(message: impl Into<String>) -> Self {
        Self::Storage {
            message: message.into(),
        }
    }

    pub fn serialization(message: impl Into<String>) -> Self {
        Self::Serialization {
            message: message.into(),
        }
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::Io {
            message: message.into(),
        }
    }

    pub fn not_found(key: impl Into<String>) -> Self {
        Self::NotFound { key: key.into() }
    }

    pub fn expired(key: impl Into<String>) -> Self {
        Self::Expired { key: key.into() }
    }

    pub fn invalid_key(key: impl Into<String>) -> Self {
        Self::InvalidKey { key: key.into() }
    }

    pub fn configuration(message: impl Into<String>) -> Self {
        Self::Configuration {
            message: message.into(),
        }
    }

    pub fn size_limit_exceeded(current_size: u64, max_size: u64) -> Self {
        Self::SizeLimitExceeded {
            current_size,
            max_size,
        }
    }

    pub fn initialization_failed(message: impl Into<String>) -> Self {
        Self::InitializationFailed {
            message: message.into(),
        }
    }

    pub fn severity(&self) -> crate::ErrorSeverity {
        match self {
            CacheError::Storage { .. } => crate::ErrorSeverity::Error,
            CacheError::Serialization { .. } => crate::ErrorSeverity::Error,
            CacheError::Io { .. } => crate::ErrorSeverity::Error,
            CacheError::NotFound { .. } => crate::ErrorSeverity::Info,
            CacheError::Expired { .. } => crate::ErrorSeverity::Info,
            CacheError::InvalidKey { .. } => crate::ErrorSeverity::Error,
            CacheError::Configuration { .. } => crate::ErrorSeverity::Error,
            CacheError::SizeLimitExceeded { .. } => crate::ErrorSeverity::Warning,
            CacheError::InitializationFailed { .. } => crate::ErrorSeverity::Error,
        }
    }
}

impl Diagnostic for CacheError {
    fn severity(&self) -> Option<miette::Severity> {
        Some(self.severity().into())
    }

    fn code<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        let code = match self {
            CacheError::Storage { .. } => "cache::storage",
            CacheError::Serialization { .. } => "cache::serialization",
            CacheError::Io { .. } => "cache::io",
            CacheError::NotFound { .. } => "cache::not_found",
            CacheError::Expired { .. } => "cache::expired",
            CacheError::InvalidKey { .. } => "cache::invalid_key",
            CacheError::Configuration { .. } => "cache::configuration",
            CacheError::SizeLimitExceeded { .. } => "cache::size_limit",
            CacheError::InitializationFailed { .. } => "cache::initialization",
        };
        Some(Box::new(code))
    }

    fn help<'a>(&'a self) -> Option<Box<dyn std::fmt::Display + 'a>> {
        let help = match self {
            CacheError::Storage { .. } => {
                "Check if the cache storage path is accessible and has sufficient permissions"
            }
            CacheError::Serialization { .. } => {
                "This is likely a bug in the cache implementation. Consider clearing the cache"
            }
            CacheError::Io { .. } => "Check file system permissions and available disk space",
            CacheError::NotFound { .. } => {
                "This cache entry may have been removed or never existed"
            }
            CacheError::Expired { .. } => {
                "Cache entries automatically expire based on TTL settings"
            }
            CacheError::InvalidKey { .. } => "Cache keys must be valid UTF-8 strings",
            CacheError::Configuration { .. } => "Check the cache configuration settings",
            CacheError::SizeLimitExceeded { .. } => {
                "Consider increasing the cache size limit or clearing old entries"
            }
            CacheError::InitializationFailed { .. } => {
                "Check if the cache directory exists and is writable"
            }
        };
        Some(Box::new(help))
    }
}

// Note: External crate conversions are implemented in braise-cache crate
// to avoid adding dependencies to braise-errors

pub mod core;
pub mod domains;
pub mod diagnostics;
pub mod context;
pub mod conversion;
pub mod formatting;

// Re-export the main types for convenience
pub use core::*;
pub use domains::*;
pub use diagnostics::*;
pub use context::*;
pub use conversion::*;
pub use formatting::*;

// Unified Result type for the entire Braise ecosystem
pub type Result<T, E = BraiseError> = std::result::Result<T, E>;
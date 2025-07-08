pub mod context;
pub mod conversion;
pub mod core;
pub mod diagnostics;
pub mod domains;
pub mod formatting;

// Re-export the main types for convenience
pub use context::*;
pub use conversion::*;
pub use core::*;
pub use diagnostics::*;
pub use domains::*;
pub use formatting::*;

// Unified Result type for the entire Braise ecosystem
pub type Result<T, E = BraiseError> = std::result::Result<T, E>;

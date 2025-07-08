pub mod ast;
pub mod constants;
pub mod diagnostic;
pub mod error;
pub mod span;
pub mod symbol;

// Re-export types from braise-types (excluding types that conflict with braise-errors)
pub use braise_types::{BraiseType, TypeInferenceEngine, TypedValue, ValueData};

// Re-export the unified error system
pub use error::{BraiseError, ErrorSeverity, Result};

// Re-export other core modules
pub use ast::*;
pub use constants::*;
pub use diagnostic::*;
pub use span::*;
pub use symbol::*;

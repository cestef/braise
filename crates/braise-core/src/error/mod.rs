pub use braise_errors::*;

pub mod cli {
    pub use braise_errors::CliError;
    pub type Result<T> = std::result::Result<T, CliError>;
}

pub mod parser {
    pub use braise_errors::ParserError as ParseError;
    pub type Result<T> = std::result::Result<T, ParseError>;
}

pub mod runtime {
    pub use braise_errors::RuntimeError;
    pub type Result<T, E = RuntimeError> = std::result::Result<T, E>;
}

// Legacy type aliases for backward compatibility
pub type Result<T, E = BraiseError> = std::result::Result<T, E>;

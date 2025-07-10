pub use braise_errors::*;

pub mod cli {
    pub use braise_errors::CliError;
    pub type Result<T> = std::result::Result<T, Box<CliError>>;
}

pub mod parser {
    pub use braise_errors::ParserError as ParseError;
    pub type Result<T> = std::result::Result<T, Box<ParseError>>;
}

pub mod runtime {
    pub use braise_errors::RuntimeError;
    pub type Result<T, E = Box<RuntimeError>> = std::result::Result<T, E>;
}

// Legacy type aliases for backward compatibility
pub type Result<T, E = Box<BraiseError>> = std::result::Result<T, E>;

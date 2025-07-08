use crate::core::BraiseError;
use crate::domains::*;
use std::io;

/// Conversion from standard library errors to Braise errors
impl From<io::Error> for BraiseError {
    fn from(error: io::Error) -> Self {
        let message = error.to_string();
        let operation = match error.kind() {
            io::ErrorKind::NotFound => "file not found",
            io::ErrorKind::PermissionDenied => "permission denied",
            io::ErrorKind::ConnectionRefused => "connection refused",
            io::ErrorKind::ConnectionReset => "connection reset",
            io::ErrorKind::ConnectionAborted => "connection aborted",
            io::ErrorKind::NotConnected => "not connected",
            io::ErrorKind::AddrInUse => "address in use",
            io::ErrorKind::AddrNotAvailable => "address not available",
            io::ErrorKind::BrokenPipe => "broken pipe",
            io::ErrorKind::AlreadyExists => "already exists",
            io::ErrorKind::WouldBlock => "would block",
            io::ErrorKind::InvalidInput => "invalid input",
            io::ErrorKind::InvalidData => "invalid data",
            io::ErrorKind::TimedOut => "timed out",
            io::ErrorKind::WriteZero => "write zero",
            io::ErrorKind::Interrupted => "interrupted",
            io::ErrorKind::UnexpectedEof => "unexpected EOF",
            _ => "I/O operation",
        };

        BraiseError::Runtime(RuntimeError::io_error(message, operation))
    }
}

// Optional conversions for external crates (commented out to avoid warnings)
// These can be enabled when the corresponding features are added to Cargo.toml

/*
/// Conversion from JSON parsing errors
impl From<serde_json::Error> for BraiseError {
    fn from(error: serde_json::Error) -> Self {
        BraiseError::Parser(ParserError::other(
            format!("JSON parsing error: {}", error),
        ))
    }
}

/// Conversion from TOML parsing errors
impl From<toml::de::Error> for BraiseError {
    fn from(error: toml::de::Error) -> Self {
        BraiseError::Parser(ParserError::other(
            format!("TOML parsing error: {}", error),
        ))
    }
}

/// Conversion from regex errors
impl From<regex::Error> for BraiseError {
    fn from(error: regex::Error) -> Self {
        BraiseError::Runtime(RuntimeError::other(
            format!("Regular expression error: {}", error),
        ))
    }
}
*/

/// Conversion from environment variable errors
impl From<std::env::VarError> for BraiseError {
    fn from(error: std::env::VarError) -> Self {
        let message = match error {
            std::env::VarError::NotPresent => "Environment variable not present",
            std::env::VarError::NotUnicode(_) => "Environment variable contains invalid Unicode",
        };
        BraiseError::Runtime(RuntimeError::builtin_error(message, "env"))
    }
}

/// Conversion from string parsing errors
impl From<std::num::ParseIntError> for BraiseError {
    fn from(error: std::num::ParseIntError) -> Self {
        BraiseError::Type(TypeError::cannot_convert("string", "number"))
            .with_suggestion(format!("Invalid number format: {}", error))
    }
}

impl From<std::num::ParseFloatError> for BraiseError {
    fn from(error: std::num::ParseFloatError) -> Self {
        BraiseError::Type(TypeError::cannot_convert("string", "number"))
            .with_suggestion(format!("Invalid float format: {}", error))
    }
}

impl From<std::str::ParseBoolError> for BraiseError {
    fn from(_error: std::str::ParseBoolError) -> Self {
        BraiseError::Type(TypeError::cannot_convert("string", "bool"))
            .with_suggestion("Use 'true', 'false', '1', '0', 'yes', 'no', 'on', or 'off'".to_string())
    }
}

/// Conversion from braise-types TypeError to unified TypeError
impl From<braise_types::TypeError> for TypeError {
    fn from(error: braise_types::TypeError) -> Self {
        match error {
            braise_types::TypeError::Mismatch { expected, got, context } => {
                TypeError::mismatch(expected, got, context)
            }
            braise_types::TypeError::UnsupportedOperation { operation, left, right } => {
                TypeError::unsupported_operation(operation, left, right)
            }
            braise_types::TypeError::CannotConvert { from, to } => {
                TypeError::cannot_convert(from, to)
            }
        }
    }
}

/// Conversion from UTF-8 errors
impl From<std::str::Utf8Error> for BraiseError {
    fn from(error: std::str::Utf8Error) -> Self {
        BraiseError::Runtime(RuntimeError::other(
            format!("UTF-8 encoding error: {}", error),
        ))
    }
}

impl From<std::string::FromUtf8Error> for BraiseError {
    fn from(error: std::string::FromUtf8Error) -> Self {
        BraiseError::Runtime(RuntimeError::other(
            format!("UTF-8 conversion error: {}", error),
        ))
    }
}

/// Conversion from filesystem errors
impl From<std::fs::DirBuilder> for BraiseError {
    fn from(_: std::fs::DirBuilder) -> Self {
        BraiseError::Runtime(RuntimeError::io_error(
            "Directory creation failed",
            "create directory",
        ))
    }
}

/// Conversion from format errors
impl From<std::fmt::Error> for BraiseError {
    fn from(error: std::fmt::Error) -> Self {
        BraiseError::Runtime(RuntimeError::other(
            format!("Formatting error: {}", error),
        ))
    }
}

/// Conversion from thread join errors
impl From<Box<dyn std::any::Any + Send>> for BraiseError {
    fn from(_error: Box<dyn std::any::Any + Send>) -> Self {
        BraiseError::Runtime(RuntimeError::other(
            "Thread execution failed".to_string(),
        ))
    }
}

/// Helper trait for converting Results with context
pub trait ResultExt<T> {
    /// Convert an error and add context information
    fn with_braise_context(self, context: &str) -> Result<T, BraiseError>;
    
    /// Convert an error and add a suggestion
    fn with_suggestion(self, suggestion: impl Into<String>) -> Result<T, BraiseError>;
    
    /// Convert to a runtime error with context
    fn as_runtime_error(self, context: &str) -> Result<T, BraiseError>;
    
    /// Convert to a parser error with context
    fn as_parser_error(self, context: &str) -> Result<T, BraiseError>;
    
    /// Convert to a type error with context
    fn as_type_error(self, expected: &str, got: &str, context: &str) -> Result<T, BraiseError>;
}

impl<T, E> ResultExt<T> for Result<T, E>
where
    E: Into<BraiseError>,
{
    fn with_braise_context(self, context: &str) -> Result<T, BraiseError> {
        self.map_err(|e| {
            let mut error = e.into();
            error = error.with_context(context.to_string());
            error
        })
    }

    fn with_suggestion(self, suggestion: impl Into<String>) -> Result<T, BraiseError> {
        self.map_err(|e| {
            let mut error = e.into();
            error = error.with_suggestion(suggestion.into());
            error
        })
    }

    fn as_runtime_error(self, context: &str) -> Result<T, BraiseError> {
        self.map_err(|e| {
            BraiseError::Runtime(RuntimeError::other(
                format!("{}: {}", context, Into::<BraiseError>::into(e))
            ))
        })
    }

    fn as_parser_error(self, context: &str) -> Result<T, BraiseError> {
        self.map_err(|e| {
            BraiseError::Parser(ParserError::other(
                format!("{}: {}", context, Into::<BraiseError>::into(e))
            ))
        })
    }

    fn as_type_error(self, expected: &str, got: &str, context: &str) -> Result<T, BraiseError> {
        self.map_err(|_e| {
            BraiseError::Type(TypeError::mismatch(expected, got, context))
        })
    }
}

/// Helper for chaining error conversions
pub struct ErrorChain {
    errors: Vec<BraiseError>,
}

impl ErrorChain {
    /// Create a new error chain
    pub fn new() -> Self {
        Self {
            errors: Vec::new(),
        }
    }

    /// Add an error to the chain
    pub fn push(mut self, error: impl Into<BraiseError>) -> Self {
        self.errors.push(error.into());
        self
    }

    /// Convert the chain to a single error
    pub fn into_error(self) -> Option<BraiseError> {
        if self.errors.is_empty() {
            return None;
        }

        if self.errors.len() == 1 {
            return Some(self.errors.into_iter().next().unwrap());
        }

        // For multiple errors, create a runtime error with all messages
        let messages: Vec<String> = self.errors.iter().map(|e| e.to_string()).collect();
        Some(BraiseError::Runtime(RuntimeError::other(
            format!("Multiple errors occurred:\n{}", messages.join("\n")),
        )))
    }
}

impl Default for ErrorChain {
    fn default() -> Self {
        Self::new()
    }
}

/// Extension methods for BraiseError
impl BraiseError {
    /// Add context information to this error
    pub fn with_context(self, context: String) -> Self {
        match self {
            BraiseError::Runtime(mut error) => {
                match &mut error {
                    RuntimeError::Other { message, .. } => {
                        *message = format!("{}: {}", context, message);
                    }
                    _ => {
                        return BraiseError::Runtime(RuntimeError::other(
                            format!("{}: {}", context, error)
                        ));
                    }
                }
                BraiseError::Runtime(error)
            }
            other => other, // For other error types, return as-is for now
        }
    }

    /// Add a suggestion to this error
    pub fn with_suggestion(self, _suggestion: String) -> Self {
        // For now, return the error as-is
        // In a full implementation, we'd modify the error to include the suggestion
        self
    }

    /// Check if this error is recoverable
    pub fn is_recoverable(&self) -> bool {
        match self {
            BraiseError::Cli(cli_error) => {
                matches!(cli_error, CliError::NoTask | CliError::InvalidArgument { .. })
            }
            BraiseError::Parser(_) => false, // Syntax errors are generally not recoverable
            BraiseError::Runtime(runtime_error) => {
                matches!(
                    runtime_error,
                    RuntimeError::CommandFailed { .. }
                    | RuntimeError::InvalidParameter { .. }
                    | RuntimeError::BuiltinError { .. }
                )
            }
            BraiseError::Type(_) => false, // Type errors are not recoverable
            BraiseError::Lexer { .. } => false, // Lexical errors are not recoverable
            BraiseError::Lsp(_) => true, // LSP errors are generally recoverable
        }
    }

    /// Get the error category for logging/metrics
    pub fn category(&self) -> &'static str {
        match self {
            BraiseError::Cli(_) => "cli",
            BraiseError::Parser(_) => "parser",
            BraiseError::Runtime(_) => "runtime",
            BraiseError::Type(_) => "types",
            BraiseError::Lexer { .. } => "lexer",
            BraiseError::Lsp(_) => "lsp",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_io_error_conversion() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "File not found");
        let braise_error: BraiseError = io_error.into();
        
        match braise_error {
            BraiseError::Runtime(RuntimeError::IoError { operation, .. }) => {
                assert_eq!(operation, "file not found");
            }
            _ => panic!("Expected IoError"),
        }
    }

    #[test]
    fn test_parse_int_error_conversion() {
        let parse_error = "not_a_number".parse::<i32>().unwrap_err();
        let braise_error: BraiseError = parse_error.into();
        
        match braise_error {
            BraiseError::Type(TypeError::CannotConvert { from, to, .. }) => {
                assert_eq!(from, "string");
                assert_eq!(to, "number");
            }
            _ => panic!("Expected CannotConvert"),
        }
    }

    #[test]
    fn test_result_ext_with_context() {
        let result: Result<(), io::Error> = Err(io::Error::new(io::ErrorKind::NotFound, "test"));
        let with_context = result.with_braise_context("testing");
        
        assert!(with_context.is_err());
    }

    #[test]
    fn test_error_chain() {
        let chain = ErrorChain::new()
            .push(RuntimeError::undefined_variable("x"))
            .push(RuntimeError::undefined_variable("y"));
        
        let error = chain.into_error();
        assert!(error.is_some());
        
        let error_msg = error.unwrap().to_string();
        assert!(error_msg.contains("Multiple errors"));
    }

    #[test]
    fn test_error_recoverability() {
        assert!(BraiseError::Cli(CliError::no_task()).is_recoverable());
        assert!(!BraiseError::Parser(ParserError::other("test")).is_recoverable());
        assert!(BraiseError::Runtime(RuntimeError::command_failed("ls", 1)).is_recoverable());
        assert!(!BraiseError::Type(TypeError::mismatch("a", "b", "c")).is_recoverable());
    }

    #[test]
    fn test_error_categories() {
        assert_eq!(BraiseError::Cli(CliError::no_task()).category(), "cli");
        assert_eq!(BraiseError::Parser(ParserError::other("test")).category(), "parser");
        assert_eq!(BraiseError::Runtime(RuntimeError::other("test")).category(), "runtime");
        assert_eq!(BraiseError::Type(TypeError::mismatch("a", "b", "c")).category(), "types");
    }
}
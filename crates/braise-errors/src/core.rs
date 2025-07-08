use crate::domains::*;
use miette::{Severity, SourceSpan};
use thiserror::Error;

/// Core error type that aggregates all domain-specific errors in the Braise ecosystem
#[derive(Error, Debug)]
pub enum BraiseError {
    /// Lexical analysis errors
    #[error("Lexer error")]
    Lexer {
        /// Error message
        code: String,
        /// Location of the error
        span: SourceSpan,
        /// Error details
        message: String,
    },

    /// Parsing errors
    #[error(transparent)]
    Parser(#[from] ParserError),

    /// Runtime execution errors
    #[error(transparent)]
    Runtime(#[from] RuntimeError),

    /// Type system errors
    #[error(transparent)]
    Type(#[from] TypeError),

    /// Command-line interface errors
    #[error(transparent)]
    Cli(#[from] CliError),

    /// Language Server Protocol errors
    #[error(transparent)]
    Lsp(#[from] LspError),
}

impl BraiseError {
    /// Create a lexer error with source context
    pub fn lexer(message: impl Into<String>, code: impl Into<String>, span: SourceSpan) -> Self {
        Self::Lexer {
            message: message.into(),
            code: code.into(),
            span,
        }
    }

    /// Check if this error is a specific domain error
    pub fn is_parser_error(&self) -> bool {
        matches!(self, BraiseError::Parser(_))
    }

    /// Check if this error is a runtime error
    pub fn is_runtime_error(&self) -> bool {
        matches!(self, BraiseError::Runtime(_))
    }

    /// Check if this error is a type error
    pub fn is_type_error(&self) -> bool {
        matches!(self, BraiseError::Type(_))
    }

    /// Check if this error is a CLI error
    pub fn is_cli_error(&self) -> bool {
        matches!(self, BraiseError::Cli(_))
    }

    /// Check if this error is an LSP error
    pub fn is_lsp_error(&self) -> bool {
        matches!(self, BraiseError::Lsp(_))
    }

    /// Get the error domain as a string
    pub fn domain(&self) -> &'static str {
        match self {
            BraiseError::Lexer { .. } => "lexer",
            BraiseError::Parser(_) => "parser", 
            BraiseError::Runtime(_) => "runtime",
            BraiseError::Type(_) => "type",
            BraiseError::Cli(_) => "cli",
            BraiseError::Lsp(_) => "lsp",
        }
    }

    /// Get the severity level of this error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            BraiseError::Lexer { .. } => ErrorSeverity::Error,
            BraiseError::Parser(_) => ErrorSeverity::Error,
            BraiseError::Runtime(e) => e.severity(),
            BraiseError::Type(_) => ErrorSeverity::Error,
            BraiseError::Cli(_) => ErrorSeverity::Error,
            BraiseError::Lsp(_) => ErrorSeverity::Warning,
        }
    }
}

/// Error severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorSeverity {
    /// Informational message
    Info,
    /// Warning that doesn't prevent execution
    Warning,
    /// Error that prevents successful execution
    Error,
    /// Critical error that indicates a serious problem
    Critical,
}

impl std::fmt::Display for ErrorSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ErrorSeverity::Info => write!(f, "info"),
            ErrorSeverity::Warning => write!(f, "warning"),
            ErrorSeverity::Error => write!(f, "error"),
            ErrorSeverity::Critical => write!(f, "critical"),
        }
    }
}

impl From<ErrorSeverity> for Severity {
    fn from(severity: ErrorSeverity) -> Self {
        match severity {
            ErrorSeverity::Info => Severity::Advice,
            ErrorSeverity::Warning => Severity::Warning,
            ErrorSeverity::Error => Severity::Error,
            ErrorSeverity::Critical => Severity::Error,
        }
    }
}

impl miette::Diagnostic for BraiseError {
    fn severity(&self) -> Option<miette::Severity> {
        Some(self.severity().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_domain_identification() {
        let lexer_error = BraiseError::lexer("test", "source", (0, 1).into());
        assert_eq!(lexer_error.domain(), "lexer");
        assert!(matches!(lexer_error.severity(), ErrorSeverity::Error));
    }

    #[test]
    fn test_error_type_checking() {
        let parser_error = BraiseError::Parser(ParserError::other("test"));
        assert!(parser_error.is_parser_error());
        assert!(!parser_error.is_runtime_error());
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(ErrorSeverity::Info.to_string(), "info");
        assert_eq!(ErrorSeverity::Warning.to_string(), "warning");
        assert_eq!(ErrorSeverity::Error.to_string(), "error");
        assert_eq!(ErrorSeverity::Critical.to_string(), "critical");
    }
}
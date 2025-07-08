use crate::core::ErrorSeverity;
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

/// Lexical analysis errors
#[derive(Debug, Error, Diagnostic)]
pub enum LexerError {
    /// Unrecognized token in source code
    #[error("Unrecognized token")]
    #[diagnostic(code(braise::lexer::unrecognized_token))]
    UnrecognizedToken {
        /// Source code being lexed
        #[source_code]
        code: String,
        /// Location of the unrecognized token
        #[label("unexpected token here")]
        span: SourceSpan,
    },

    /// Invalid string literal
    #[error("Invalid string literal")]
    #[diagnostic(
        code(braise::lexer::invalid_string),
        help("Make sure string literals are properly quoted and escaped")
    )]
    InvalidString {
        /// Source code being lexed
        #[source_code]
        code: String,
        /// Location of the invalid string
        #[label("invalid string literal")]
        span: SourceSpan,
        /// Specific issue with the string
        reason: String,
    },

    /// Invalid number literal
    #[error("Invalid number literal")]
    #[diagnostic(
        code(braise::lexer::invalid_number),
        help("Use valid number formats like 42, 3.14, or 1e10")
    )]
    InvalidNumber {
        /// Source code being lexed
        #[source_code]
        code: String,
        /// Location of the invalid number
        #[label("invalid number literal")]
        span: SourceSpan,
        /// The invalid number text
        text: String,
    },

    /// Unterminated string literal
    #[error("Unterminated string literal")]
    #[diagnostic(
        code(braise::lexer::unterminated_string),
        help("Add a closing quote to terminate the string")
    )]
    UnterminatedString {
        /// Source code being lexed
        #[source_code]
        code: String,
        /// Location where the string started
        #[label("string starts here")]
        span: SourceSpan,
    },

    /// Invalid escape sequence in string
    #[error("Invalid escape sequence: \\{escape}")]
    #[diagnostic(
        code(braise::lexer::invalid_escape),
        help("Valid escape sequences: \\n, \\t, \\r, \\\\, \\\", \\'")
    )]
    InvalidEscape {
        /// Source code being lexed
        #[source_code]
        code: String,
        /// Location of the invalid escape
        #[label("invalid escape sequence")]
        span: SourceSpan,
        /// The invalid escape character
        escape: char,
    },

    /// Invalid unicode escape sequence
    #[error("Invalid unicode escape sequence")]
    #[diagnostic(
        code(braise::lexer::invalid_unicode_escape),
        help("Use \\u{{XXXX}} format for unicode escapes")
    )]
    InvalidUnicodeEscape {
        /// Source code being lexed
        #[source_code]
        code: String,
        /// Location of the invalid unicode escape
        #[label("invalid unicode escape")]
        span: SourceSpan,
        /// The invalid unicode sequence
        sequence: String,
    },
}

impl LexerError {
    /// Create an unrecognized token error
    pub fn unrecognized_token(code: impl Into<String>, span: SourceSpan) -> Self {
        Self::UnrecognizedToken {
            code: code.into(),
            span,
        }
    }

    /// Create an invalid string error
    pub fn invalid_string(
        code: impl Into<String>,
        span: SourceSpan,
        reason: impl Into<String>,
    ) -> Self {
        Self::InvalidString {
            code: code.into(),
            span,
            reason: reason.into(),
        }
    }

    /// Create an invalid number error
    pub fn invalid_number(
        code: impl Into<String>,
        span: SourceSpan,
        text: impl Into<String>,
    ) -> Self {
        Self::InvalidNumber {
            code: code.into(),
            span,
            text: text.into(),
        }
    }

    /// Create an unterminated string error
    pub fn unterminated_string(code: impl Into<String>, span: SourceSpan) -> Self {
        Self::UnterminatedString {
            code: code.into(),
            span,
        }
    }

    /// Create an invalid escape sequence error
    pub fn invalid_escape(code: impl Into<String>, span: SourceSpan, escape: char) -> Self {
        Self::InvalidEscape {
            code: code.into(),
            span,
            escape,
        }
    }

    /// Create an invalid unicode escape error
    pub fn invalid_unicode_escape(
        code: impl Into<String>,
        span: SourceSpan,
        sequence: impl Into<String>,
    ) -> Self {
        Self::InvalidUnicodeEscape {
            code: code.into(),
            span,
            sequence: sequence.into(),
        }
    }

    /// Get the severity of this lexer error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            LexerError::UnrecognizedToken { .. } => ErrorSeverity::Error,
            LexerError::InvalidString { .. } => ErrorSeverity::Error,
            LexerError::InvalidNumber { .. } => ErrorSeverity::Error,
            LexerError::UnterminatedString { .. } => ErrorSeverity::Error,
            LexerError::InvalidEscape { .. } => ErrorSeverity::Error,
            LexerError::InvalidUnicodeEscape { .. } => ErrorSeverity::Error,
        }
    }

    /// Get the source span for this error
    pub fn span(&self) -> &SourceSpan {
        match self {
            LexerError::UnrecognizedToken { span, .. } => span,
            LexerError::InvalidString { span, .. } => span,
            LexerError::InvalidNumber { span, .. } => span,
            LexerError::UnterminatedString { span, .. } => span,
            LexerError::InvalidEscape { span, .. } => span,
            LexerError::InvalidUnicodeEscape { span, .. } => span,
        }
    }

    /// Get the source code for this error
    pub fn source_code(&self) -> &str {
        match self {
            LexerError::UnrecognizedToken { code, .. } => code,
            LexerError::InvalidString { code, .. } => code,
            LexerError::InvalidNumber { code, .. } => code,
            LexerError::UnterminatedString { code, .. } => code,
            LexerError::InvalidEscape { code, .. } => code,
            LexerError::InvalidUnicodeEscape { code, .. } => code,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_error_creation() {
        let error = LexerError::unrecognized_token("invalid token @", (13, 1).into());

        match error {
            LexerError::UnrecognizedToken { code, span } => {
                assert_eq!(code, "invalid token @");
                assert_eq!(span, (13, 1).into());
            }
            _ => panic!("Expected UnrecognizedToken"),
        }
    }

    #[test]
    fn test_lexer_error_severity() {
        let error = LexerError::invalid_number("42.invalid", (0, 10).into(), "42.invalid");
        assert_eq!(error.severity(), ErrorSeverity::Error);
    }

    #[test]
    fn test_lexer_error_span_access() {
        let span = (5, 3).into();
        let error = LexerError::unterminated_string("\"hello", span);
        assert_eq!(error.span(), &span);
        assert_eq!(error.source_code(), "\"hello");
    }

    #[test]
    fn test_invalid_escape_error() {
        let error = LexerError::invalid_escape("\"\\q\"", (1, 2).into(), 'q');

        match error {
            LexerError::InvalidEscape { escape, .. } => {
                assert_eq!(escape, 'q');
            }
            _ => panic!("Expected InvalidEscape"),
        }
    }
}

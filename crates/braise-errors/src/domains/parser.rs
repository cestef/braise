use crate::core::ErrorSeverity;
use miette::{Diagnostic, SourceSpan};
use owo_colors::OwoColorize;
use thiserror::Error;

/// Parsing and syntax analysis errors
#[derive(Debug, Error, Diagnostic)]
pub enum ParserError {
    /// Unexpected token found during parsing
    #[error("Unexpected token: expected {}, found {}", 
        expected.bold().green(),
        found.bold().red()
    )]
    #[diagnostic(
        code(braise::parser::unexpected_token),
        help("Check the syntax around this location")
    )]
    UnexpectedToken {
        /// What token was expected
        expected: String,
        /// What token was actually found
        found: String,
        /// Source code being parsed
        #[source_code]
        code: String,
        /// Location of the unexpected token
        #[label("unexpected token here")]
        span: SourceSpan,
    },

    /// Invalid expression encountered
    #[error("Invalid expression: {reason}")]
    #[diagnostic(code(braise::parser::invalid_expression))]
    InvalidExpression {
        /// Reason why the expression is invalid
        reason: String,
        /// Source code being parsed
        #[source_code]
        code: String,
        /// Location of the invalid expression
        #[label("invalid expression")]
        span: SourceSpan,
    },

    /// Non-exhaustive pattern matching
    #[error("Non-exhaustive match: {}", 
        missing.as_deref()
            .map(|s| format!("missing: '{s}'"))
            .unwrap_or_else(|| "consider adding a wildcard pattern '_'".to_string())
    )]
    #[diagnostic(
        code(braise::parser::non_exhaustive_match),
        help("Add patterns to cover all possible values, or use a wildcard pattern '_'")
    )]
    NonExhaustiveMatch {
        /// Source code being parsed
        #[source_code]
        code: String,
        /// Location of the match expression
        #[label("non-exhaustive match")]
        span: SourceSpan,
        /// Description of what patterns are missing
        missing: Option<String>,
    },

    /// Invalid function signature
    #[error("Invalid function signature: {reason}")]
    #[diagnostic(
        code(braise::parser::invalid_function_signature),
        help("Check function parameter types and return type")
    )]
    InvalidFunctionSignature {
        /// Reason for the invalid signature
        reason: String,
        /// Source code being parsed
        #[source_code]
        code: String,
        /// Location of the function signature
        #[label("invalid signature")]
        span: SourceSpan,
    },

    /// Invalid type annotation
    #[error("Invalid type annotation: {reason}")]
    #[diagnostic(
        code(braise::parser::invalid_type_annotation),
        help("Use valid type syntax like 'string', 'number', '[string]', or 'string | number'")
    )]
    InvalidTypeAnnotation {
        /// Reason for the invalid type
        reason: String,
        /// Source code being parsed
        #[source_code]
        code: String,
        /// Location of the type annotation
        #[label("invalid type")]
        span: SourceSpan,
    },

    /// Duplicate parameter names
    #[error("Duplicate parameter name: '{name}'")]
    #[diagnostic(
        code(braise::parser::duplicate_parameter),
        help("Parameter names must be unique within a recipe")
    )]
    DuplicateParameter {
        /// The duplicate parameter name
        name: String,
        /// Source code being parsed
        #[source_code]
        code: String,
        /// Location of the duplicate parameter
        #[label("duplicate parameter")]
        span: SourceSpan,
        /// Location of the first occurrence
        #[label("first defined here")]
        first_span: SourceSpan,
    },

    /// Invalid recipe dependency
    #[error("Invalid recipe dependency: {reason}")]
    #[diagnostic(
        code(braise::parser::invalid_dependency),
        help("Recipe dependencies must be valid recipe names")
    )]
    InvalidDependency {
        /// Reason for the invalid dependency
        reason: String,
        /// Source code being parsed
        #[source_code]
        code: String,
        /// Location of the invalid dependency
        #[label("invalid dependency")]
        span: SourceSpan,
    },

    /// Generic parsing error
    #[error("Parse error: {message}")]
    #[diagnostic(code(braise::parser::other))]
    Other {
        /// Error message
        message: String,
        /// Optional source code context
        #[source_code]
        code: Option<String>,
        /// Optional error location
        #[label("here")]
        span: Option<SourceSpan>,
    },
}

impl ParserError {
    /// Create an unexpected token error
    pub fn unexpected_token(
        expected: impl Into<String>,
        found: impl Into<String>,
        code: impl Into<String>,
        span: SourceSpan,
    ) -> Self {
        Self::UnexpectedToken {
            expected: expected.into(),
            found: found.into(),
            code: code.into(),
            span,
        }
    }

    /// Create an invalid expression error
    pub fn invalid_expression(
        reason: impl Into<String>,
        code: impl Into<String>,
        span: SourceSpan,
    ) -> Self {
        Self::InvalidExpression {
            reason: reason.into(),
            code: code.into(),
            span,
        }
    }

    /// Create a non-exhaustive match error
    pub fn non_exhaustive_match(
        code: impl Into<String>,
        span: SourceSpan,
        missing: Option<String>,
    ) -> Self {
        Self::NonExhaustiveMatch {
            code: code.into(),
            span,
            missing,
        }
    }

    /// Create an invalid function signature error
    pub fn invalid_function_signature(
        reason: impl Into<String>,
        code: impl Into<String>,
        span: SourceSpan,
    ) -> Self {
        Self::InvalidFunctionSignature {
            reason: reason.into(),
            code: code.into(),
            span,
        }
    }

    /// Create an invalid type annotation error
    pub fn invalid_type_annotation(
        reason: impl Into<String>,
        code: impl Into<String>,
        span: SourceSpan,
    ) -> Self {
        Self::InvalidTypeAnnotation {
            reason: reason.into(),
            code: code.into(),
            span,
        }
    }

    /// Create a duplicate parameter error
    pub fn duplicate_parameter(
        name: impl Into<String>,
        code: impl Into<String>,
        span: SourceSpan,
        first_span: SourceSpan,
    ) -> Self {
        Self::DuplicateParameter {
            name: name.into(),
            code: code.into(),
            span,
            first_span,
        }
    }

    /// Create an invalid dependency error
    pub fn invalid_dependency(
        reason: impl Into<String>,
        code: impl Into<String>,
        span: SourceSpan,
    ) -> Self {
        Self::InvalidDependency {
            reason: reason.into(),
            code: code.into(),
            span,
        }
    }

    /// Create a generic parsing error
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other {
            message: message.into(),
            code: None,
            span: None,
        }
    }

    /// Create a generic parsing error with context
    pub fn other_with_context(
        message: impl Into<String>,
        code: impl Into<String>,
        span: SourceSpan,
    ) -> Self {
        Self::Other {
            message: message.into(),
            code: Some(code.into()),
            span: Some(span),
        }
    }

    /// Get the severity of this parser error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            ParserError::UnexpectedToken { .. } => ErrorSeverity::Error,
            ParserError::InvalidExpression { .. } => ErrorSeverity::Error,
            ParserError::NonExhaustiveMatch { .. } => ErrorSeverity::Warning,
            ParserError::InvalidFunctionSignature { .. } => ErrorSeverity::Error,
            ParserError::InvalidTypeAnnotation { .. } => ErrorSeverity::Error,
            ParserError::DuplicateParameter { .. } => ErrorSeverity::Error,
            ParserError::InvalidDependency { .. } => ErrorSeverity::Error,
            ParserError::Other { .. } => ErrorSeverity::Error,
        }
    }

    /// Get the source span for this error, if available
    pub fn span(&self) -> Option<&SourceSpan> {
        match self {
            ParserError::UnexpectedToken { span, .. } => Some(span),
            ParserError::InvalidExpression { span, .. } => Some(span),
            ParserError::NonExhaustiveMatch { span, .. } => Some(span),
            ParserError::InvalidFunctionSignature { span, .. } => Some(span),
            ParserError::InvalidTypeAnnotation { span, .. } => Some(span),
            ParserError::DuplicateParameter { span, .. } => Some(span),
            ParserError::InvalidDependency { span, .. } => Some(span),
            ParserError::Other { span, .. } => span.as_ref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_error_creation() {
        let error =
            ParserError::unexpected_token("identifier", "number", "42 invalid", (0, 2).into());

        match error {
            ParserError::UnexpectedToken {
                expected, found, ..
            } => {
                assert_eq!(expected, "identifier");
                assert_eq!(found, "number");
            }
            _ => panic!("Expected UnexpectedToken"),
        }
    }

    #[test]
    fn test_parser_error_severity() {
        let error = ParserError::non_exhaustive_match(
            "match x { 1 => }",
            (0, 16).into(),
            Some("2, _".to_string()),
        );
        assert_eq!(error.severity(), ErrorSeverity::Warning);

        let error = ParserError::invalid_expression("missing operator", "x y", (0, 3).into());
        assert_eq!(error.severity(), ErrorSeverity::Error);
    }

    #[test]
    fn test_duplicate_parameter_error() {
        let error = ParserError::duplicate_parameter(
            "name",
            "recipe test(name: string, name: number)",
            (26, 4).into(),
            (12, 4).into(),
        );

        match error {
            ParserError::DuplicateParameter { name, .. } => {
                assert_eq!(name, "name");
            }
            _ => panic!("Expected DuplicateParameter"),
        }
    }

    #[test]
    fn test_other_error_with_context() {
        let error = ParserError::other_with_context("custom error", "source", (0, 6).into());
        assert!(error.span().is_some());

        let error = ParserError::other("simple error");
        assert!(error.span().is_none());
    }
}

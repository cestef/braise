use crate::core::ErrorSeverity;
use miette::{Diagnostic, SourceSpan};
use owo_colors::OwoColorize;
use thiserror::Error;

/// Type system errors in the Braise language
#[derive(Error, Debug, Clone, PartialEq, Diagnostic)]
pub enum TypeError {
    /// Type mismatch between expected and actual types
    #[error("Type mismatch: expected {}, got {} in {}", 
        expected.green(), 
        got.red(), 
        context.yellow()
    )]
    #[diagnostic(
        code(braise::types::mismatch)
    )]
    Mismatch {
        /// Expected type
        expected: String,
        /// Actual type found
        got: String,
        /// Context where the mismatch occurred
        context: String,
        /// Optional suggestion for fixing the error
        suggestion: Option<String>,
        /// Optional source code context
        #[source_code]
        code: Option<String>,
        /// Optional location of the type mismatch
        #[label("type mismatch")]
        span: Option<SourceSpan>,
    },

    /// Unsupported operation between types
    #[error("Unsupported operation: {} on types {} and {}", 
        operation.yellow(), 
        left.cyan(), 
        right.cyan()
    )]
    #[diagnostic(
        code(braise::types::unsupported_operation),
        help("Check that the operation is valid for these types")
    )]
    UnsupportedOperation {
        /// The operation being attempted
        operation: String,
        /// Left operand type
        left: String,
        /// Right operand type
        right: String,
        /// Optional source code context
        #[source_code]
        code: Option<String>,
        /// Optional location of the operation
        #[label("unsupported operation")]
        span: Option<SourceSpan>,
    },

    /// Type conversion failure
    #[error("Cannot convert from {} to {}", 
        from.red(), 
        to.green()
    )]
    #[diagnostic(
        code(braise::types::cannot_convert)
    )]
    CannotConvert {
        /// Source type
        from: String,
        /// Target type
        to: String,
        /// Optional suggestion for fixing the conversion
        suggestion: Option<String>,
        /// Optional source code context
        #[source_code]
        code: Option<String>,
        /// Optional location of the conversion attempt
        #[label("conversion error")]
        span: Option<SourceSpan>,
    },

    /// Invalid type annotation or declaration
    #[error("Invalid type: {reason}")]
    #[diagnostic(
        code(braise::types::invalid_type),
        help("Use valid type syntax like 'string', 'number', '[string]', or 'string | number'")
    )]
    InvalidType {
        /// Reason why the type is invalid
        reason: String,
        /// Optional source code context
        #[source_code]
        code: Option<String>,
        /// Optional location of the invalid type
        #[label("invalid type")]
        span: Option<SourceSpan>,
    },

    /// Type inference failure
    #[error("Cannot infer type: {reason}")]
    #[diagnostic(
        code(braise::types::inference_failure),
        help("Provide explicit type annotations to help with type inference")
    )]
    InferenceFailure {
        /// Reason why inference failed
        reason: String,
        /// Optional source code context
        #[source_code]
        code: Option<String>,
        /// Optional location where inference failed
        #[label("inference failure")]
        span: Option<SourceSpan>,
    },

    /// Type constraint violation
    #[error("Type constraint violated: {constraint}")]
    #[diagnostic(
        code(braise::types::constraint_violation),
        help("Ensure the type satisfies all required constraints")
    )]
    ConstraintViolation {
        /// The constraint that was violated
        constraint: String,
        /// Type that violated the constraint
        violating_type: String,
        /// Optional source code context
        #[source_code]
        code: Option<String>,
        /// Optional location of the violation
        #[label("constraint violation")]
        span: Option<SourceSpan>,
    },
}

impl TypeError {
    /// Create a type mismatch error with context
    pub fn mismatch(expected: impl Into<String>, got: impl Into<String>, context: impl Into<String>) -> Self {
        Self::Mismatch {
            expected: expected.into(),
            got: got.into(),
            context: context.into(),
            suggestion: None,
            code: None,
            span: None,
        }
    }

    /// Create a type mismatch error with suggestion
    pub fn mismatch_with_suggestion(
        expected: impl Into<String>,
        got: impl Into<String>,
        context: impl Into<String>,
        suggestion: impl Into<String>,
    ) -> Self {
        Self::Mismatch {
            expected: expected.into(),
            got: got.into(),
            context: context.into(),
            suggestion: Some(suggestion.into()),
            code: None,
            span: None,
        }
    }

    /// Create a type mismatch error with full context
    pub fn mismatch_with_context(
        expected: impl Into<String>,
        got: impl Into<String>,
        context: impl Into<String>,
        code: impl Into<String>,
        span: SourceSpan,
    ) -> Self {
        let expected = expected.into();
        let got = got.into();
        Self::Mismatch {
            suggestion: Self::suggest_type_fix(&expected, &got),
            expected,
            got,
            context: context.into(),
            code: Some(code.into()),
            span: Some(span),
        }
    }

    /// Create an unsupported operation error
    pub fn unsupported_operation(
        operation: impl Into<String>,
        left: impl Into<String>,
        right: impl Into<String>,
    ) -> Self {
        Self::UnsupportedOperation {
            operation: operation.into(),
            left: left.into(),
            right: right.into(),
            code: None,
            span: None,
        }
    }

    /// Create an unsupported operation error with context
    pub fn unsupported_operation_with_context(
        operation: impl Into<String>,
        left: impl Into<String>,
        right: impl Into<String>,
        code: impl Into<String>,
        span: SourceSpan,
    ) -> Self {
        Self::UnsupportedOperation {
            operation: operation.into(),
            left: left.into(),
            right: right.into(),
            code: Some(code.into()),
            span: Some(span),
        }
    }

    /// Create a conversion error
    pub fn cannot_convert(from: impl Into<String>, to: impl Into<String>) -> Self {
        let from = from.into();
        let to = to.into();
        Self::CannotConvert {
            suggestion: Self::suggest_conversion_fix(&from, &to),
            from,
            to,
            code: None,
            span: None,
        }
    }

    /// Create a conversion error with context
    pub fn cannot_convert_with_context(
        from: impl Into<String>,
        to: impl Into<String>,
        code: impl Into<String>,
        span: SourceSpan,
    ) -> Self {
        let from = from.into();
        let to = to.into();
        Self::CannotConvert {
            suggestion: Self::suggest_conversion_fix(&from, &to),
            from,
            to,
            code: Some(code.into()),
            span: Some(span),
        }
    }

    /// Create an invalid type error
    pub fn invalid_type(reason: impl Into<String>) -> Self {
        Self::InvalidType {
            reason: reason.into(),
            code: None,
            span: None,
        }
    }

    /// Create an invalid type error with context
    pub fn invalid_type_with_context(
        reason: impl Into<String>,
        code: impl Into<String>,
        span: SourceSpan,
    ) -> Self {
        Self::InvalidType {
            reason: reason.into(),
            code: Some(code.into()),
            span: Some(span),
        }
    }

    /// Create an inference failure error
    pub fn inference_failure(reason: impl Into<String>) -> Self {
        Self::InferenceFailure {
            reason: reason.into(),
            code: None,
            span: None,
        }
    }

    /// Create a constraint violation error
    pub fn constraint_violation(
        constraint: impl Into<String>,
        violating_type: impl Into<String>,
    ) -> Self {
        Self::ConstraintViolation {
            constraint: constraint.into(),
            violating_type: violating_type.into(),
            code: None,
            span: None,
        }
    }

    /// Get the severity of this type error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            TypeError::Mismatch { .. } => ErrorSeverity::Error,
            TypeError::UnsupportedOperation { .. } => ErrorSeverity::Error,
            TypeError::CannotConvert { .. } => ErrorSeverity::Error,
            TypeError::InvalidType { .. } => ErrorSeverity::Error,
            TypeError::InferenceFailure { .. } => ErrorSeverity::Warning,
            TypeError::ConstraintViolation { .. } => ErrorSeverity::Error,
        }
    }

    /// Get a helpful suggestion for fixing the error
    pub fn suggestion(&self) -> Option<&str> {
        match self {
            TypeError::Mismatch { suggestion, .. } => suggestion.as_deref(),
            TypeError::CannotConvert { suggestion, .. } => suggestion.as_deref(),
            _ => None,
        }
    }

    /// Add source context to this error
    pub fn with_context(mut self, code: impl Into<String>, span: SourceSpan) -> Self {
        match &mut self {
            TypeError::Mismatch { code: c, span: s, .. } => {
                *c = Some(code.into());
                *s = Some(span);
            }
            TypeError::UnsupportedOperation { code: c, span: s, .. } => {
                *c = Some(code.into());
                *s = Some(span);
            }
            TypeError::CannotConvert { code: c, span: s, .. } => {
                *c = Some(code.into());
                *s = Some(span);
            }
            TypeError::InvalidType { code: c, span: s, .. } => {
                *c = Some(code.into());
                *s = Some(span);
            }
            TypeError::InferenceFailure { code: c, span: s, .. } => {
                *c = Some(code.into());
                *s = Some(span);
            }
            TypeError::ConstraintViolation { code: c, span: s, .. } => {
                *c = Some(code.into());
                *s = Some(span);
            }
        }
        self
    }

    /// Suggest fixes for type conversion errors
    fn suggest_type_fix(expected: &str, got: &str) -> Option<String> {
        match (expected, got) {
            ("number", s) if s.starts_with("string") => {
                Some("Try parsing the string as a number or use a numeric literal".to_string())
            }
            ("bool", s) if s.starts_with("string") => {
                Some("Use \"true\", \"false\", \"1\", \"0\", \"yes\", \"no\", \"on\", or \"off\"".to_string())
            }
            (expected, _) if expected.starts_with('[') => {
                Some("Use array syntax like [\"item1\", \"item2\"]".to_string())
            }
            ("string", "number") => {
                Some("Numbers are automatically converted to strings".to_string())
            }
            _ => None,
        }
    }

    /// Suggest fixes for conversion errors
    fn suggest_conversion_fix(from: &str, to: &str) -> Option<String> {
        match (from, to) {
            ("string", "number") => Some("Ensure the string contains a valid number".to_string()),
            ("string", "bool") => Some("Use \"true\", \"false\", \"1\", \"0\", \"yes\", \"no\", \"on\", or \"off\"".to_string()),
            ("number", "string") => Some("Numbers are automatically converted to strings".to_string()),
            ("bool", "string") => Some("Booleans are automatically converted to strings".to_string()),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_error_creation() {
        let error = TypeError::mismatch("string", "number", "variable assignment");
        
        match error {
            TypeError::Mismatch { expected, got, context, .. } => {
                assert_eq!(expected, "string");
                assert_eq!(got, "number");
                assert_eq!(context, "variable assignment");
            }
            _ => panic!("Expected Mismatch error"),
        }
    }

    #[test]
    fn test_type_error_with_suggestion() {
        let error = TypeError::mismatch_with_suggestion(
            "number", 
            "string", 
            "test", 
            "Use a numeric value"
        );
        
        assert_eq!(error.suggestion(), Some("Use a numeric value"));
    }

    #[test]
    fn test_type_error_severity() {
        assert_eq!(TypeError::mismatch("a", "b", "c").severity(), ErrorSeverity::Error);
        assert_eq!(TypeError::inference_failure("test").severity(), ErrorSeverity::Warning);
    }

    #[test]
    fn test_unsupported_operation_error() {
        let error = TypeError::unsupported_operation("+", "string", "array");
        
        match error {
            TypeError::UnsupportedOperation { operation, left, right, .. } => {
                assert_eq!(operation, "+");
                assert_eq!(left, "string");
                assert_eq!(right, "array");
            }
            _ => panic!("Expected UnsupportedOperation"),
        }
    }

    #[test]
    fn test_cannot_convert_error() {
        let error = TypeError::cannot_convert("string", "number");
        
        match error {
            TypeError::CannotConvert { from, to, suggestion, .. } => {
                assert_eq!(from, "string");
                assert_eq!(to, "number");
                assert!(suggestion.is_some());
            }
            _ => panic!("Expected CannotConvert"),
        }
    }

    #[test]
    fn test_error_with_context() {
        let mut error = TypeError::mismatch("string", "number", "test");
        error = error.with_context("let x: string = 42", (16, 2).into());
        
        match error {
            TypeError::Mismatch { code, span, .. } => {
                assert!(code.is_some());
                assert!(span.is_some());
            }
            _ => panic!("Expected Mismatch with context"),
        }
    }

    #[test]
    fn test_suggestion_generation() {
        // Test type fix suggestions
        assert!(TypeError::suggest_type_fix("number", "string").is_some());
        assert!(TypeError::suggest_type_fix("bool", "string").is_some());
        assert!(TypeError::suggest_type_fix("[string]", "number").is_some());
        assert!(TypeError::suggest_type_fix("unknown", "other").is_none());
        
        // Test conversion fix suggestions
        assert!(TypeError::suggest_conversion_fix("string", "number").is_some());
        assert!(TypeError::suggest_conversion_fix("string", "bool").is_some());
        assert!(TypeError::suggest_conversion_fix("unknown", "other").is_none());
    }
}
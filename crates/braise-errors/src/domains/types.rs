use crate::core::ErrorSeverity;
use miette::{Diagnostic, SourceSpan};
use owo_colors::OwoColorize;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Diagnostic)]
pub enum TypeError {
    #[error("Type mismatch: expected {}, got {} in {}", 
        expected.green(),
        got.red(),
        context.yellow()
    )]
    #[diagnostic(code(braise::types::mismatch))]
    Mismatch {
        expected: String,

        got: String,

        context: String,

        suggestion: Option<String>,

        #[source_code]
        code: Option<String>,

        #[label("type mismatch")]
        span: Option<SourceSpan>,
    },

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
        operation: String,

        left: String,

        right: String,

        #[source_code]
        code: Option<String>,

        #[label("unsupported operation")]
        span: Option<SourceSpan>,
    },

    #[error("Cannot convert from {} to {}", 
        from.red(),
        to.green()
    )]
    #[diagnostic(code(braise::types::cannot_convert))]
    CannotConvert {
        from: String,

        to: String,

        suggestion: Option<String>,

        #[source_code]
        code: Option<String>,

        #[label("conversion error")]
        span: Option<SourceSpan>,
    },

    #[error("Invalid type: {reason}")]
    #[diagnostic(
        code(braise::types::invalid_type),
        help("Use valid type syntax like 'string', 'number', '[string]', or 'string | number'")
    )]
    InvalidType {
        reason: String,

        #[source_code]
        code: Option<String>,

        #[label("invalid type")]
        span: Option<SourceSpan>,
    },

    #[error("Cannot infer type: {reason}")]
    #[diagnostic(
        code(braise::types::inference_failure),
        help("Provide explicit type annotations to help with type inference")
    )]
    InferenceFailure {
        reason: String,

        #[source_code]
        code: Option<String>,

        #[label("inference failure")]
        span: Option<SourceSpan>,
    },

    #[error("Type constraint violated: {constraint}")]
    #[diagnostic(
        code(braise::types::constraint_violation),
        help("Ensure the type satisfies all required constraints")
    )]
    ConstraintViolation {
        constraint: String,

        violating_type: String,

        #[source_code]
        code: Option<String>,

        #[label("constraint violation")]
        span: Option<SourceSpan>,
    },

    #[error("Division by zero is not allowed")]
    #[diagnostic(code(braise::types::division_by_zero))]
    DivisionByZero {
        #[source_code]
        code: Option<String>,
        #[label("division by zero")]
        span: Option<SourceSpan>,
    },
}

impl TypeError {
    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }

    pub fn mismatch(expected: impl ToString, got: impl ToString, context: impl ToString) -> Self {
        Self::Mismatch {
            expected: expected.to_string(),
            got: got.to_string(),
            context: context.to_string(),
            suggestion: None,
            code: None,
            span: None,
        }
    }

    pub fn mismatch_with_suggestion(
        expected: impl ToString,
        got: impl ToString,
        context: impl ToString,
        suggestion: impl ToString,
    ) -> Self {
        Self::Mismatch {
            expected: expected.to_string(),
            got: got.to_string(),
            context: context.to_string(),
            suggestion: Some(suggestion.to_string()),
            code: None,
            span: None,
        }
    }

    pub fn mismatch_with_context(
        expected: impl ToString,
        got: impl ToString,
        context: impl ToString,
        code: impl ToString,
        span: SourceSpan,
    ) -> Self {
        let expected = expected.to_string();
        let got = got.to_string();
        Self::Mismatch {
            suggestion: Self::suggest_type_fix(&expected, &got),
            expected,
            got,
            context: context.to_string(),
            code: Some(code.to_string()),
            span: Some(span),
        }
    }

    pub fn unsupported_operation(
        operation: impl ToString,
        left: impl ToString,
        right: impl ToString,
    ) -> Self {
        Self::UnsupportedOperation {
            operation: operation.to_string(),
            left: left.to_string(),
            right: right.to_string(),
            code: None,
            span: None,
        }
    }

    pub fn unsupported_operation_with_context(
        operation: impl ToString,
        left: impl ToString,
        right: impl ToString,
        code: impl ToString,
        span: SourceSpan,
    ) -> Self {
        Self::UnsupportedOperation {
            operation: operation.to_string(),
            left: left.to_string(),
            right: right.to_string(),
            code: Some(code.to_string()),
            span: Some(span),
        }
    }

    pub fn cannot_convert(from: impl ToString, to: impl ToString) -> Self {
        let from = from.to_string();
        let to = to.to_string();
        Self::CannotConvert {
            suggestion: Self::suggest_conversion_fix(&from, &to),
            from,
            to,
            code: None,
            span: None,
        }
    }

    pub fn cannot_convert_with_context(
        from: impl ToString,
        to: impl ToString,
        code: impl ToString,
        span: SourceSpan,
    ) -> Self {
        let from = from.to_string();
        let to = to.to_string();
        Self::CannotConvert {
            suggestion: Self::suggest_conversion_fix(&from, &to),
            from,
            to,
            code: Some(code.to_string()),
            span: Some(span),
        }
    }

    pub fn invalid_type(reason: impl ToString) -> Self {
        Self::InvalidType {
            reason: reason.to_string(),
            code: None,
            span: None,
        }
    }

    pub fn invalid_type_with_context(
        reason: impl ToString,
        code: impl ToString,
        span: SourceSpan,
    ) -> Self {
        Self::InvalidType {
            reason: reason.to_string(),
            code: Some(code.to_string()),
            span: Some(span),
        }
    }

    pub fn inference_failure(reason: impl ToString) -> Self {
        Self::InferenceFailure {
            reason: reason.to_string(),
            code: None,
            span: None,
        }
    }

    pub fn constraint_violation(constraint: impl ToString, violating_type: impl ToString) -> Self {
        Self::ConstraintViolation {
            constraint: constraint.to_string(),
            violating_type: violating_type.to_string(),
            code: None,
            span: None,
        }
    }

    pub fn division_by_zero() -> Self {
        Self::DivisionByZero {
            code: None,
            span: None,
        }
    }

    pub fn severity(&self) -> ErrorSeverity {
        match self {
            TypeError::Mismatch { .. } => ErrorSeverity::Error,
            TypeError::UnsupportedOperation { .. } => ErrorSeverity::Error,
            TypeError::CannotConvert { .. } => ErrorSeverity::Error,
            TypeError::InvalidType { .. } => ErrorSeverity::Error,
            TypeError::InferenceFailure { .. } => ErrorSeverity::Warning,
            TypeError::ConstraintViolation { .. } => ErrorSeverity::Error,
            TypeError::DivisionByZero { .. } => ErrorSeverity::Error,
        }
    }

    pub fn suggestion(&self) -> Option<&str> {
        match self {
            TypeError::Mismatch { suggestion, .. } => suggestion.as_deref(),
            TypeError::CannotConvert { suggestion, .. } => suggestion.as_deref(),
            _ => None,
        }
    }

    pub fn with_context(mut self, code: impl ToString, span: SourceSpan) -> Self {
        match &mut self {
            TypeError::Mismatch {
                code: c, span: s, ..
            } => {
                *c = Some(code.to_string());
                *s = Some(span);
            }
            TypeError::UnsupportedOperation {
                code: c, span: s, ..
            } => {
                *c = Some(code.to_string());
                *s = Some(span);
            }
            TypeError::CannotConvert {
                code: c, span: s, ..
            } => {
                *c = Some(code.to_string());
                *s = Some(span);
            }
            TypeError::InvalidType {
                code: c, span: s, ..
            } => {
                *c = Some(code.to_string());
                *s = Some(span);
            }
            TypeError::InferenceFailure {
                code: c, span: s, ..
            } => {
                *c = Some(code.to_string());
                *s = Some(span);
            }
            TypeError::ConstraintViolation {
                code: c, span: s, ..
            } => {
                *c = Some(code.to_string());
                *s = Some(span);
            }
            TypeError::DivisionByZero { code: c, span: s } => {
                *c = Some(code.to_string());
                *s = Some(span);
            }
        }
        self
    }

    fn suggest_type_fix(expected: &str, got: &str) -> Option<String> {
        match (expected, got) {
            ("number", s) if s.starts_with("string") => {
                Some("Try parsing the string as a number or use a numeric literal".to_string())
            }
            ("bool", s) if s.starts_with("string") => Some(
                "Use \"true\", \"false\", \"1\", \"0\", \"yes\", \"no\", \"on\", or \"off\""
                    .to_string(),
            ),
            (expected, _) if expected.starts_with('[') => {
                Some("Use array syntax like [\"item1\", \"item2\"]".to_string())
            }
            ("string", "number") => {
                Some("Numbers are automatically converted to strings".to_string())
            }
            _ => None,
        }
    }

    fn suggest_conversion_fix(from: &str, to: &str) -> Option<String> {
        match (from, to) {
            ("string", "number") => Some("Ensure the string contains a valid number".to_string()),
            ("string", "bool") => Some(
                "Use \"true\", \"false\", \"1\", \"0\", \"yes\", \"no\", \"on\", or \"off\""
                    .to_string(),
            ),
            ("number", "string") => {
                Some("Numbers are automatically converted to strings".to_string())
            }
            ("bool", "string") => {
                Some("Booleans are automatically converted to strings".to_string())
            }
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
            TypeError::Mismatch {
                expected,
                got,
                context,
                ..
            } => {
                assert_eq!(expected, "string");
                assert_eq!(got, "number");
                assert_eq!(context, "variable assignment");
            }
            _ => panic!("Expected Mismatch error"),
        }
    }

    #[test]
    fn test_type_error_with_suggestion() {
        let error =
            TypeError::mismatch_with_suggestion("number", "string", "test", "Use a numeric value");

        assert_eq!(error.suggestion(), Some("Use a numeric value"));
    }

    #[test]
    fn test_type_error_severity() {
        assert_eq!(
            TypeError::mismatch("a", "b", "c").severity(),
            ErrorSeverity::Error
        );
        assert_eq!(
            TypeError::inference_failure("test").severity(),
            ErrorSeverity::Warning
        );
    }

    #[test]
    fn test_unsupported_operation_error() {
        let error = TypeError::unsupported_operation("+", "string", "array");

        match error {
            TypeError::UnsupportedOperation {
                operation,
                left,
                right,
                ..
            } => {
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
            TypeError::CannotConvert {
                from,
                to,
                suggestion,
                ..
            } => {
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

use crate::BraiseType;
use miette::{Diagnostic, SourceSpan};
use owo_colors::OwoColorize;
use thiserror::Error;

/// Type-related errors in the Braise language
#[derive(Error, Debug, Clone, PartialEq)]
pub enum TypeError {
    #[error("Type mismatch: expected {expected}, got {got} in {context}")]
    Mismatch {
        expected: String,
        got: String,
        context: String,
    },

    #[error("Unsupported operation: {operation} on types {left} and {right}")]
    UnsupportedOperation {
        operation: String,
        left: String,
        right: String,
    },

    #[error("Cannot convert from {from} to {to}")]
    CannotConvert { from: String, to: String },
}

impl TypeError {
    /// Create a type mismatch error with context
    pub fn mismatch(
        expected: impl Into<String>,
        got: impl Into<String>,
        context: impl Into<String>,
    ) -> Self {
        Self::Mismatch {
            expected: expected.into(),
            got: got.into(),
            context: context.into(),
        }
    }

    /// Create an unsupported operation error
    pub fn unsupported_operation(
        operation: impl Into<String>,
        left: &BraiseType,
        right: &BraiseType,
    ) -> Self {
        Self::UnsupportedOperation {
            operation: operation.into(),
            left: left.to_string(),
            right: right.to_string(),
        }
    }

    /// Create a conversion error
    pub fn cannot_convert(from: &BraiseType, to: &BraiseType) -> Self {
        Self::CannotConvert {
            from: from.to_string(),
            to: to.to_string(),
        }
    }

    /// Get a suggestion for fixing this type error
    pub fn suggestion(&self) -> Option<String> {
        match self {
            TypeError::Mismatch { expected, got, .. } => suggest_type_fix(expected, got),
            TypeError::CannotConvert { from, to } => suggest_conversion_fix(from, to),
            _ => None,
        }
    }

    /// Create a colored, user-friendly error message
    pub fn colored_message(&self) -> String {
        match self {
            TypeError::Mismatch {
                expected,
                got,
                context,
            } => {
                format!(
                    "Type mismatch in {}: expected {}, got {}",
                    context.yellow(),
                    expected.green(),
                    got.red()
                )
            }
            TypeError::UnsupportedOperation {
                operation,
                left,
                right,
            } => {
                format!(
                    "Cannot apply {} to {} and {}",
                    operation.yellow(),
                    left.cyan(),
                    right.cyan()
                )
            }
            TypeError::CannotConvert { from, to } => {
                format!("Cannot convert {} to {}", from.red(), to.green())
            }
        }
    }
}

/// Suggest fixes for type conversion errors
fn suggest_type_fix(expected: &str, got: &str) -> Option<String> {
    match (expected, got) {
        ("number", s) if s.starts_with("string") => {
            Some("Try using a numeric string like \"42\"".to_string())
        }
        ("boolean or boolean-convertible", s) if s.starts_with("string") => {
            Some("Use \"true\"/\"false\", \"1\"/\"0\", or \"yes\"/\"no\"".to_string())
        }
        ("array", _) => Some("Use array syntax like [\"item1\", \"item2\"]".to_string()),
        _ => None,
    }
}

/// Suggest fixes for conversion errors
fn suggest_conversion_fix(from: &str, to: &str) -> Option<String> {
    match (from, to) {
        ("string", "number") => Some("Ensure the string contains a valid number".to_string()),
        ("string", "bool") => Some(
            "Use \"true\", \"false\", \"1\", \"0\", \"yes\", \"no\", \"on\", or \"off\""
                .to_string(),
        ),
        ("number", "string") => Some("Numbers are automatically converted to strings".to_string()),
        ("bool", "string") => Some("Booleans are automatically converted to strings".to_string()),
        _ => None,
    }
}

/// Error wrapper that includes source context for miette diagnostics
#[derive(Error, Debug, Diagnostic)]
#[error("Type error")]
pub struct TypeErrorWithContext {
    #[source]
    pub source: TypeError,

    #[source_code]
    pub code: Option<String>,

    #[label("here")]
    pub span: Option<SourceSpan>,

    #[help]
    pub suggestion: Option<String>,
}

impl TypeErrorWithContext {
    pub fn new(error: TypeError) -> Self {
        let suggestion = error.suggestion();
        Self {
            source: error,
            code: None,
            span: None,
            suggestion,
        }
    }

    pub fn with_source_context(mut self, code: String, span: SourceSpan) -> Self {
        self.code = Some(code);
        self.span = Some(span);
        self
    }

    pub fn with_suggestion(mut self, suggestion: String) -> Self {
        self.suggestion = Some(suggestion);
        self
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
            } => {
                assert_eq!(expected, "string");
                assert_eq!(got, "number");
                assert_eq!(context, "variable assignment");
            }
            _ => panic!("Expected Mismatch error"),
        }
    }

    #[test]
    fn test_error_suggestions() {
        let error = TypeError::mismatch("number", "string", "test");
        assert!(error.suggestion().is_some());

        let error = TypeError::cannot_convert(&BraiseType::String, &BraiseType::Number);
        assert!(error.suggestion().is_some());
    }

    #[test]
    fn test_colored_messages() {
        let error = TypeError::mismatch("string", "number", "test context");
        let message = error.colored_message();
        assert!(message.contains("Type mismatch"));
        assert!(message.contains("test context"));
    }
}

use miette::{Diagnostic, SourceSpan};
use owo_colors::OwoColorize;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum ParseError {
    #[error("Unexpected token: expected {}, found {}", expected.bold().green(), found.bold().red())]
    #[diagnostic(code(braise::parser::unexpected_token))]
    UnexpectedToken {
        expected: String,
        found: String,
        #[source_code]
        code: String,
        #[label("right here")]
        span: SourceSpan,
    },

    #[error("Invalid expression: {reason}")]
    #[diagnostic(code(braise::parser::invalid_expression))]
    InvalidExpression {
        reason: String,
        #[source_code]
        code: String,
        #[label("right here")]
        span: SourceSpan,
    },
    // #[error("Non-exhaustive match: consider adding a wildcard pattern '_'")]
    #[error("Non-exhaustive match: {}", missing.as_deref().map(|s| format!("missing: '{}'", s)).unwrap_or("consider adding a wildcard pattern '_'".to_string()))]
    #[diagnostic(code(braise::parser::non_exhaustive_match))]
    NonExhaustiveMatch {
        #[source_code]
        code: String,
        #[label("right here")]
        span: SourceSpan,
        missing: Option<String>,
    },

    #[error("Error: {0}")]
    #[diagnostic(code(braise::parser::other))]
    Other(String),
}

pub type Result<T> = std::result::Result<T, ParseError>;

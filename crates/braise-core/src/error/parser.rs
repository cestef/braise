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

    #[error("Invalid expression")]
    InvalidExpression {
        expression: String,
        #[source_code]
        code: String,
        #[label("right here")]
        span: SourceSpan,
    },

    #[error("Error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, ParseError>;

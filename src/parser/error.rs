#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Unexpected token: expected {expected:?}, found {found:?}")]
    UnexpectedToken {
        expected: String,
        found: crate::lexer::Token,
    },
    #[error("Unexpected end of file while parsing")]
    UnexpectedEof,
    #[error("Invalid expression: {0}")]
    InvalidExpression(String),

    #[error("Error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, ParseError>;

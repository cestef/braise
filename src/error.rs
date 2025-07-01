use miette::Diagnostic;

use crate::{parser::ParseError, runtime::RuntimeError};

#[derive(Debug, thiserror::Error, Diagnostic)]
pub enum BraiseError {
    #[error("Unrecognized token")]
    #[diagnostic(code(braise::lexer))]
    LexerError,
    #[error(transparent)]
    #[diagnostic(code(braise::parser))]
    ParserError(#[from] ParseError),
    #[error(transparent)]
    #[diagnostic(code(braise::runtime))]
    RuntimeError(#[from] RuntimeError),

    #[error(transparent)]
    #[diagnostic(code(braise::io))]
    IoError(#[from] std::io::Error),
}

pub type Result<T, E = BraiseError> = std::result::Result<T, E>;

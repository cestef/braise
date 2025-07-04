pub mod cli;
pub mod parser;
pub mod runtime;

use crate::{cli::CliError, parser::ParseError, runtime::RuntimeError};
use miette::{Diagnostic, SourceSpan};

#[derive(Debug, thiserror::Error, Diagnostic)]
pub enum BraiseError {
    #[error("Unrecognized token")]
    #[diagnostic(code(braise::lexer))]
    LexerError {
        #[source_code]
        code: String,
        #[label("right here")]
        span: SourceSpan,
    },
    #[error(transparent)]
    #[diagnostic(transparent)]
    ParserError(#[from] ParseError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    RuntimeError(#[from] RuntimeError),
    #[error(transparent)]
    #[diagnostic(transparent)]
    CliError(#[from] CliError),
}

pub type Result<T, E = BraiseError> = std::result::Result<T, E>;

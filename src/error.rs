use miette::{Diagnostic, SourceSpan};
use owo_colors::OwoColorize;

use crate::{parser::ParseError, runtime::RuntimeError};

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

    #[error("No task provided")]
    #[diagnostic(code(braise::no_task))]
    NoTask,

    #[error(
        "No recipe file found, searched for: {}",
        crate::constants::DEFAULT_FILES.iter().map(|e| e.bold().to_string()).collect::<Vec<_>>().join(", ")
    )]
    #[diagnostic(code(braise::no_recipe))]
    NoRecipeFileFound,

    #[error("Could not read recipe file")]
    #[diagnostic(code(braise::read_recipe))]
    ReadRecipeError {
        #[source]
        src: Box<dyn std::error::Error + Send + Sync>,
        file: String,
    },
}

pub type Result<T, E = BraiseError> = std::result::Result<T, E>;

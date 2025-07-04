use miette::Diagnostic;
use owo_colors::OwoColorize;
use thiserror::Error;

#[derive(Debug, Error, Diagnostic)]
pub enum CliError {
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

pub type Result<T> = std::result::Result<T, CliError>;

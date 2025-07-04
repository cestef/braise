use std::collections::HashSet;

use miette::Diagnostic;
use owo_colors::OwoColorize;

#[derive(Debug, thiserror::Error, Diagnostic, Clone)]
pub enum RuntimeError {
    #[error("Undefined variable: {0}")]
    #[diagnostic(code(braise::runtime::undefined_variable))]
    UndefinedVariable(String),

    #[error("Undefined recipe: {0}")]
    #[diagnostic(code(braise::runtime::undefined_recipe))]
    UndefinedRecipe(String),

    #[error("Type error: {0}")]
    #[diagnostic(code(braise::runtime::type_error))]
    TypeError(String),

    #[error("Command failed: {command} (exit code: {exit_code})", command = .command.bold().green(), exit_code = .exit_code)]
    #[diagnostic(code(braise::runtime::command_failed))]
    CommandFailed { command: String, exit_code: i32 },

    #[error("Invalid parameter: {name} (expected: {}, got: {})", expected.green(), got.red(), name = .name.bold())]
    #[diagnostic(code(braise::runtime::invalid_parameter))]
    InvalidParameter {
        name: String,
        expected: String,
        got: String,
    },

    #[error("Builtin error: {0}")]
    #[diagnostic(code(braise::runtime::builtin_error))]
    BuiltinError(String),

    #[error("Exit called with code: {0}")]
    #[diagnostic(code(braise::runtime::exit))]
    Exit(i32),

    #[error("Error executing command: {0}")]
    #[diagnostic(code(braise::runtime::other))]
    Other(String),

    #[error("Circular dependency detected in recipe: {recipe} (stack: {stack:?})")]
    #[diagnostic(code(braise::runtime::circular_dependency))]
    CircularDependency {
        recipe: String,
        stack: HashSet<String>,
    },

    #[error("Match expression has no arms for value: {value}")]
    #[diagnostic(code(braise::runtime::match_no_arm))]
    MatchNoArm { value: String },
}

pub type Result<T> = std::result::Result<T, RuntimeError>;

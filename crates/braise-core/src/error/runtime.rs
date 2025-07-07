use std::collections::HashSet;

use miette::Diagnostic;
use owo_colors::OwoColorize;

use crate::error::types::TypeError;

#[derive(Debug, thiserror::Error, Diagnostic, Clone)]
pub enum RuntimeError {
    #[error("Undefined variable: {0}")]
    #[diagnostic(code(braise::runtime::undefined_variable))]
    UndefinedVariable(String),

    #[error("Undefined recipe: {0}")]
    #[diagnostic(code(braise::runtime::undefined_recipe))]
    UndefinedRecipe(String),

    #[error("Type error")]
    #[diagnostic(code(braise::runtime::type_error))]
    TypeError {
        source: TypeError,
        #[source_code]
        code: Option<String>,
        #[label("right here")]
        span: Option<miette::SourceSpan>,
    },

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

    #[error("Missing required parameter: {name} ({expected_type})",
        name = .name.bold(),
        expected_type = .expected_type.green()
    )]
    #[diagnostic(code(braise::runtime::missing_required_parameter))]
    #[help("You can also provide a default value for this parameter in the recipe definition.")]
    MissingRequiredParameter { name: String, expected_type: String },

    #[error("Builtin error: {0}")]
    #[diagnostic(code(braise::runtime::builtin_error))]
    BuiltinError(String),

    #[error("Exit called with code: {0}")]
    #[diagnostic(code(braise::runtime::exit))]
    Exit(i32),

    #[error("Error executing command: {0}")]
    #[diagnostic(code(braise::runtime::other))]
    Other(String),

    #[error("Circular dependency detected in recipe: {recipe} (stack: {stack})",
        recipe = .recipe.bold(),
        stack = .stack.iter().map(|s| s.bold().to_string()).collect::<Vec<_>>().join(", ")
    )]
    #[diagnostic(code(braise::runtime::circular_dependency))]
    #[help("This usually means that the recipe is trying to call itself directly or indirectly.")]
    CircularDependency {
        recipe: String,
        stack: HashSet<String>,
    },

    #[error("Match expression has no arms for value: {value}")]
    #[diagnostic(code(braise::runtime::match_no_arm))]
    #[help("Ensure that all possible values are covered by the match arms.")]
    MatchNoArm { value: String },
}

pub type Result<T, E = RuntimeError> = std::result::Result<T, E>;

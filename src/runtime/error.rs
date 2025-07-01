#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    #[error("Undefined variable: {0}")]
    UndefinedVariable(String),
    #[error("Undefined recipe: {0}")]
    UndefinedRecipe(String),
    #[error("Type error: {0}")]
    TypeError(String),
    #[error("Command failed: {command} (exit code: {exit_code})")]
    CommandFailed { command: String, exit_code: i32 },
    #[error("Invalid parameter: {name} (expected: {expected}, got: {got})")]
    InvalidParameter {
        name: String,
        expected: String,
        got: String,
    },
    #[error("Builtin error: {0}")]
    BuiltinError(String),
    #[error("Exit called with code: {0}")]
    Exit(i32),
    #[error("Error executing command: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, RuntimeError>;

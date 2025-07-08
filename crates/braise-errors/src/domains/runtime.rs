use crate::core::ErrorSeverity;
use crate::domains::TypeError;
use miette::{Diagnostic, SourceSpan};
use owo_colors::OwoColorize;
use std::collections::HashSet;
use thiserror::Error;

/// Runtime execution errors
#[derive(Debug, Error, Diagnostic, Clone)]
pub enum RuntimeError {
    /// Undefined variable reference
    #[error("Undefined variable: '{name}'")]
    #[diagnostic(
        code(braise::runtime::undefined_variable),
        help("Make sure the variable is defined before using it")
    )]
    UndefinedVariable {
        /// Name of the undefined variable
        name: String,
        /// Optional source context
        #[source_code]
        code: Option<String>,
        /// Optional location of the variable reference
        #[label("undefined variable")]
        span: Option<SourceSpan>,
    },

    /// Undefined recipe reference
    #[error("Undefined recipe: '{name}'")]
    #[diagnostic(
        code(braise::runtime::undefined_recipe),
        help("Check that the recipe is defined in the current file or included files")
    )]
    UndefinedRecipe {
        /// Name of the undefined recipe
        name: String,
        /// Optional source context
        #[source_code]
        code: Option<String>,
        /// Optional location of the recipe reference
        #[label("undefined recipe")]
        span: Option<SourceSpan>,
    },

    /// Type system errors during runtime
    #[error("Type error")]
    #[diagnostic(code(braise::runtime::type_error))]
    Type {
        /// The underlying type error
        #[source]
        source: TypeError,
        /// Source code context
        #[source_code]
        code: Option<String>,
        /// Location of the type error
        #[label("type error here")]
        span: Option<SourceSpan>,
    },

    /// External command execution failure
    #[error("Command failed: {command} (exit code: {exit_code})", 
        command = .command.bold().green(), 
        exit_code = .exit_code
    )]
    #[diagnostic(
        code(braise::runtime::command_failed),
        help("Check the command syntax and ensure required tools are installed")
    )]
    CommandFailed {
        /// The command that failed
        command: String,
        /// Exit code returned by the command
        exit_code: i32,
        /// Optional stderr output
        stderr: Option<String>,
        /// Optional source context
        #[source_code]
        code: Option<String>,
        /// Optional location where command was invoked
        #[label("command executed here")]
        span: Option<SourceSpan>,
    },

    /// Invalid parameter value
    #[error("Invalid parameter: {name} (expected: {}, got: {})", 
        expected.green(), 
        got.red(), 
        name = .name.bold()
    )]
    #[diagnostic(
        code(braise::runtime::invalid_parameter),
        help("Check the parameter type and provide a valid value")
    )]
    InvalidParameter {
        /// Parameter name
        name: String,
        /// Expected parameter type/format
        expected: String,
        /// Actual value provided
        got: String,
        /// Optional source context
        #[source_code]
        code: Option<String>,
        /// Optional location of the parameter
        #[label("invalid parameter")]
        span: Option<SourceSpan>,
    },

    /// Missing required parameter
    #[error("Missing required parameter: {name} ({expected_type})",
        name = .name.bold(),
        expected_type = .expected_type.green()
    )]
    #[diagnostic(
        code(braise::runtime::missing_required_parameter),
        help("Provide the required parameter or add a default value in the recipe definition")
    )]
    MissingRequiredParameter {
        /// Parameter name
        name: String,
        /// Expected parameter type
        expected_type: String,
        /// Optional source context
        #[source_code]
        code: Option<String>,
        /// Optional location of the recipe call
        #[label("recipe called here")]
        span: Option<SourceSpan>,
    },

    /// Built-in module errors
    #[error("Builtin module error: {message}")]
    #[diagnostic(code(braise::runtime::builtin_error))]
    BuiltinError {
        /// Error message from the builtin module
        message: String,
        /// Module name where the error occurred
        module: String,
        /// Function name where the error occurred
        function: Option<String>,
        /// Optional source context
        #[source_code]
        code: Option<String>,
        /// Optional location of the builtin call
        #[label("builtin call")]
        span: Option<SourceSpan>,
    },

    /// Explicit exit from recipe execution
    #[error("Exit called with code: {code}")]
    #[diagnostic(code(braise::runtime::exit))]
    Exit {
        /// Exit code
        code: i32,
        /// Optional exit message
        message: Option<String>,
        /// Optional source context
        #[source_code]
        source_code: Option<String>,
        /// Optional location of the exit call
        #[label("exit called here")]
        span: Option<SourceSpan>,
    },

    /// Circular dependency in recipe calls
    #[error("Circular dependency detected in recipe: {recipe} (stack: {stack})",
        recipe = .recipe.bold(),
        stack = .stack.iter().map(|s| s.bold().to_string()).collect::<Vec<_>>().join(" → ")
    )]
    #[diagnostic(
        code(braise::runtime::circular_dependency),
        help("Remove the circular dependency by restructuring recipe dependencies or calls")
    )]
    CircularDependency {
        /// Recipe where circular dependency was detected
        recipe: String,
        /// Call stack showing the dependency cycle
        stack: HashSet<String>,
        /// Optional source context
        #[source_code]
        code: Option<String>,
        /// Optional location of the recipe call
        #[label("circular dependency")]
        span: Option<SourceSpan>,
    },

    /// Pattern matching exhaustion failure
    #[error("Match expression has no arms for value: {value}")]
    #[diagnostic(
        code(braise::runtime::match_no_arm),
        help("Add a wildcard pattern '_' or cover all possible values")
    )]
    MatchNoArm {
        /// Value that couldn't be matched
        value: String,
        /// Optional source context
        #[source_code]
        code: Option<String>,
        /// Optional location of the match expression
        #[label("match expression")]
        span: Option<SourceSpan>,
    },

    /// I/O operation failures
    #[error("I/O error: {message}")]
    #[diagnostic(
        code(braise::runtime::io_error),
        help("Check file permissions and path accessibility")
    )]
    IoError {
        /// I/O error message
        message: String,
        /// File path involved in the operation
        path: Option<String>,
        /// Type of I/O operation
        operation: String,
        /// Optional source context
        #[source_code]
        code: Option<String>,
        /// Optional location of the I/O operation
        #[label("I/O operation")]
        span: Option<SourceSpan>,
    },

    /// Generic runtime errors
    #[error("Runtime error: {message}")]
    #[diagnostic(code(braise::runtime::other))]
    Other {
        /// Error message
        message: String,
        /// Optional source context
        #[source_code]
        code: Option<String>,
        /// Optional error location
        #[label("error location")]
        span: Option<SourceSpan>,
    },
}

impl RuntimeError {
    /// Create an undefined variable error
    pub fn undefined_variable(name: impl Into<String>) -> Self {
        Self::UndefinedVariable {
            name: name.into(),
            code: None,
            span: None,
        }
    }

    /// Create an undefined variable error with context
    pub fn undefined_variable_with_context(
        name: impl Into<String>,
        code: impl Into<String>,
        span: SourceSpan,
    ) -> Self {
        Self::UndefinedVariable {
            name: name.into(),
            code: Some(code.into()),
            span: Some(span),
        }
    }

    /// Create an undefined recipe error
    pub fn undefined_recipe(name: impl Into<String>) -> Self {
        Self::UndefinedRecipe {
            name: name.into(),
            code: None,
            span: None,
        }
    }

    /// Create a type error
    pub fn type_error(source: TypeError) -> Self {
        Self::Type {
            source,
            code: None,
            span: None,
        }
    }

    /// Create a type error with context
    pub fn type_error_with_context(
        source: TypeError,
        code: impl Into<String>,
        span: SourceSpan,
    ) -> Self {
        Self::Type {
            source,
            code: Some(code.into()),
            span: Some(span),
        }
    }

    /// Create a command failed error
    pub fn command_failed(command: impl Into<String>, exit_code: i32) -> Self {
        Self::CommandFailed {
            command: command.into(),
            exit_code,
            stderr: None,
            code: None,
            span: None,
        }
    }

    /// Create a command failed error with stderr
    pub fn command_failed_with_stderr(
        command: impl Into<String>,
        exit_code: i32,
        stderr: impl Into<String>,
    ) -> Self {
        Self::CommandFailed {
            command: command.into(),
            exit_code,
            stderr: Some(stderr.into()),
            code: None,
            span: None,
        }
    }

    /// Create an invalid parameter error
    pub fn invalid_parameter(
        name: impl Into<String>,
        expected: impl Into<String>,
        got: impl Into<String>,
    ) -> Self {
        Self::InvalidParameter {
            name: name.into(),
            expected: expected.into(),
            got: got.into(),
            code: None,
            span: None,
        }
    }

    /// Create a missing required parameter error
    pub fn missing_required_parameter(
        name: impl Into<String>,
        expected_type: impl Into<String>,
    ) -> Self {
        Self::MissingRequiredParameter {
            name: name.into(),
            expected_type: expected_type.into(),
            code: None,
            span: None,
        }
    }

    /// Create a builtin error
    pub fn builtin_error(
        message: impl Into<String>,
        module: impl Into<String>,
    ) -> Self {
        Self::BuiltinError {
            message: message.into(),
            module: module.into(),
            function: None,
            code: None,
            span: None,
        }
    }

    /// Create an exit error
    pub fn exit(code: i32) -> Self {
        Self::Exit {
            code,
            message: None,
            source_code: None,
            span: None,
        }
    }

    /// Create a circular dependency error
    pub fn circular_dependency(recipe: impl Into<String>, stack: HashSet<String>) -> Self {
        Self::CircularDependency {
            recipe: recipe.into(),
            stack,
            code: None,
            span: None,
        }
    }

    /// Create a match no arm error
    pub fn match_no_arm(value: impl Into<String>) -> Self {
        Self::MatchNoArm {
            value: value.into(),
            code: None,
            span: None,
        }
    }

    /// Create an I/O error
    pub fn io_error(
        message: impl Into<String>,
        operation: impl Into<String>,
    ) -> Self {
        Self::IoError {
            message: message.into(),
            path: None,
            operation: operation.into(),
            code: None,
            span: None,
        }
    }

    /// Create a generic runtime error
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other {
            message: message.into(),
            code: None,
            span: None,
        }
    }

    /// Get the severity of this runtime error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            RuntimeError::UndefinedVariable { .. } => ErrorSeverity::Error,
            RuntimeError::UndefinedRecipe { .. } => ErrorSeverity::Error,
            RuntimeError::Type { .. } => ErrorSeverity::Error,
            RuntimeError::CommandFailed { .. } => ErrorSeverity::Error,
            RuntimeError::InvalidParameter { .. } => ErrorSeverity::Error,
            RuntimeError::MissingRequiredParameter { .. } => ErrorSeverity::Error,
            RuntimeError::BuiltinError { .. } => ErrorSeverity::Error,
            RuntimeError::Exit { .. } => ErrorSeverity::Info,
            RuntimeError::CircularDependency { .. } => ErrorSeverity::Critical,
            RuntimeError::MatchNoArm { .. } => ErrorSeverity::Error,
            RuntimeError::IoError { .. } => ErrorSeverity::Error,
            RuntimeError::Other { .. } => ErrorSeverity::Error,
        }
    }

    /// Check if this is an exit error
    pub fn is_exit(&self) -> bool {
        matches!(self, RuntimeError::Exit { .. })
    }

    /// Get the exit code if this is an exit error
    pub fn exit_code(&self) -> Option<i32> {
        match self {
            RuntimeError::Exit { code, .. } => Some(*code),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_error_creation() {
        let error = RuntimeError::undefined_variable("test_var");
        
        match error {
            RuntimeError::UndefinedVariable { name, .. } => {
                assert_eq!(name, "test_var");
            }
            _ => panic!("Expected UndefinedVariable"),
        }
    }

    #[test]
    fn test_runtime_error_severity() {
        assert_eq!(RuntimeError::undefined_variable("x").severity(), ErrorSeverity::Error);
        assert_eq!(RuntimeError::exit(0).severity(), ErrorSeverity::Info);
        assert_eq!(
            RuntimeError::circular_dependency("test", HashSet::new()).severity(),
            ErrorSeverity::Critical
        );
    }

    #[test]
    fn test_exit_error_handling() {
        let error = RuntimeError::exit(42);
        assert!(error.is_exit());
        assert_eq!(error.exit_code(), Some(42));
        
        let error = RuntimeError::undefined_variable("x");
        assert!(!error.is_exit());
        assert_eq!(error.exit_code(), None);
    }

    #[test]
    fn test_command_failed_with_stderr() {
        let error = RuntimeError::command_failed_with_stderr("ls /invalid", 1, "No such file");
        
        match error {
            RuntimeError::CommandFailed { command, exit_code, stderr, .. } => {
                assert_eq!(command, "ls /invalid");
                assert_eq!(exit_code, 1);
                assert_eq!(stderr, Some("No such file".to_string()));
            }
            _ => panic!("Expected CommandFailed"),
        }
    }
}
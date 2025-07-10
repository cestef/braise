use crate::core::ErrorSeverity;
use miette::Diagnostic;
use owo_colors::OwoColorize;
use thiserror::Error;

/// Command-line interface errors
#[derive(Debug, Error, Diagnostic)]
pub enum CliError {
    /// No task or recipe name provided
    #[error("No task provided")]
    #[diagnostic(
        code(braise::cli::no_task),
        help("Specify a recipe name or use 'braise list' to see available recipes")
    )]
    NoTask,

    /// No recipe file found in default locations
    #[error(
        "No recipe file found, searched for: {}",
        format_file_list(DEFAULT_FILES)
    )]
    #[diagnostic(
        code(braise::cli::no_recipe_file),
        help("Create a 'Braisefile' in the current directory or specify a file with --file")
    )]
    NoRecipeFileFound,

    /// Could not read the recipe file
    #[error("Could not read recipe file: {file}")]
    #[diagnostic(code(braise::cli::read_recipe_error))]
    ReadRecipeError {
        /// The underlying I/O error
        #[source]
        src: Box<dyn std::error::Error + Send + Sync>,
        /// The file path that failed to read
        file: String,
    },

    /// Invalid command-line arguments
    #[error("Invalid argument: {argument}")]
    #[diagnostic(code(braise::cli::invalid_argument), help("{suggestion}"))]
    InvalidArgument {
        /// The invalid argument
        argument: String,
        /// Suggestion for fixing the argument
        suggestion: String,
    },

    /// File format not supported
    #[error("Unsupported file format: {format}")]
    #[diagnostic(
        code(braise::cli::unsupported_format),
        help("Supported formats: .braise")
    )]
    UnsupportedFormat {
        /// The unsupported format
        format: String,
    },
}

impl CliError {
    pub fn boxed(self) -> Box<Self> {
        Box::new(self)
    }
    /// Create a read recipe error
    pub fn read_recipe_error(
        src: impl Into<Box<dyn std::error::Error + Send + Sync>>,
        file: impl Into<String>,
    ) -> Self {
        Self::ReadRecipeError {
            src: src.into(),
            file: file.into(),
        }
    }

    /// Create an invalid argument error
    pub fn invalid_argument(argument: impl Into<String>, suggestion: impl Into<String>) -> Self {
        Self::InvalidArgument {
            argument: argument.into(),
            suggestion: suggestion.into(),
        }
    }

    /// Create an unsupported format error
    pub fn unsupported_format(format: impl Into<String>) -> Self {
        Self::UnsupportedFormat {
            format: format.into(),
        }
    }

    /// Create a no task error
    pub fn no_task() -> Self {
        Self::NoTask
    }

    /// Get the severity of this CLI error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            CliError::NoTask => ErrorSeverity::Warning,
            CliError::NoRecipeFileFound => ErrorSeverity::Error,
            CliError::ReadRecipeError { .. } => ErrorSeverity::Error,
            CliError::InvalidArgument { .. } => ErrorSeverity::Error,
            CliError::UnsupportedFormat { .. } => ErrorSeverity::Error,
        }
    }
}

// Default files to search for (will be imported from constants)
const DEFAULT_FILES: &[&str] = &["Braisefile", "braise.toml", "braise.yaml"];

fn format_file_list(files: &[&str]) -> String {
    files
        .iter()
        .map(|f| f.bold().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_error_creation() {
        let error = CliError::invalid_argument("--invalid", "Use --help to see available options");

        match error {
            CliError::InvalidArgument {
                argument,
                suggestion,
            } => {
                assert_eq!(argument, "--invalid");
                assert_eq!(suggestion, "Use --help to see available options");
            }
            _ => panic!("Expected InvalidArgument"),
        }
    }

    #[test]
    fn test_cli_error_severity() {
        assert_eq!(CliError::NoTask.severity(), ErrorSeverity::Warning);
        assert_eq!(CliError::NoRecipeFileFound.severity(), ErrorSeverity::Error);
    }

    #[test]
    fn test_read_recipe_error() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let error = CliError::read_recipe_error(io_error, "missing.braise");

        match error {
            CliError::ReadRecipeError { file, .. } => {
                assert_eq!(file, "missing.braise");
            }
            _ => panic!("Expected ReadRecipeError"),
        }
    }
}

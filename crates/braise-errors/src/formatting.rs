use crate::core::{BraiseError, ErrorSeverity};
use owo_colors::{OwoColorize, Style};

/// Configuration for error formatting
#[derive(Debug, Clone)]
pub struct FormattingConfig {
    /// Whether to use colors in output
    pub use_colors: bool,
    /// Whether to show error codes
    pub show_codes: bool,
    /// Whether to show source code context
    pub show_source: bool,
    /// Number of context lines to show around errors
    pub context_lines: usize,
    /// Maximum width for error messages
    pub max_width: usize,
    /// Style for different severity levels
    pub severity_styles: SeverityStyles,
}

/// Styling configuration for different error severities
#[derive(Debug, Clone)]
pub struct SeverityStyles {
    pub critical: Style,
    pub error: Style,
    pub warning: Style,
    pub info: Style,
}

impl Default for FormattingConfig {
    fn default() -> Self {
        Self {
            use_colors: true,
            show_codes: true,
            show_source: true,
            context_lines: 2,
            max_width: 120,
            severity_styles: SeverityStyles::default(),
        }
    }
}

impl Default for SeverityStyles {
    fn default() -> Self {
        Self {
            critical: Style::new().red().bold(),
            error: Style::new().red(),
            warning: Style::new().yellow(),
            info: Style::new().blue(),
        }
    }
}

/// Enhanced error formatter that provides rich error display
pub struct ErrorFormatter {
    config: FormattingConfig,
}

impl ErrorFormatter {
    /// Create a new error formatter with default configuration
    pub fn new() -> Self {
        Self {
            config: FormattingConfig::default(),
        }
    }

    /// Create an error formatter with custom configuration
    pub fn with_config(config: FormattingConfig) -> Self {
        Self { config }
    }

    /// Format a BraiseError for display
    pub fn format_error(&self, error: &BraiseError) -> String {
        let severity = error.severity();
        let style = self.config.severity_styles.style_for_severity(severity);

        let mut output = String::new();

        // Error header with severity and code
        let header = self.format_error_header(error, severity, style);
        output.push_str(&header);
        output.push('\n');

        // Main error message
        let message = self.format_error_message(error, style);
        output.push_str(&message);
        output.push('\n');

        // Source context if available
        if self.config.show_source
            && let Some(source_context) = self.format_source_context(error)
        {
            output.push('\n');
            output.push_str(&source_context);
            output.push('\n');
        }

        // Help text if available
        if let Some(help) = self.format_help_text(error) {
            output.push('\n');
            output.push_str(&help);
        }

        output
    }

    /// Format the error header (severity + code)
    fn format_error_header(
        &self,
        error: &BraiseError,
        severity: ErrorSeverity,
        style: Style,
    ) -> String {
        let severity_text = severity.display_name();
        let prefix = if self.config.use_colors {
            format!("{}", style.style(severity_text))
        } else {
            severity_text.to_uppercase()
        };

        if self.config.show_codes {
            if let Some(code) = error.error_code() {
                format!("{prefix} [{code}]")
            } else {
                prefix
            }
        } else {
            prefix
        }
    }

    /// Format the main error message
    fn format_error_message(&self, error: &BraiseError, style: Style) -> String {
        let message = error.to_string();

        if self.config.use_colors {
            format!("{}", style.style(&message))
        } else {
            message
        }
    }

    /// Format source code context if available
    fn format_source_context(&self, error: &BraiseError) -> Option<String> {
        // This would extract source context from the error
        // For now, return None as we'd need to implement source extraction
        // from each error type's diagnostic information
        let _ = error;
        None
    }

    /// Format help text
    fn format_help_text(&self, error: &BraiseError) -> Option<String> {
        let help = error.help_text()?;

        let help_prefix = if self.config.use_colors {
            "help".blue().to_string()
        } else {
            "HELP".to_string()
        };

        Some(format!("{help_prefix}: {help}"))
    }

    /// Format a list of errors
    pub fn format_errors(&self, errors: &[BraiseError]) -> String {
        if errors.is_empty() {
            return String::new();
        }

        if errors.len() == 1 {
            return self.format_error(&errors[0]);
        }

        let mut output = String::new();

        // Header for multiple errors
        let header = if self.config.use_colors {
            format!("{} errors found:", errors.len().to_string().red().bold())
        } else {
            format!("{} ERRORS FOUND:", errors.len())
        };

        output.push_str(&header);
        output.push('\n');
        output.push('\n');

        for (i, error) in errors.iter().enumerate() {
            let error_num = if self.config.use_colors {
                format!("Error {}:", (i + 1).to_string().bold())
            } else {
                format!("ERROR {}:", i + 1)
            };

            output.push_str(&error_num);
            output.push('\n');
            output.push_str(&self.format_error(error));

            if i < errors.len() - 1 {
                output.push('\n');
                output.push_str(&"-".repeat(40));
                output.push('\n');
                output.push('\n');
            }
        }

        output
    }

    /// Format an error summary (for quick display)
    pub fn format_error_summary(&self, error: &BraiseError) -> String {
        let severity = error.severity();
        let style = self.config.severity_styles.style_for_severity(severity);

        let severity_text = severity.display_name();
        let message = error.to_string();

        if self.config.use_colors {
            format!("{}: {}", style.style(severity_text), message)
        } else {
            format!("{}: {}", severity_text.to_uppercase(), message)
        }
    }

    /// Create a compact, single-line error representation
    pub fn format_compact(&self, error: &BraiseError) -> String {
        let severity = error.severity();
        let icon = severity.icon();
        let message = error.to_string();

        // Truncate message if too long
        let truncated_message = if message.len() > 80 {
            format!("{}...", &message[..77])
        } else {
            message
        };

        if self.config.use_colors {
            let style = self.config.severity_styles.style_for_severity(severity);
            format!("{} {}", style.style(icon), truncated_message)
        } else {
            format!("{icon} {truncated_message}")
        }
    }
}

impl Default for ErrorFormatter {
    fn default() -> Self {
        Self::new()
    }
}

impl SeverityStyles {
    /// Get the style for a given severity level
    pub fn style_for_severity(&self, severity: ErrorSeverity) -> Style {
        match severity {
            ErrorSeverity::Critical => self.critical,
            ErrorSeverity::Error => self.error,
            ErrorSeverity::Warning => self.warning,
            ErrorSeverity::Info => self.info,
        }
    }
}

impl ErrorSeverity {
    /// Get the display name for this severity
    pub fn display_name(self) -> &'static str {
        match self {
            ErrorSeverity::Critical => "critical",
            ErrorSeverity::Error => "error",
            ErrorSeverity::Warning => "warning",
            ErrorSeverity::Info => "info",
        }
    }

    /// Get an icon/symbol for this severity
    pub fn icon(self) -> &'static str {
        match self {
            ErrorSeverity::Critical => "🚨",
            ErrorSeverity::Error => "❌",
            ErrorSeverity::Warning => "⚠️",
            ErrorSeverity::Info => "ℹ️",
        }
    }

    /// Get a plain text symbol (for non-unicode terminals)
    pub fn symbol(self) -> &'static str {
        match self {
            ErrorSeverity::Critical => "!!",
            ErrorSeverity::Error => "x",
            ErrorSeverity::Warning => "!",
            ErrorSeverity::Info => "i",
        }
    }
}

impl BraiseError {
    /// Get an error code for this error (if available)
    pub fn error_code(&self) -> Option<String> {
        match self {
            BraiseError::Cli(_) => Some("braise::cli".to_string()),
            BraiseError::Parser(_) => Some("braise::parser".to_string()),
            BraiseError::Runtime(_) => Some("braise::runtime".to_string()),
            BraiseError::Type(_) => Some("braise::types".to_string()),
            BraiseError::Lexer { .. } => Some("braise::lexer".to_string()),
            BraiseError::Lsp(_) => Some("braise::lsp".to_string()),
        }
    }

    /// Get help text for this error (if available)
    pub fn help_text(&self) -> Option<String> {
        match self {
            BraiseError::Cli(cli_error) => match cli_error {
                crate::domains::CliError::NoTask => Some(
                    "Specify a recipe name to run, or use --list to see available recipes"
                        .to_string(),
                ),
                crate::domains::CliError::InvalidArgument { .. } => {
                    Some("Check the command line usage with --help".to_string())
                }
                _ => None,
            },
            BraiseError::Parser(_) => Some("Check the syntax of your braise file".to_string()),
            BraiseError::Runtime(_) => {
                Some("This error occurred during recipe execution".to_string())
            }
            BraiseError::Type(_) => {
                Some("Check the types of your variables and expressions".to_string())
            }
            BraiseError::Lexer { .. } => {
                Some("Check for invalid characters or token sequences".to_string())
            }
            BraiseError::Lsp(_) => Some("This is an LSP protocol error".to_string()),
        }
    }
}

/// Utility functions for text formatting
pub mod text_utils {
    /// Wrap text to a specified width
    pub fn wrap_text(text: &str, width: usize) -> Vec<String> {
        if text.len() <= width {
            return vec![text.to_string()];
        }

        let mut lines = Vec::new();
        let mut current_line = String::new();

        for word in text.split_whitespace() {
            if current_line.is_empty() {
                current_line = word.to_string();
            } else if current_line.len() + word.len() < width {
                current_line.push(' ');
                current_line.push_str(word);
            } else {
                lines.push(current_line);
                current_line = word.to_string();
            }
        }

        if !current_line.is_empty() {
            lines.push(current_line);
        }

        lines
    }

    /// Indent text by a specified number of spaces
    pub fn indent_text(text: &str, spaces: usize) -> String {
        let indent = " ".repeat(spaces);
        text.lines()
            .map(|line| format!("{indent}{line}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Truncate text to a maximum length with ellipsis
    pub fn truncate_text(text: &str, max_len: usize) -> String {
        if text.len() <= max_len {
            text.to_string()
        } else if max_len <= 3 {
            "...".to_string()
        } else {
            format!("{}...", &text[..max_len - 3])
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domains::*;

    #[test]
    fn test_error_formatter_basic() {
        let formatter = ErrorFormatter::new();
        let error = BraiseError::Cli(CliError::no_task());

        let formatted = formatter.format_error(&error);
        assert!(!formatted.is_empty());
        assert!(formatted.contains("error"));
    }

    #[test]
    fn test_error_formatter_without_colors() {
        let mut config = FormattingConfig::default();
        config.use_colors = false;

        let formatter = ErrorFormatter::with_config(config);
        let error = BraiseError::Runtime(RuntimeError::undefined_variable("test"));

        let formatted = formatter.format_error(&error);
        assert!(!formatted.is_empty());
        // Should not contain ANSI color codes
        assert!(!formatted.contains("\x1b["));
    }

    #[test]
    fn test_multiple_errors_formatting() {
        let formatter = ErrorFormatter::new();
        let errors = vec![
            BraiseError::Cli(CliError::no_task()),
            BraiseError::Runtime(RuntimeError::undefined_variable("x")),
        ];

        let formatted = formatter.format_errors(&errors);
        assert!(formatted.contains("2 errors") || formatted.contains("2"));
        assert!(formatted.contains("Error") && formatted.contains("1"));
        assert!(formatted.contains("Error") && formatted.contains("2"));
    }

    #[test]
    fn test_error_summary() {
        let formatter = ErrorFormatter::new();
        let error = BraiseError::Type(TypeError::mismatch("string", "number", "test"));

        let summary = formatter.format_error_summary(&error);
        assert!(!summary.is_empty());
        assert!(summary.contains("Type") || summary.contains("error"));
    }

    #[test]
    fn test_compact_formatting() {
        let formatter = ErrorFormatter::new();
        let error = BraiseError::Runtime(RuntimeError::other(
            "A very long error message that should be truncated because it exceeds the maximum length for compact display",
        ));

        let compact = formatter.format_compact(&error);
        assert!(compact.len() < 100); // Should be truncated
        assert!(compact.contains("..."));
    }

    #[test]
    fn test_severity_display_names() {
        assert_eq!(ErrorSeverity::Critical.display_name(), "critical");
        assert_eq!(ErrorSeverity::Error.display_name(), "error");
        assert_eq!(ErrorSeverity::Warning.display_name(), "warning");
        assert_eq!(ErrorSeverity::Info.display_name(), "info");
    }

    #[test]
    fn test_text_utils() {
        use text_utils::*;

        // Test text wrapping
        let wrapped = wrap_text("This is a long line that should be wrapped", 10);
        assert!(wrapped.len() > 1);
        assert!(wrapped.iter().all(|line| line.len() <= 10));

        // Test text indentation
        let indented = indent_text("line1\nline2", 4);
        assert!(indented.contains("    line1"));
        assert!(indented.contains("    line2"));

        // Test text truncation
        let truncated = truncate_text("This is a long string", 10);
        assert_eq!(truncated, "This is...");
        assert!(truncated.len() <= 10);
    }
}

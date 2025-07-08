use miette::SourceSpan;
use owo_colors::OwoColorize;

/// Enhanced diagnostic information for error reporting
#[derive(Debug, Clone)]
pub struct DiagnosticInfo {
    /// Error code for categorization
    pub code: Option<String>,
    /// Human-readable help message
    pub help: Option<String>,
    /// Source code context
    pub source_code: Option<String>,
    /// Location spans with labels
    pub labels: Vec<LabeledSpan>,
    /// Related diagnostic information
    pub related: Vec<DiagnosticInfo>,
}

/// A labeled span for highlighting specific code sections
#[derive(Debug, Clone)]
pub struct LabeledSpan {
    /// The span in the source code
    pub span: SourceSpan,
    /// Label text to display
    pub label: String,
    /// Style/color for the label
    pub style: LabelStyle,
}

/// Style options for diagnostic labels
#[derive(Debug, Clone, Copy)]
pub enum LabelStyle {
    /// Primary error location (red)
    Primary,
    /// Secondary information (blue)
    Secondary,
    /// Warning location (yellow)
    Warning,
    /// Information location (green)
    Info,
}

impl DiagnosticInfo {
    /// Create a new diagnostic with basic information
    pub fn new() -> Self {
        Self {
            code: None,
            help: None,
            source_code: None,
            labels: Vec::new(),
            related: Vec::new(),
        }
    }

    /// Set the error code
    pub fn with_code(mut self, code: impl Into<String>) -> Self {
        self.code = Some(code.into());
        self
    }

    /// Set the help message
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Set the source code context
    pub fn with_source_code(mut self, source_code: impl Into<String>) -> Self {
        self.source_code = Some(source_code.into());
        self
    }

    /// Add a labeled span
    pub fn with_label(mut self, span: SourceSpan, label: impl Into<String>, style: LabelStyle) -> Self {
        self.labels.push(LabeledSpan {
            span,
            label: label.into(),
            style,
        });
        self
    }

    /// Add related diagnostic information
    pub fn with_related(mut self, related: DiagnosticInfo) -> Self {
        self.related.push(related);
        self
    }
}

impl Default for DiagnosticInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl LabelStyle {
    /// Get the color for this label style
    pub fn color(self) -> impl Fn(&str) -> String {
        match self {
            LabelStyle::Primary => |s: &str| s.red().to_string(),
            LabelStyle::Secondary => |s: &str| s.blue().to_string(),
            LabelStyle::Warning => |s: &str| s.yellow().to_string(),
            LabelStyle::Info => |s: &str| s.green().to_string(),
        }
    }

    /// Get the prefix symbol for this label style
    pub fn symbol(self) -> &'static str {
        match self {
            LabelStyle::Primary => "×",
            LabelStyle::Secondary => "→",
            LabelStyle::Warning => "⚠",
            LabelStyle::Info => "ℹ",
        }
    }
}

/// Helper trait for converting errors to diagnostic information
pub trait ToDiagnostic {
    /// Convert this error to diagnostic information
    fn to_diagnostic(&self) -> DiagnosticInfo;
}

/// Helper for creating common diagnostic patterns
pub struct DiagnosticBuilder {
    info: DiagnosticInfo,
}

impl DiagnosticBuilder {
    /// Create a new diagnostic builder
    pub fn new() -> Self {
        Self {
            info: DiagnosticInfo::new(),
        }
    }

    /// Set error code
    pub fn code(mut self, code: impl Into<String>) -> Self {
        self.info.code = Some(code.into());
        self
    }

    /// Set help message
    pub fn help(mut self, help: impl Into<String>) -> Self {
        self.info.help = Some(help.into());
        self
    }

    /// Set source code
    pub fn source_code(mut self, source: impl Into<String>) -> Self {
        self.info.source_code = Some(source.into());
        self
    }

    /// Add primary error location
    pub fn primary_label(mut self, span: SourceSpan, label: impl Into<String>) -> Self {
        self.info.labels.push(LabeledSpan {
            span,
            label: label.into(),
            style: LabelStyle::Primary,
        });
        self
    }

    /// Add secondary information
    pub fn secondary_label(mut self, span: SourceSpan, label: impl Into<String>) -> Self {
        self.info.labels.push(LabeledSpan {
            span,
            label: label.into(),
            style: LabelStyle::Secondary,
        });
        self
    }

    /// Add warning annotation
    pub fn warning_label(mut self, span: SourceSpan, label: impl Into<String>) -> Self {
        self.info.labels.push(LabeledSpan {
            span,
            label: label.into(),
            style: LabelStyle::Warning,
        });
        self
    }

    /// Add info annotation
    pub fn info_label(mut self, span: SourceSpan, label: impl Into<String>) -> Self {
        self.info.labels.push(LabeledSpan {
            span,
            label: label.into(),
            style: LabelStyle::Info,
        });
        self
    }

    /// Build the diagnostic info
    pub fn build(self) -> DiagnosticInfo {
        self.info
    }
}

impl Default for DiagnosticBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_builder() {
        let diagnostic = DiagnosticBuilder::new()
            .code("braise::test::error")
            .help("This is a test error")
            .source_code("let x = 42")
            .primary_label((4, 1).into(), "variable declaration")
            .build();

        assert_eq!(diagnostic.code, Some("braise::test::error".to_string()));
        assert_eq!(diagnostic.help, Some("This is a test error".to_string()));
        assert_eq!(diagnostic.source_code, Some("let x = 42".to_string()));
        assert_eq!(diagnostic.labels.len(), 1);
        assert_eq!(diagnostic.labels[0].label, "variable declaration");
    }

    #[test]
    fn test_label_style_colors() {
        // Test that color functions work without panicking
        let _ = LabelStyle::Primary.color()("test");
        let _ = LabelStyle::Secondary.color()("test");
        let _ = LabelStyle::Warning.color()("test");
        let _ = LabelStyle::Info.color()("test");
    }

    #[test]
    fn test_label_style_symbols() {
        assert_eq!(LabelStyle::Primary.symbol(), "×");
        assert_eq!(LabelStyle::Secondary.symbol(), "→");
        assert_eq!(LabelStyle::Warning.symbol(), "⚠");
        assert_eq!(LabelStyle::Info.symbol(), "ℹ");
    }
}
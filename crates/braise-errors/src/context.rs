use crate::core::BraiseError;
use miette::SourceSpan;
use std::collections::VecDeque;

/// Context information for error reporting
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// Source file path
    pub file_path: Option<String>,
    /// Source code content
    pub source_code: Option<String>,
    /// Line and column information
    pub location: Option<Location>,
    /// Stack of context breadcrumbs
    pub breadcrumbs: VecDeque<Breadcrumb>,
    /// Additional metadata
    pub metadata: std::collections::HashMap<String, String>,
}

/// Location information within source code
#[derive(Debug, Clone, Copy)]
pub struct Location {
    /// Line number (0-based)
    pub line: usize,
    /// Column number (0-based)
    pub column: usize,
    /// Byte offset in source
    pub offset: usize,
    /// Length of the span
    pub length: usize,
}

/// Breadcrumb for tracking execution context
#[derive(Debug, Clone)]
pub struct Breadcrumb {
    /// Type of context (recipe, function, etc.)
    pub context_type: ContextType,
    /// Name or identifier for this context
    pub name: String,
    /// Optional location where this context was entered
    pub location: Option<Location>,
    /// Timestamp when this context was entered
    pub timestamp: std::time::Instant,
}

/// Types of execution contexts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextType {
    /// Inside a recipe definition
    Recipe,
    /// Inside a function call
    Function,
    /// Inside a conditional block
    Conditional,
    /// Inside a loop
    Loop,
    /// Inside a match expression
    Match,
    /// Inside a shell command
    Shell,
    /// Top-level file parsing
    File,
    /// Inside a module
    Module,
}

impl ErrorContext {
    /// Create a new empty error context
    pub fn new() -> Self {
        Self {
            file_path: None,
            source_code: None,
            location: None,
            breadcrumbs: VecDeque::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Create an error context for a specific file
    pub fn for_file(file_path: impl Into<String>, source_code: impl Into<String>) -> Self {
        Self {
            file_path: Some(file_path.into()),
            source_code: Some(source_code.into()),
            location: None,
            breadcrumbs: VecDeque::new(),
            metadata: std::collections::HashMap::new(),
        }
    }

    /// Set the current location
    pub fn with_location(mut self, location: Location) -> Self {
        self.location = Some(location);
        self
    }

    /// Add a breadcrumb to the context stack
    pub fn push_context(&mut self, context_type: ContextType, name: impl Into<String>) {
        self.breadcrumbs.push_back(Breadcrumb {
            context_type,
            name: name.into(),
            location: self.location,
            timestamp: std::time::Instant::now(),
        });
    }

    /// Remove the most recent context from the stack
    pub fn pop_context(&mut self) -> Option<Breadcrumb> {
        self.breadcrumbs.pop_back()
    }

    /// Get the current context (most recent breadcrumb)
    pub fn current_context(&self) -> Option<&Breadcrumb> {
        self.breadcrumbs.back()
    }

    /// Add metadata to the context
    pub fn add_metadata(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.metadata.insert(key.into(), value.into());
    }

    /// Get metadata value
    pub fn get_metadata(&self, key: &str) -> Option<&str> {
        self.metadata.get(key).map(|s| s.as_str())
    }

    /// Convert a byte offset to line/column information
    pub fn offset_to_location(&self, offset: usize) -> Option<Location> {
        let source = self.source_code.as_ref()?;
        if offset > source.len() {
            return None;
        }

        let mut line = 0;
        let mut column = 0;
        let mut current_offset = 0;

        for ch in source.chars() {
            if current_offset >= offset {
                break;
            }

            if ch == '\n' {
                line += 1;
                column = 0;
            } else {
                column += 1;
            }

            current_offset += ch.len_utf8();
        }

        Some(Location {
            line,
            column,
            offset,
            length: 1,
        })
    }

    /// Convert a SourceSpan to a Location
    pub fn span_to_location(&self, span: SourceSpan) -> Option<Location> {
        let mut location = self.offset_to_location(span.offset())?;
        location.length = span.len();
        Some(location)
    }

    /// Get a formatted context stack for error display
    pub fn format_stack(&self) -> String {
        if self.breadcrumbs.is_empty() {
            return "at top level".to_string();
        }

        let stack_items: Vec<String> = self
            .breadcrumbs
            .iter()
            .map(|breadcrumb| {
                let context_name = match breadcrumb.context_type {
                    ContextType::Recipe => "recipe",
                    ContextType::Function => "function",
                    ContextType::Conditional => "if",
                    ContextType::Loop => "loop",
                    ContextType::Match => "match",
                    ContextType::Shell => "shell",
                    ContextType::File => "file",
                    ContextType::Module => "module",
                };
                format!("{} '{}'", context_name, breadcrumb.name)
            })
            .collect();

        format!("in {}", stack_items.join(" → "))
    }

    /// Get the deepest context name (for error messages)
    pub fn current_context_name(&self) -> String {
        self.breadcrumbs
            .back()
            .map(|b| b.name.clone())
            .unwrap_or_else(|| "top level".to_string())
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new()
    }
}

impl Location {
    /// Create a new location
    pub fn new(line: usize, column: usize, offset: usize, length: usize) -> Self {
        Self {
            line,
            column,
            offset,
            length,
        }
    }

    /// Convert to SourceSpan for miette
    pub fn to_source_span(self) -> SourceSpan {
        SourceSpan::new(self.offset.into(), self.length.into())
    }

    /// Format as human-readable location
    pub fn format(&self) -> String {
        format!("{}:{}", self.line + 1, self.column + 1)
    }
}

impl From<Location> for SourceSpan {
    fn from(location: Location) -> Self {
        location.to_source_span()
    }
}

impl ContextType {
    /// Get the display name for this context type
    pub fn display_name(self) -> &'static str {
        match self {
            ContextType::Recipe => "recipe",
            ContextType::Function => "function",
            ContextType::Conditional => "conditional",
            ContextType::Loop => "loop",
            ContextType::Match => "match",
            ContextType::Shell => "shell",
            ContextType::File => "file",
            ContextType::Module => "module",
        }
    }

    /// Check if this context type represents a callable unit
    pub fn is_callable(self) -> bool {
        matches!(self, ContextType::Recipe | ContextType::Function)
    }

    /// Check if this context type represents a control flow construct
    pub fn is_control_flow(self) -> bool {
        matches!(
            self,
            ContextType::Conditional | ContextType::Loop | ContextType::Match
        )
    }
}

/// Extension trait for adding context to errors
pub trait WithContext<T> {
    /// Add context information to this result
    fn with_context(self, context: ErrorContext) -> Result<T, BraiseError>;

    /// Add context information with a closure
    fn with_context_fn<F>(self, f: F) -> Result<T, BraiseError>
    where
        F: FnOnce() -> ErrorContext;
}

impl<T, E> WithContext<T> for Result<T, E>
where
    E: Into<BraiseError>,
{
    fn with_context(self, _context: ErrorContext) -> Result<T, BraiseError> {
        self.map_err(|e| e.into())
    }

    fn with_context_fn<F>(self, _f: F) -> Result<T, BraiseError>
    where
        F: FnOnce() -> ErrorContext,
    {
        self.map_err(|e| e.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_context_creation() {
        let context = ErrorContext::new();
        assert!(context.file_path.is_none());
        assert!(context.source_code.is_none());
        assert!(context.breadcrumbs.is_empty());
    }

    #[test]
    fn test_error_context_with_file() {
        let context = ErrorContext::for_file("test.braise", "let x = 42");
        assert_eq!(context.file_path, Some("test.braise".to_string()));
        assert_eq!(context.source_code, Some("let x = 42".to_string()));
    }

    #[test]
    fn test_breadcrumb_management() {
        let mut context = ErrorContext::new();

        context.push_context(ContextType::Recipe, "test_recipe");
        assert_eq!(context.breadcrumbs.len(), 1);
        assert_eq!(context.current_context().unwrap().name, "test_recipe");

        context.push_context(ContextType::Function, "test_function");
        assert_eq!(context.breadcrumbs.len(), 2);
        assert_eq!(context.current_context().unwrap().name, "test_function");

        let popped = context.pop_context();
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().name, "test_function");
        assert_eq!(context.breadcrumbs.len(), 1);
    }

    #[test]
    fn test_offset_to_location() {
        let context = ErrorContext::for_file("test.braise", "hello\nworld\ntest");

        // Test start of file
        let loc = context.offset_to_location(0).unwrap();
        assert_eq!(loc.line, 0);
        assert_eq!(loc.column, 0);

        // Test after first newline
        let loc = context.offset_to_location(6).unwrap();
        assert_eq!(loc.line, 1);
        assert_eq!(loc.column, 0);

        // Test middle of second line
        let loc = context.offset_to_location(8).unwrap();
        assert_eq!(loc.line, 1);
        assert_eq!(loc.column, 2);
    }

    #[test]
    fn test_context_stack_formatting() {
        let mut context = ErrorContext::new();

        assert_eq!(context.format_stack(), "at top level");

        context.push_context(ContextType::Recipe, "main");
        context.push_context(ContextType::Function, "build");

        let formatted = context.format_stack();
        assert!(formatted.contains("recipe 'main'"));
        assert!(formatted.contains("function 'build'"));
        assert!(formatted.contains("→"));
    }

    #[test]
    fn test_context_type_properties() {
        assert!(ContextType::Recipe.is_callable());
        assert!(ContextType::Function.is_callable());
        assert!(!ContextType::Loop.is_callable());

        assert!(ContextType::Conditional.is_control_flow());
        assert!(ContextType::Loop.is_control_flow());
        assert!(ContextType::Match.is_control_flow());
        assert!(!ContextType::Recipe.is_control_flow());
    }

    #[test]
    fn test_location_formatting() {
        let location = Location::new(5, 10, 100, 5);
        assert_eq!(location.format(), "6:11"); // 1-based display
    }
}

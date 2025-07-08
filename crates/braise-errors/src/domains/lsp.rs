use crate::core::ErrorSeverity;
use miette::Diagnostic;
use thiserror::Error;

/// Language Server Protocol errors
#[derive(Debug, Error, Diagnostic)]
pub enum LspError {
    /// Failed to initialize the language server
    #[error("Failed to initialize LSP server: {reason}")]
    #[diagnostic(
        code(braise::lsp::initialization_failed),
        help("Check LSP client configuration and server capabilities")
    )]
    InitializationFailed {
        /// Reason for initialization failure
        reason: String,
    },

    /// Invalid LSP request or notification
    #[error("Invalid LSP request: {method}")]
    #[diagnostic(
        code(braise::lsp::invalid_request),
        help("Check that the request follows LSP specification")
    )]
    InvalidRequest {
        /// LSP method name
        method: String,
        /// Error details
        details: String,
    },

    /// Document synchronization error
    #[error("Document sync error: {message}")]
    #[diagnostic(
        code(braise::lsp::document_sync_error),
        help("Ensure document changes are properly synchronized")
    )]
    DocumentSyncError {
        /// Error message
        message: String,
        /// Document URI
        uri: Option<String>,
    },

    /// Workspace operation failure
    #[error("Workspace operation failed: {operation}")]
    #[diagnostic(
        code(braise::lsp::workspace_operation_failed),
        help("Check workspace configuration and file permissions")
    )]
    WorkspaceOperationFailed {
        /// Operation that failed
        operation: String,
        /// Error details
        details: String,
    },

    /// Code action provider error
    #[error("Code action error: {message}")]
    #[diagnostic(
        code(braise::lsp::code_action_error),
        help("Check code action provider implementation")
    )]
    CodeActionError {
        /// Error message
        message: String,
        /// Document URI
        uri: Option<String>,
        /// Range where error occurred
        range: Option<String>,
    },

    /// Completion provider error
    #[error("Completion error: {message}")]
    #[diagnostic(
        code(braise::lsp::completion_error),
        help("Check completion provider implementation")
    )]
    CompletionError {
        /// Error message
        message: String,
        /// Document URI
        uri: Option<String>,
        /// Position where completion was requested
        position: Option<String>,
    },

    /// Hover provider error
    #[error("Hover error: {message}")]
    #[diagnostic(
        code(braise::lsp::hover_error),
        help("Check hover provider implementation")
    )]
    HoverError {
        /// Error message
        message: String,
        /// Document URI
        uri: Option<String>,
        /// Position where hover was requested
        position: Option<String>,
    },

    /// Diagnostic provider error
    #[error("Diagnostic error: {message}")]
    #[diagnostic(
        code(braise::lsp::diagnostic_error),
        help("Check diagnostic provider implementation")
    )]
    DiagnosticError {
        /// Error message
        message: String,
        /// Document URI
        uri: Option<String>,
    },

    /// Symbol provider error
    #[error("Symbol provider error: {message}")]
    #[diagnostic(
        code(braise::lsp::symbol_error),
        help("Check symbol provider implementation")
    )]
    SymbolError {
        /// Error message
        message: String,
        /// Document URI
        uri: Option<String>,
        /// Symbol query
        query: Option<String>,
    },

    /// Server transport error
    #[error("Transport error: {message}")]
    #[diagnostic(
        code(braise::lsp::transport_error),
        help("Check network connectivity and server status")
    )]
    TransportError {
        /// Transport error message
        message: String,
        /// Connection details
        details: Option<String>,
    },
}

impl LspError {
    /// Create an initialization failed error
    pub fn initialization_failed(reason: impl Into<String>) -> Self {
        Self::InitializationFailed {
            reason: reason.into(),
        }
    }

    /// Create an invalid request error
    pub fn invalid_request(method: impl Into<String>, details: impl Into<String>) -> Self {
        Self::InvalidRequest {
            method: method.into(),
            details: details.into(),
        }
    }

    /// Create a document sync error
    pub fn document_sync_error(message: impl Into<String>) -> Self {
        Self::DocumentSyncError {
            message: message.into(),
            uri: None,
        }
    }

    /// Create a document sync error with URI
    pub fn document_sync_error_with_uri(
        message: impl Into<String>,
        uri: impl Into<String>,
    ) -> Self {
        Self::DocumentSyncError {
            message: message.into(),
            uri: Some(uri.into()),
        }
    }

    /// Create a workspace operation failed error
    pub fn workspace_operation_failed(
        operation: impl Into<String>,
        details: impl Into<String>,
    ) -> Self {
        Self::WorkspaceOperationFailed {
            operation: operation.into(),
            details: details.into(),
        }
    }

    /// Create a code action error
    pub fn code_action_error(message: impl Into<String>) -> Self {
        Self::CodeActionError {
            message: message.into(),
            uri: None,
            range: None,
        }
    }

    /// Create a completion error
    pub fn completion_error(message: impl Into<String>) -> Self {
        Self::CompletionError {
            message: message.into(),
            uri: None,
            position: None,
        }
    }

    /// Create a hover error
    pub fn hover_error(message: impl Into<String>) -> Self {
        Self::HoverError {
            message: message.into(),
            uri: None,
            position: None,
        }
    }

    /// Create a diagnostic error
    pub fn diagnostic_error(message: impl Into<String>) -> Self {
        Self::DiagnosticError {
            message: message.into(),
            uri: None,
        }
    }

    /// Create a symbol error
    pub fn symbol_error(message: impl Into<String>) -> Self {
        Self::SymbolError {
            message: message.into(),
            uri: None,
            query: None,
        }
    }

    /// Create a transport error
    pub fn transport_error(message: impl Into<String>) -> Self {
        Self::TransportError {
            message: message.into(),
            details: None,
        }
    }

    /// Get the severity of this LSP error
    pub fn severity(&self) -> ErrorSeverity {
        match self {
            LspError::InitializationFailed { .. } => ErrorSeverity::Critical,
            LspError::InvalidRequest { .. } => ErrorSeverity::Warning,
            LspError::DocumentSyncError { .. } => ErrorSeverity::Warning,
            LspError::WorkspaceOperationFailed { .. } => ErrorSeverity::Error,
            LspError::CodeActionError { .. } => ErrorSeverity::Warning,
            LspError::CompletionError { .. } => ErrorSeverity::Warning,
            LspError::HoverError { .. } => ErrorSeverity::Warning,
            LspError::DiagnosticError { .. } => ErrorSeverity::Warning,
            LspError::SymbolError { .. } => ErrorSeverity::Warning,
            LspError::TransportError { .. } => ErrorSeverity::Error,
        }
    }

    /// Check if this error should cause the server to shutdown
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            LspError::InitializationFailed { .. } | LspError::TransportError { .. }
        )
    }

    /// Get the LSP method associated with this error, if any
    pub fn method(&self) -> Option<&str> {
        match self {
            LspError::InvalidRequest { method, .. } => Some(method),
            LspError::CodeActionError { .. } => Some("textDocument/codeAction"),
            LspError::CompletionError { .. } => Some("textDocument/completion"),
            LspError::HoverError { .. } => Some("textDocument/hover"),
            LspError::DiagnosticError { .. } => Some("textDocument/diagnostic"),
            LspError::SymbolError { .. } => Some("textDocument/documentSymbol"),
            _ => None,
        }
    }

    /// Get the document URI associated with this error, if any
    pub fn uri(&self) -> Option<&str> {
        match self {
            LspError::DocumentSyncError { uri, .. } => uri.as_deref(),
            LspError::CodeActionError { uri, .. } => uri.as_deref(),
            LspError::CompletionError { uri, .. } => uri.as_deref(),
            LspError::HoverError { uri, .. } => uri.as_deref(),
            LspError::DiagnosticError { uri, .. } => uri.as_deref(),
            LspError::SymbolError { uri, .. } => uri.as_deref(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsp_error_creation() {
        let error = LspError::initialization_failed("Server startup failed");
        
        match error {
            LspError::InitializationFailed { reason } => {
                assert_eq!(reason, "Server startup failed");
            }
            _ => panic!("Expected InitializationFailed"),
        }
    }

    #[test]
    fn test_lsp_error_severity() {
        assert_eq!(
            LspError::initialization_failed("test").severity(),
            ErrorSeverity::Critical
        );
        assert_eq!(
            LspError::completion_error("test").severity(),
            ErrorSeverity::Warning
        );
        assert_eq!(
            LspError::transport_error("test").severity(),
            ErrorSeverity::Error
        );
    }

    #[test]
    fn test_lsp_error_fatality() {
        assert!(LspError::initialization_failed("test").is_fatal());
        assert!(LspError::transport_error("test").is_fatal());
        assert!(!LspError::completion_error("test").is_fatal());
    }

    #[test]
    fn test_lsp_error_method_association() {
        let error = LspError::completion_error("test");
        assert_eq!(error.method(), Some("textDocument/completion"));
        
        let error = LspError::invalid_request("custom/method", "details");
        assert_eq!(error.method(), Some("custom/method"));
        
        let error = LspError::initialization_failed("test");
        assert_eq!(error.method(), None);
    }

    #[test]
    fn test_document_sync_error_with_uri() {
        let error = LspError::document_sync_error_with_uri("sync failed", "file:///test.braise");
        assert_eq!(error.uri(), Some("file:///test.braise"));
        
        let error = LspError::document_sync_error("sync failed");
        assert_eq!(error.uri(), None);
    }
}
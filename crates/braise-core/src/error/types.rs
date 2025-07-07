use miette::Diagnostic;
use owo_colors::OwoColorize;

#[derive(Debug, thiserror::Error, Diagnostic, Clone)]
pub enum TypeError {
    #[error("Expected {expected}, got {got} (context: {context})",
        expected = .expected.bold().green(),
        got = .got.bold().red(),
        context = .context.dimmed()
    )]
    #[diagnostic(code(braise::type_error))]
    Mismatch {
        expected: String,
        got: String,
        context: String,
    },

    #[error("Unsupported operation for type: {0}")]
    #[diagnostic(code(braise::unsupported_operation))]
    UnsupportedOperation(String),

    #[error("Cannot convert type: {from} to {to}",
        from = .from.bold().red(),
        to = .to.bold().green()
    )]
    #[diagnostic(code(braise::cannot_convert))]
    CannotConvert { from: String, to: String },
}

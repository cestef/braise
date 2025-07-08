//! Display and output formatting utilities

use tracing_subscriber::{EnvFilter, fmt as tracing_fmt, prelude::*};

/// Initialize tracing subscriber for debug logging
pub fn init_tracing(debug: bool) {
    let filter = if debug {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| {
            EnvFilter::new(
                "braise=debug,braise_runtime=debug,braise_parser=debug,braise_lexer=debug",
            )
        })
    } else {
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("braise=info"))
    };

    tracing_subscriber::registry()
        .with(
            tracing_fmt::layer()
                .with_target(false)
                .with_thread_ids(false)
                .with_file(false)
                .with_line_number(false)
                .compact(),
        )
        .with(filter)
        .init();
}
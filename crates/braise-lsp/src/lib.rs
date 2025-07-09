use miette::Result;

use tower_lsp::{LspService, Server};

mod server;
use server::BraiseLspServer;

mod utils;

pub async fn run() -> Result<()> {
    // Initialize tracing for LSP with stderr output only
    init_lsp_tracing();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(BraiseLspServer::new);

    Server::new(stdin, stdout, socket).serve(service).await;

    Ok(())
}

fn init_lsp_tracing() {
    use tracing_subscriber::{EnvFilter, fmt as tracing_fmt, prelude::*};

    // Only initialize if not already initialized
    if tracing::subscriber::set_global_default(
        tracing_subscriber::registry()
            .with(
                tracing_fmt::layer()
                    .with_writer(std::io::stderr) // Force output to stderr
                    .with_target(false)
                    .with_thread_ids(false)
                    .with_file(false)
                    .with_line_number(false)
                    .compact(),
            )
            .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"))),
    )
    .is_err()
    {
        // Subscriber already set, that's fine
    }
}

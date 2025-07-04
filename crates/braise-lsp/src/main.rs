use miette::Result;

use tower_lsp::{LspService, Server};
use tracing_subscriber::{EnvFilter, fmt};

mod server;
use server::BraiseLspServer;

#[tokio::main]
async fn main() -> Result<()> {
    fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(BraiseLspServer::new);

    Server::new(stdin, stdout, socket).serve(service).await;

    Ok(())
}

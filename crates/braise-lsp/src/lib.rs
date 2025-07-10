use miette::Result;

use tower_lsp::{LspService, Server};

mod server;
use server::BraiseLspServer;

mod utils;

pub async fn run() -> Result<()> {
    init_lsp_tracing();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(BraiseLspServer::new);

    Server::new(stdin, stdout, socket).serve(service).await;

    Ok(())
}

fn init_lsp_tracing() {
    env_logger::Builder::new()
        .filter_level(log::LevelFilter::Info)
        .filter_module("braise_lsp", log::LevelFilter::Debug)
        .target(env_logger::Target::Stderr)
        .format_timestamp(None)
        .format_target(false)
        .format_module_path(false)
        .init();
}

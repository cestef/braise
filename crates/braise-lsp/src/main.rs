#[tokio::main]
async fn main() -> miette::Result<()> {
    braise_lsp::run().await
}

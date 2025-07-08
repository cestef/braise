use braise_errors::CliError;
use clap::Parser;
use core::constants::DEFAULT_FILES;

mod commands;
mod display;
mod utils;

use commands::{Cli, Commands, format_recipe, list_recipes, run_recipe, show_recipe_info};
use display::init_tracing;

#[tokio::main]
async fn main() -> miette::Result<()> {
    miette::set_hook(Box::new(|_| {
        Box::new(miette::MietteHandlerOpts::new().build())
    }))?;

    let cli = Cli::parse();

    init_tracing(cli.debug);

    tracing::debug!(
        "Starting braise CLI with args: {:?}",
        std::env::args().collect::<Vec<_>>()
    );

    let file = if let Some(file) = cli.file {
        file
    } else {
        utils::find_first_existing_file(DEFAULT_FILES).ok_or(CliError::NoRecipeFileFound)?
    };

    let contents =
        std::fs::read_to_string(&file).map_err(|e| CliError::read_recipe_error(e, file.clone()))?;

    match cli.command {
        Some(Commands::List) => list_recipes(&contents, &file)?,
        Some(Commands::Format { stdout }) => format_recipe(&contents, &file, stdout)?,
        Some(Commands::Lsp) => start_lsp().await?,
        Some(Commands::Info { recipe }) => show_recipe_info(&contents, &file, &recipe)?,
        None => {
            if let Some(recipe_name) = cli.recipe {
                run_recipe(&contents, &file, &recipe_name, &cli.args, cli.dry)?;
            } else {
                list_recipes(&contents, &file)?;
            }
        }
    }

    Ok(())
}

async fn start_lsp() -> miette::Result<()> {
    lsp::run().await
}

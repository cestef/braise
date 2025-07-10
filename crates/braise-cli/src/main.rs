use braise_errors::CliError;
use clap::Parser;
use core::constants::DEFAULT_FILES;

mod commands;
mod display;
mod utils;

use commands::{Cli, Commands, format_recipe, list_recipes, run_recipe, show_recipe_info};
use display::init_log;

#[tokio::main]
async fn main() -> miette::Result<()> {
    miette::set_hook(Box::new(|_| {
        Box::new(miette::MietteHandlerOpts::new().build())
    }))?;

    let cli = Cli::parse();

    // Initialize tracing subscriber (but not for LSP to avoid stdout pollution)
    if !matches!(cli.command, Some(Commands::Lsp)) {
        init_log(cli.debug);
        log::debug!(
            "Starting braise CLI with args: {:?}",
            std::env::args().collect::<Vec<_>>()
        );
    }

    match cli.command {
        Some(Commands::Lsp) => start_lsp().await?,
        _ => {
            let file = if let Some(ref file) = cli.file {
                file.clone()
            } else {
                utils::find_first_existing_file(DEFAULT_FILES).ok_or(CliError::NoRecipeFileFound)?
            };

            let contents = std::fs::read_to_string(&file)
                .map_err(|e| CliError::read_recipe_error(e, file.clone()))?;

            match cli.command {
                Some(Commands::List) => list_recipes(&contents, &file).map_err(|e| *e)?,
                Some(Commands::Format { stdout }) => {
                    format_recipe(&contents, &file, stdout).map_err(|e| *e)?
                }
                Some(Commands::Info { recipe }) => {
                    show_recipe_info(&contents, &file, &recipe).map_err(|e| *e)?
                }
                Some(Commands::External(ref ext)) => {
                    let (recipe, args): (Option<String>, Vec<String>) =
                        ext.split_first().map_or((None, vec![]), |(first, rest)| {
                            (Some(first.to_string()), rest.to_vec())
                        });
                    if let Some(recipe) = recipe {
                        run_recipe(&contents, &file, &recipe, &args, &cli).map_err(|e| *e)?;
                    } else {
                        list_recipes(&contents, &file).map_err(|e| *e)?;
                    }
                }
                None => {
                    list_recipes(&contents, &file).map_err(|e| *e)?;
                }
                _ => unreachable!(),
            }
        }
    }

    Ok(())
}

async fn start_lsp() -> miette::Result<()> {
    lsp::run().await
}

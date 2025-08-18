use core::error::runtime::RuntimeError;
use core::{BraiseError, Result};
use lexer::tokenize;
use parser::Parser as BraiseParser;
use runtime::{Runtime, ShellConfig, ShellMode as RuntimeShellMode};
use std::path::PathBuf;

use crate::commands::Cli;
use crate::utils;

pub fn run_recipe(
    contents: &str,
    file: &str,
    recipe_name: &str,
    args: &[String],
    cli: &Cli,
) -> Result<()> {
    let tokens = tokenize(contents)?;
    let mut parser = BraiseParser::new(&tokens, contents, file.to_string());
    let ast = parser.parse().map_err(|e| BraiseError::from(*e))?;

    // Configure shell execution
    let (runtime_shell_mode, shell_cmd) = match cli.shell.as_deref() {
        Some("isolated") => (RuntimeShellMode::Isolated, None),
        Some("persistent") => (RuntimeShellMode::Persistent, None),
        Some(cmd) if cmd.starts_with("isolated:") => {
            let shell_cmd = cmd.strip_prefix("isolated:").unwrap();
            (RuntimeShellMode::Isolated, Some(shell_cmd.to_string()))
        }
        Some(cmd) if cmd.starts_with("persistent:") => {
            let shell_cmd = cmd.strip_prefix("persistent:").unwrap();
            (RuntimeShellMode::Persistent, Some(shell_cmd.to_string()))
        }
        Some(cmd) => (RuntimeShellMode::Persistent, Some(cmd.to_string())),
        None => (RuntimeShellMode::Persistent, None), // Default to persistent
    };

    let mut shell_config = ShellConfig::new()
        .with_mode(runtime_shell_mode)
        .with_dry_run(cli.dry)
        .with_quiet(cli.quiet);

    if let Some(shell_cmd) = shell_cmd {
        shell_config = shell_config.with_shell(shell_cmd);
    }

    // Extract and resolve CLI arguments (including unnamed parameters) before creating runtime
    let cli_args = utils::extract_args(args);
    let params = utils::resolve_args(cli_args, &ast, recipe_name)
        .map_err(|e| BraiseError::from(RuntimeError::other(e).boxed()))?;

    let mut runtime =
        Runtime::new(ast, contents.to_string().into()).with_shell_config(shell_config);

    if cli.dry {
        runtime = runtime.with_dry_run();
    }

    if cli.quiet {
        runtime = runtime.with_quiet();
    }

    // Configure cache if enabled
    if !cli.no_cache {
        let mut cache_config = braise_cache::CacheConfig::new();

        // Set cache directory if specified
        if let Some(ref dir) = cli.cache_dir {
            cache_config = cache_config.with_storage_path(PathBuf::from(dir));
        }

        runtime = runtime.with_cache(cache_config);
    }

    runtime
        .execute_recipe(recipe_name, params)
        .map_err(BraiseError::from)?;

    Ok(())
}

use core::{BraiseError, Result};
use lexer::tokenize;
use parser::Parser as BraiseParser;
use runtime::Runtime;
use std::path::PathBuf;
use std::sync::Arc;

use crate::utils;

pub fn run_recipe(
    contents: &str,
    file: &str,
    recipe_name: &str,
    args: &[String],
    dry_run: bool,
    cache_enabled: bool,
    cache_dir: Option<String>,
) -> Result<()> {
    let tokens = tokenize(contents)?;
    let source = Arc::new(contents.to_string());
    let mut parser = BraiseParser::new(tokens, source.clone(), file.to_string());
    let ast = parser.parse().map_err(|e| BraiseError::from(*e))?;

    let mut runtime = Runtime::new(ast, source);
    if dry_run {
        runtime = runtime.with_dry_run();
    }

    // Configure cache if enabled
    if cache_enabled {
        let mut cache_config = braise_cache::CacheConfig::new();

        // Set cache directory if specified
        if let Some(dir) = cache_dir {
            cache_config = cache_config.with_storage_path(PathBuf::from(dir));
        }

        runtime = runtime.with_cache(cache_config);
    }

    let params = utils::extract_args(args);
    runtime
        .execute_recipe(recipe_name, params)
        .map_err(BraiseError::from)?;

    Ok(())
}

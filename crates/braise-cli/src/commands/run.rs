use core::Result;
use lexer::tokenize;
use parser::Parser as BraiseParser;
use runtime::Runtime;
use std::sync::Arc;

use crate::utils;

pub fn run_recipe(
    contents: &str,
    file: &str,
    recipe_name: &str,
    args: &[String],
    dry_run: bool,
) -> Result<()> {
    let tokens = tokenize(contents)?;
    let source = Arc::new(contents.to_string());
    let mut parser = BraiseParser::new(tokens, source.clone(), file.to_string());
    let ast = parser.parse()?;

    let mut runtime = Runtime::new(ast, source);
    if dry_run {
        runtime = runtime.with_dry_run();
    }

    let params = utils::extract_args(args);
    runtime.execute_recipe(recipe_name, params)?;

    Ok(())
}
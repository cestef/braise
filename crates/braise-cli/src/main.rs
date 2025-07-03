use clap::{Command, arg};
use core::{BraiseError, constants::DEFAULT_FILES, utils::find_first_existing_file};
use lexer::tokenize;
use parser::Parser;
use runtime::{Runtime, Value};
use std::collections::HashMap;

fn main() -> miette::Result<()> {
    Ok(run()?)
}

fn run() -> core::Result<()> {
    let matches = create().get_matches();
    let (sub, sub_matches) = matches.subcommand().ok_or(BraiseError::NoTask)?;
    let file = if let Some(file) = matches.get_one::<String>("file") {
        file
    } else {
        &find_first_existing_file(DEFAULT_FILES).ok_or(BraiseError::NoRecipeFileFound)?
    };

    let contents = std::fs::read_to_string(file).map_err(|e| BraiseError::ReadRecipeError {
        src: Box::new(e),
        file: file.clone(),
    })?;

    let tokens = tokenize(&contents).map_err(|error_span| BraiseError::LexerError {
        code: contents.clone(),
        span: miette::SourceSpan::new(error_span.start.into(), error_span.len()),
    })?;

    let mut parser = Parser::new(tokens, contents, file.clone());
    let ast = parser.parse()?;

    let mut runtime = Runtime::new(ast);

    if matches.get_flag("dry") {
        runtime = runtime.with_dry_run();
    }

    let args: Vec<String> = sub_matches
        .get_many::<std::ffi::os_str::OsString>("")
        .map(|args| args.map(|s| s.to_string_lossy().to_string()).collect())
        .unwrap_or_default();

    let params = extract_args(&args);

    runtime.execute_recipe(sub, params)?;
    Ok(())
}

pub fn create() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .allow_external_subcommands(true)
        .version(env!("CARGO_PKG_VERSION"))
        .author(clap::crate_authors!())
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(arg!(-f --file <FILE> "Path to the recipe file"))
        .arg(arg!(-d --dry "Dry run mode, does not execute the recipe"))
}

// --arg1 value1 --arg2=value2 -x 21 -y=23 --z
// {"arg1": "value1", "arg2": "value2", "x": "21", "y": "23", "z": "true"}
pub fn extract_args(args: &[String]) -> HashMap<String, Value> {
    let mut result = HashMap::new();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        if arg.starts_with("--") {
            let arg_name = arg.trim_start_matches("--");

            if let Some(equals_pos) = arg_name.find('=') {
                let (name, value) = arg_name.split_at(equals_pos);
                result.insert(name.to_string(), Value::String(value[1..].to_string()));
            } else if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                result.insert(arg_name.to_string(), Value::String(args[i + 1].clone()));
                i += 1;
            } else {
                result.insert(arg_name.to_string(), Value::Bool(true));
            }
        } else if arg.starts_with('-') {
            let arg_name = arg.trim_start_matches("-");

            if let Some(equals_pos) = arg_name.find('=') {
                let (name, value) = arg_name.split_at(equals_pos);
                result.insert(name.to_string(), Value::String(value[1..].to_string()));
            } else if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                result.insert(arg_name.to_string(), Value::String(args[i + 1].clone()));
                i += 1;
            } else {
                result.insert(arg_name.to_string(), Value::Bool(true));
            }
        }

        i += 1;
    }

    result
}

use clap::{Command, arg};
use core::{
    BraiseError,
    constants::DEFAULT_FILES,
    utils::{extract_args, find_first_existing_file},
};
use lexer::tokenize;
use parser::Parser;
use runtime::Runtime;

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

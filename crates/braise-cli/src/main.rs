use clap::{Command, arg};
use core::{cli::CliError, constants::DEFAULT_FILES};
use lexer::tokenize;
use parser::Parser;
use runtime::Runtime;

mod utils;

fn main() -> miette::Result<()> {
    Ok(run()?)
}

fn run() -> core::Result<()> {
    let matches = create_cli().get_matches();
    let (sub, sub_matches) = matches.subcommand().ok_or(CliError::NoTask)?;
    let file = if let Some(file) = matches.get_one::<String>("file") {
        file
    } else {
        &utils::find_first_existing_file(DEFAULT_FILES).ok_or(CliError::NoRecipeFileFound)?
    };

    let contents = std::fs::read_to_string(file).map_err(|e| CliError::ReadRecipeError {
        src: Box::new(e),
        file: file.clone(),
    })?;

    if matches.get_flag("format") {
        let formatted = fmt::Formatter::format(&contents);
        println!("{}", formatted);
        return Ok(());
    }

    let tokens = tokenize(&contents)?;

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

    let params = utils::extract_args(&args);

    runtime.execute_recipe(sub, params)?;
    Ok(())
}

pub fn create_cli() -> Command {
    Command::new(env!("CARGO_PKG_NAME"))
        .allow_external_subcommands(true)
        .version(env!("CARGO_PKG_VERSION"))
        .author(clap::crate_authors!())
        .about(env!("CARGO_PKG_DESCRIPTION"))
        .arg(arg!(-f --file <FILE> "Path to the recipe file"))
        .arg(arg!(-F --format "Format the provided recipe file"))
        .arg(arg!(-d --dry "Dry run mode, does not execute the recipe"))
}

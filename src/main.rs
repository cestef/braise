use braise::{BraiseError, cli::Opts, lexer::Token, parser::Parser, runtime::Runtime};
use clap::Parser as _;
use logos::Logos;

fn main() -> miette::Result<()> {
    Ok(run()?)
}

fn run() -> braise::Result<()> {
    let opts = Opts::parse();
    let contents = std::fs::read_to_string(opts.file)?;
    let mut lexer = Token::lexer(&contents);

    let tokens = lexer
        .by_ref()
        .map(|token| token)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| BraiseError::LexerError)?;

    let mut parser = Parser::new(tokens);
    let ast = parser.parse()?;
    let runtime = Runtime::with_dry_run(ast);
    runtime.execute_recipe(
        &opts.task,
        vec![("service", "api")]
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
    )?;
    Ok(())
}

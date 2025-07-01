use braise::{
    BraiseError, cli,
    constants::DEFAULT_FILES,
    lexer::{SpannedToken, Token},
    parser::Parser,
    runtime::Runtime,
    utils::{extract_args, find_first_existing_file},
};
use logos::Logos;

fn main() -> miette::Result<()> {
    Ok(run()?)
}

fn run() -> braise::Result<()> {
    let matches = cli::create().get_matches();
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

    let mut lexer = Token::lexer(&contents).spanned();

    let mut tokens = vec![];
    while let Some((token, span)) = lexer.next() {
        if let Ok(token) = token {
            tokens.push(SpannedToken::new(token, span));
        } else {
            return Err(BraiseError::LexerError {
                code: contents.clone(),
                span: span.into(),
            });
        }
    }

    let mut parser = Parser::new(tokens, contents);
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

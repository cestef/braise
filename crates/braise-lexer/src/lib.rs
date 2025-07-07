use std::ops::Range;

use braise_core::BraiseError;
use logos::{Lexer, Logos, Skip};

pub use crate::{extras::LexerExtras, spanned_token::SpannedToken};

pub mod extras;
pub mod spanned_token;

fn newline_callback(lex: &mut Lexer<Token>) -> Skip {
    let slice = lex.slice();
    let span = lex.span();

    let newline_count = slice.matches('\n').count();

    if newline_count > 0 {
        lex.extras.line += newline_count as u32;

        if let Some(last_newline_pos) = slice.rfind('\n') {
            lex.extras.line_start_offset = span.start + last_newline_pos + 1;
        }
    }

    Skip
}

#[derive(Logos, Debug, PartialEq, Clone, logos_display::Display)]
#[logos(skip(r"[ \t\r]+"))] // whitespace
#[logos(skip(r"\n", callback = newline_callback))] // newlines
#[logos(error(Range<usize>, callback = |lex| lex.span()))]
#[logos(extras = LexerExtras)]
pub enum Token {
    // Keywords
    #[token("recipe")]
    Recipe,
    #[token("param")]
    Param,
    #[token("run")]
    Run,
    #[token("if")]
    If,
    #[token("else")]
    Else,
    #[token("match")]
    Match,
    #[token("for")]
    For,
    #[token("in")]
    In,
    #[token("async")]
    Async,
    #[token("exit")]
    Exit,
    #[token("print")]
    Print,
    #[token("let")]
    Let,
    #[token("call")]
    Call,
    #[token("shell")]
    Shell,

    // Type keywords
    #[token("string")]
    StringType,
    #[token("number")]
    NumberType,
    #[token("int")]
    IntType,
    #[token("bool")]
    BoolType,
    #[token("array")]
    ArrayType,

    // Literals
    #[regex(r#""[^"]*""#, |lex| lex.slice().trim_matches('"').to_string())]
    #[alias("string")]
    String(String),
    #[regex(r#"[0-9]+(\.[0-9]+)?"#, |lex| lex.slice().parse::<f64>().unwrap())]
    #[alias("number")]
    Number(f64),
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    #[alias("identifier")]
    Identifier(String),
    #[regex(r"true|false", |lex| lex.slice().parse::<bool>().unwrap())]
    #[alias("bool")]
    Bool(bool),

    // Logical operators
    #[token("&&")]
    And,
    #[token("||")]
    Or,

    // Comparison operators
    #[token("==")]
    EqualEqual,
    #[token("!=")]
    NotEqual,
    #[token("<=")]
    LessEqual,
    #[token(">=")]
    GreaterEqual,
    #[token("<")]
    Less,
    #[token(">")]
    Greater,

    // Operators
    #[token("->")]
    Arrow,
    #[token("=>")]
    FatArrow,
    #[token("=")]
    Equals,
    #[token(".")]
    Dot,
    #[token("!")]
    Bang,

    // Custom operators
    #[token("@")]
    At,
    #[token("|")]
    Pipe,
    #[token("?")]
    QuestionMark,

    // Delimiters
    #[token("{")]
    LeftBrace,
    #[token("}")]
    RightBrace,
    #[token("[")]
    LeftBracket,
    #[token("]")]
    RightBracket,
    #[token("(")]
    LeftParen,
    #[token(")")]
    RightParen,
    #[token(",")]
    Comma,
    #[token(":")]
    Colon,

    // Comments
    #[regex(r"//[^\n]*", logos::skip)]
    #[alias("line_comment")]
    LineComment,
    #[regex(r"/\*([^*]|\*[^/])*\*/", logos::skip)]
    #[alias("block_comment")]
    BlockComment,

    EOF,
}

/// Create a lexer with proper position tracking
pub fn create_lexer<'a>(source: &'a str) -> logos::Lexer<'a, Token> {
    let mut lexer = Token::lexer(source);
    lexer.extras = LexerExtras::new(source.to_string());
    lexer
}

/// Tokenize source code and return a vector of spanned tokens
pub fn tokenize(source: &str) -> Result<Vec<SpannedToken>, BraiseError> {
    _tokenize(source).map_err(|error_span| BraiseError::LexerError {
        code: source.to_string(),
        span: miette::SourceSpan::new(error_span.start.into(), error_span.len()),
    })
}

fn _tokenize(source: &str) -> Result<Vec<SpannedToken>, Range<usize>> {
    let mut lexer = create_lexer(source);
    let mut tokens = Vec::new();

    while let Some(result) = lexer.next() {
        match result {
            Ok(token) => {
                let span = lexer.extras.span_from_range(lexer.span());
                tokens.push(SpannedToken::new(token, span));
            }
            Err(r) => {
                return Err(r);
            }
        }
    }

    Ok(tokens)
}

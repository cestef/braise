use std::ops::Range;

use logos::{Lexer, Logos, Skip};

/// Update the line count and the char index.
fn newline_callback(lex: &mut Lexer<Token>) -> Skip {
    lex.extras.0 += if lex.slice().contains('\n') {
        lex.slice().matches('\n').count()
    } else {
        0
    };
    lex.extras.1 = lex.span().end;
    Skip
}

#[derive(Logos, Debug, PartialEq, Clone, logos_display::Display)]
#[logos(skip(r"[ \t\n\f]+", callback = newline_callback))] // Skip whitespace
#[logos(error(Range<usize>, callback = |lex| lex.span()))]
#[logos(extras = (usize, usize))] // Track line and column numbers
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

    // Literals
    #[regex(r#""[^"]*""#, |lex| lex.slice().trim_matches('"').to_string())]
    String(String),
    #[regex(r#"[0-9]+(\.[0-9]+)?"#, |lex| lex.slice().parse::<f64>().unwrap())]
    Number(f64),
    #[regex(r"[a-zA-Z_][a-zA-Z0-9_]*", |lex| lex.slice().to_string())]
    Identifier(String),
    #[regex(r"true|false", |lex| lex.slice().parse::<bool>().unwrap())]
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
    LineComment,
    #[regex(r"/\*[^*]*\*+(?:[^/*][^*/]*)*\*/", logos::skip)]
    BlockComment,

    EOF,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub span: Range<usize>,
}

impl SpannedToken {
    pub fn new(token: Token, span: Range<usize>) -> Self {
        SpannedToken { token, span }
    }
}

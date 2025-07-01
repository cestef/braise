use logos::Logos;

#[derive(Logos, Debug, PartialEq, Clone)]
#[logos(skip r"[ \t\n\f]+")] // Skip whitespace
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
}

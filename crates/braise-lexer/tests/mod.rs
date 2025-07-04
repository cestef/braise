#[cfg(test)]
mod tests {
    use braise_core::BraiseError;
    use braise_lexer::{Token, tokenize};

    #[test]
    fn test_keywords() {
        let input = "recipe param run if else match for let print exit call shell";
        let tokens = tokenize(input).unwrap();

        let expected = vec![
            Token::Recipe,
            Token::Param,
            Token::Run,
            Token::If,
            Token::Else,
            Token::Match,
            Token::For,
            Token::Let,
            Token::Print,
            Token::Exit,
            Token::Call,
            Token::Shell,
        ];

        for (token, expected) in tokens.iter().zip(expected.iter()) {
            assert_eq!(&token.token, expected);
        }
    }

    #[test]
    fn test_types() {
        let input = "string number bool array";
        let tokens = tokenize(input).unwrap();

        let expected = vec![
            Token::StringType,
            Token::NumberType,
            Token::BoolType,
            Token::ArrayType,
        ];

        for (token, expected) in tokens.iter().zip(expected.iter()) {
            assert_eq!(&token.token, expected);
        }
    }

    #[test]
    fn test_literals() {
        let input = r#""hello world" 42 3.14 true false"#;
        let tokens = tokenize(input).unwrap();

        assert_eq!(tokens[0].token, Token::String("hello world".to_string()));
        assert_eq!(tokens[1].token, Token::Number(42.0));
        assert_eq!(tokens[2].token, Token::Number(3.14));
        assert_eq!(tokens[3].token, Token::Bool(true));
        assert_eq!(tokens[4].token, Token::Bool(false));
    }

    #[test]
    fn test_operators() {
        let input = "== != <= >= < > && || ! = . -> => @";
        let tokens = tokenize(input).unwrap();

        let expected = vec![
            Token::EqualEqual,
            Token::NotEqual,
            Token::LessEqual,
            Token::GreaterEqual,
            Token::Less,
            Token::Greater,
            Token::And,
            Token::Or,
            Token::Bang,
            Token::Equals,
            Token::Dot,
            Token::Arrow,
            Token::FatArrow,
            Token::At,
        ];

        for (token, expected) in tokens.iter().zip(expected.iter()) {
            assert_eq!(&token.token, expected);
        }
    }

    #[test]
    fn test_delimiters() {
        let input = "{ } [ ] ( ) , :";
        let tokens = tokenize(input).unwrap();

        let expected = vec![
            Token::LeftBrace,
            Token::RightBrace,
            Token::LeftBracket,
            Token::RightBracket,
            Token::LeftParen,
            Token::RightParen,
            Token::Comma,
            Token::Colon,
        ];

        for (token, expected) in tokens.iter().zip(expected.iter()) {
            assert_eq!(&token.token, expected);
        }
    }

    #[test]
    fn test_identifiers() {
        let input = "myvar _private some123 test_var";
        let tokens = tokenize(input).unwrap();

        assert_eq!(tokens[0].token, Token::Identifier("myvar".to_string()));
        assert_eq!(tokens[1].token, Token::Identifier("_private".to_string()));
        assert_eq!(tokens[2].token, Token::Identifier("some123".to_string()));
        assert_eq!(tokens[3].token, Token::Identifier("test_var".to_string()));
    }

    #[test]
    fn test_comments_are_skipped() -> miette::Result<()> {
        let input = r#"
            recipe "test" { // line comment
                /* block comment */ run "echo hi"
                /*
                Another block comment
                */
            }
        "#;
        let tokens = tokenize(&input).map_err(|error_span| BraiseError::LexerError {
            code: input.to_string(),
            span: miette::SourceSpan::new(error_span.start.into(), error_span.len()),
        })?;
        // Comments should be skipped, so we should only see the recipe tokens
        assert_eq!(tokens[0].token, Token::Recipe);
        assert_eq!(tokens[1].token, Token::String("test".to_string()));
        assert_eq!(tokens[2].token, Token::LeftBrace);
        assert_eq!(tokens[3].token, Token::Run);
        assert_eq!(tokens[4].token, Token::String("echo hi".to_string()));
        assert_eq!(tokens[5].token, Token::RightBrace);
        Ok(())
    }

    #[test]
    fn test_position_tracking() -> miette::Result<()> {
        let input = "recipe\n\"test\"";
        let tokens = tokenize(&input).map_err(|error_span| BraiseError::LexerError {
            code: input.to_string(),
            span: miette::SourceSpan::new(error_span.start.into(), error_span.len()),
        })?;
        // First token should be on line 1
        assert_eq!(tokens[0].span.start.line, 1);
        assert_eq!(tokens[0].span.start.column, 1);

        // Second token should be on line 2
        assert_eq!(tokens[1].span.start.line, 2);
        assert_eq!(tokens[1].span.start.column, 1);
        Ok(())
    }

    #[test]
    fn test_error_handling() {
        let input = "recipe \"test\" # invalid char";
        let result = tokenize(input);

        assert!(result.is_err());
        let error_span = result.unwrap_err();
        // Error should point to the '#' character
        assert!(error_span.start > 13); // After "recipe \"test\" "
    }
}

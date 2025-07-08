use ::lexer::Token;
use core::{error::parser::ParseError, *};
use miette::SourceSpan;

use crate::Parser;

impl Parser {
    pub fn create_error(&self, expected: String, found: Token) -> ParseError {
        let (span, code) = if self.current < self.tokens.len() {
            let token_span = &self.tokens[self.current].span;
            let start_offset = token_span.start.offset;
            let end_offset = token_span.end.offset;
            (
                SourceSpan::new(start_offset.into(), end_offset.saturating_sub(start_offset)),
                self.source.as_ref().clone(),
            )
        } else {
            (
                SourceSpan::new(self.source.len().into(), 0),
                self.source.as_ref().clone(),
            )
        };

        ParseError::unexpected_token(expected, found.to_string(), code, span)
    }

    pub fn get_current_span(&self) -> Span {
        if self.current < self.tokens.len() {
            let token_span = &self.tokens[self.current].span;
            Span::from_token_span(token_span, self.file_id)
        } else {
            // EOF span
            let end_pos = Position::new(1, 1, self.source.len());
            Span::new(end_pos, end_pos, self.file_id)
        }
    }

    pub fn span_from_token_range(&self, start_token: usize, end_token: usize) -> Span {
        if start_token < self.tokens.len() && end_token <= self.tokens.len() {
            let start_span = &self.tokens[start_token].span;
            let end_span = if end_token > 0 && end_token <= self.tokens.len() {
                &self.tokens[end_token - 1].span
            } else {
                start_span
            };

            Span::new(
                start_span.start.clone().into(),
                end_span.end.clone().into(),
                self.file_id,
            )
        } else {
            self.get_current_span()
        }
    }

    pub fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }

    pub fn peek(&self) -> &Token {
        self.tokens
            .get(self.current)
            .map(|e| &e.token)
            .unwrap_or(&Token::EOF)
    }

    pub fn peek_ahead(&self, n: usize) -> Option<&Token> {
        self.tokens.get(self.current + n).map(|token| &token.token)
    }

    pub fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    pub fn previous(&self) -> &Token {
        if self.current > 0 && self.current <= self.tokens.len() {
            &self.tokens[self.current - 1].token
        } else {
            &Token::EOF
        }
    }

    pub fn check(&self, token_type: &Token) -> bool {
        if self.is_at_end() {
            return false;
        }
        std::mem::discriminant(self.peek()) == std::mem::discriminant(token_type)
    }

    pub fn check_identifier(&self) -> bool {
        matches!(self.peek(), Token::Identifier(_))
    }

    pub fn check_string(&self) -> bool {
        matches!(self.peek(), Token::String(_))
    }

    pub fn check_number(&self) -> bool {
        matches!(self.peek(), Token::Number(_))
    }

    pub fn check_identifier_value(&self, expected: &str) -> bool {
        matches!(self.peek(), Token::Identifier(name) if name == expected)
    }

    pub fn check_wildcard(&self) -> bool {
        self.check_identifier_value("_")
    }

    pub fn get_identifier_value(&self) -> Option<&str> {
        match self.peek() {
            Token::Identifier(name) => Some(name),
            _ => None,
        }
    }

    pub fn get_string_value(&self) -> Option<&str> {
        match self.peek() {
            Token::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn get_number_value(&self) -> Option<f64> {
        match self.peek() {
            Token::Number(n) => Some(*n),
            _ => None,
        }
    }

    pub fn get_bool_value(&self) -> Option<bool> {
        match self.peek() {
            Token::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn match_token(&mut self, token_type: &Token) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub fn match_identifier(&mut self) -> Option<String> {
        if let Token::Identifier(name) = self.peek().clone() {
            self.advance();
            Some(name)
        } else {
            None
        }
    }

    pub fn match_string(&mut self) -> Option<String> {
        if let Token::String(s) = self.peek().clone() {
            self.advance();
            Some(s)
        } else {
            None
        }
    }

    pub fn match_number(&mut self) -> Option<f64> {
        if let Token::Number(n) = self.peek().clone() {
            self.advance();
            Some(n)
        } else {
            None
        }
    }

    pub fn match_identifier_value(&mut self, expected: &str) -> bool {
        if self.check_identifier_value(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    pub fn consume_token(&mut self, expected: Token) -> core::error::parser::Result<&Token> {
        if self.check(&expected) {
            Ok(self.advance())
        } else {
            Err(self.create_error(expected.to_string(), self.peek().clone()))
        }
    }

    pub fn parse_identifier(&mut self) -> core::error::parser::Result<String> {
        match self.peek() {
            Token::Identifier(s) => {
                let result = s.clone();
                self.advance();
                Ok(result)
            }
            _ => Err(self.create_error("identifier".to_string(), self.peek().clone())),
        }
    }

    pub fn parse_string(&mut self) -> core::error::parser::Result<String> {
        match self.peek() {
            Token::String(s) => {
                let result = s.clone();
                self.advance();
                Ok(result)
            }
            _ => Err(self.create_error("string".to_string(), self.peek().clone())),
        }
    }

    pub fn parse_statement_block(
        &mut self,
    ) -> core::error::parser::Result<Vec<SpannedNode<Statement>>> {
        let mut statements = Vec::new();

        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }

        self.consume_token(Token::RightBrace)?;
        Ok(statements)
    }
}

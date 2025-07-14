use ::lexer::Token;
use core::{ast::*, error::parser::Result, *};

use crate::Parser;

impl<'input> Parser<'input> {
    /// Parse a match pattern with full support for all pattern types
    pub fn parse_match_pattern(&mut self) -> Result<MatchPattern> {
        self.parse_or_pattern()
    }

    /// Parse or patterns (highest precedence in pattern matching)
    fn parse_or_pattern(&mut self) -> Result<MatchPattern> {
        let mut patterns = vec![SpannedNode::new(
            self.parse_guard_pattern()?,
            self.get_current_span(),
        )];

        while self.match_token(&Token::Or) {
            patterns.push(SpannedNode::new(
                self.parse_guard_pattern()?,
                self.get_current_span(),
            ));
        }

        if patterns.len() == 1 {
            Ok(patterns.into_iter().next().unwrap().value)
        } else {
            Ok(MatchPattern::Or(patterns))
        }
    }

    /// Parse guard patterns (pattern if condition)
    fn parse_guard_pattern(&mut self) -> Result<MatchPattern> {
        let pattern = self.parse_primary_pattern()?;

        if self.match_token(&Token::If) {
            let condition = self.parse_expression()?;
            Ok(MatchPattern::Guard {
                pattern: Box::new(SpannedNode::new(pattern, self.get_current_span())),
                condition,
            })
        } else {
            Ok(pattern)
        }
    }

    /// Parse primary patterns (literals, wildcards, arrays, etc.)
    fn parse_primary_pattern(&mut self) -> Result<MatchPattern> {
        match self.peek().clone() {
            Token::String(s) => {
                self.advance();
                Ok(MatchPattern::String(s))
            }

            Token::Number(n) => {
                self.advance();

                if self.check(&Token::Dot) && self.peek_ahead(1) == Some(&Token::Dot) {
                    self.advance();
                    self.advance();

                    let inclusive = self.match_token(&Token::Equals);

                    if let Some(end) = self.get_number_value() {
                        self.advance();
                        Ok(MatchPattern::Range {
                            start: Some(n),
                            end: Some(end),
                            inclusive,
                        })
                    } else {
                        Ok(MatchPattern::Range {
                            start: Some(n),
                            end: None,
                            inclusive,
                        })
                    }
                } else {
                    Ok(MatchPattern::Number(n))
                }
            }

            Token::Bool(b) => {
                self.advance();
                Ok(MatchPattern::Bool(b))
            }

            Token::Identifier(name) => {
                self.advance();

                if name == "_" {
                    return Ok(MatchPattern::Wildcard);
                }

                if self.match_token(&Token::LeftParen) {
                    let param_type = match name.as_str() {
                        "string" => BraiseType::String,
                        "number" => BraiseType::Number,
                        "bool" => BraiseType::Bool,
                        "any" => BraiseType::Any,
                        _ => {
                            return Err(self
                                .create_error(
                                    "valid type name".to_string(),
                                    Token::Identifier(name),
                                )
                                .boxed());
                        }
                    };

                    self.consume_token(Token::RightParen)?;
                    Ok(MatchPattern::Type(param_type))
                } else {
                    Ok(MatchPattern::Variable(name))
                }
            }

            Token::LeftBracket => self.parse_array_pattern(),

            Token::Dot => self.parse_range_pattern_from_dot(),

            Token::LeftParen => {
                self.advance();
                let pattern = self.parse_match_pattern()?;
                self.consume_token(Token::RightParen)?;
                Ok(pattern)
            }

            _ => Err(self
                .create_error("match pattern".to_string(), self.peek().clone())
                .boxed()),
        }
    }

    fn parse_array_pattern(&mut self) -> Result<MatchPattern> {
        self.advance();
        let mut elements = Vec::new();

        if !self.check(&Token::RightBracket) {
            loop {
                if self.match_token(&Token::Dot) {
                    self.consume_token(Token::Dot)?;

                    if self.check_wildcard() {
                        self.advance();
                        elements.push(SpannedNode::new(
                            ArrayPatternElement::Rest(None),
                            self.get_current_span(),
                        ));
                    } else if let Some(name) = self.match_identifier() {
                        elements.push(SpannedNode::new(
                            ArrayPatternElement::Rest(Some(name)),
                            self.get_current_span(),
                        ));
                    } else {
                        return Err(self
                            .create_error(
                                "rest pattern variable or _".to_string(),
                                self.peek().clone(),
                            )
                            .boxed());
                    }
                    break;
                } else if self.check_wildcard() {
                    self.advance();
                    elements.push(SpannedNode::new(
                        ArrayPatternElement::Wildcard,
                        self.get_current_span(),
                    ));
                } else if self.check_identifier() {
                    let name = self.parse_identifier()?;
                    elements.push(SpannedNode::new(
                        ArrayPatternElement::Pattern(MatchPattern::Variable(name)),
                        self.get_current_span(),
                    ));
                } else {
                    let pattern = self.parse_match_pattern()?;
                    elements.push(SpannedNode::new(
                        ArrayPatternElement::Pattern(pattern),
                        self.get_current_span(),
                    ));
                }

                if !self.match_token(&Token::Comma) {
                    break;
                }
            }
        }

        self.consume_token(Token::RightBracket)?;
        Ok(MatchPattern::Array { elements })
    }

    fn parse_range_pattern_from_dot(&mut self) -> Result<MatchPattern> {
        self.advance();
        self.consume_token(Token::Dot)?;

        let inclusive = self.match_token(&Token::Equals);

        if let Some(end) = self.get_number_value() {
            self.advance();
            Ok(MatchPattern::Range {
                start: None,
                end: Some(end),
                inclusive,
            })
        } else {
            Ok(MatchPattern::Range {
                start: None,
                end: None,
                inclusive,
            })
        }
    }
}

use ::lexer::Token;
use core::{error::parser::Result, *};
use std::collections::HashMap;

use crate::Parser;

impl Parser {
    pub fn parse_expression(&mut self) -> Result<SpannedNode<Expression>> {
        let start_token = self.current;
        let expr = self.parse_conditional()?;
        let end_token = self.current;
        let span = self.span_from_token_range(start_token, end_token);
        Ok(SpannedNode::new(expr, span))
    }

    fn parse_conditional(&mut self) -> Result<Expression> {
        if self.check(&Token::If) {
            self.advance(); // consume if
            let condition = self.parse_logical_or()?;
            let condition_span = self.get_current_span();

            self.consume_token(Token::LeftBrace)?;
            let then_expr = self.parse_expression()?;
            self.consume_token(Token::RightBrace)?;

            self.consume_token(Token::Else)?;
            self.consume_token(Token::LeftBrace)?;
            let else_expr = self.parse_expression()?;
            self.consume_token(Token::RightBrace)?;

            Ok(Expression::Conditional {
                condition: Box::new(SpannedNode::new(condition, condition_span)),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
            })
        } else {
            self.parse_logical_or()
        }
    }

    fn parse_logical_or(&mut self) -> Result<Expression> {
        let mut expr = self.parse_logical_and()?;

        while self.match_token(&Token::Or) {
            let right = self.parse_logical_and()?;
            let span = self.get_current_span();
            expr = Expression::BinaryOp {
                left: Box::new(SpannedNode::new(expr, span.clone())),
                op: BinaryOperator::Or,
                right: Box::new(SpannedNode::new(right, span)),
            };
        }

        Ok(expr)
    }

    fn parse_logical_and(&mut self) -> Result<Expression> {
        let mut expr = self.parse_equality()?;

        while self.match_token(&Token::And) {
            let right = self.parse_equality()?;
            let span = self.get_current_span();
            expr = Expression::BinaryOp {
                left: Box::new(SpannedNode::new(expr, span.clone())),
                op: BinaryOperator::And,
                right: Box::new(SpannedNode::new(right, span)),
            };
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expression> {
        let mut expr = self.parse_comparison()?;

        while let Token::EqualEqual | Token::NotEqual = self.peek() {
            let op = match self.peek() {
                Token::EqualEqual => {
                    self.advance();
                    BinaryOperator::Equal
                }
                Token::NotEqual => {
                    self.advance();
                    BinaryOperator::NotEqual
                }
                _ => break,
            };

            let right = self.parse_comparison()?;
            let span = self.get_current_span();
            expr = Expression::BinaryOp {
                left: Box::new(SpannedNode::new(expr, span.clone())),
                op,
                right: Box::new(SpannedNode::new(right, span)),
            };
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expression> {
        let mut expr = self.parse_primary()?;

        while let Token::Less | Token::LessEqual | Token::Greater | Token::GreaterEqual =
            self.peek()
        {
            let op = match self.peek() {
                Token::Less => {
                    self.advance();
                    BinaryOperator::Less
                }
                Token::LessEqual => {
                    self.advance();
                    BinaryOperator::LessEqual
                }
                Token::Greater => {
                    self.advance();
                    BinaryOperator::Greater
                }
                Token::GreaterEqual => {
                    self.advance();
                    BinaryOperator::GreaterEqual
                }
                _ => break,
            };

            let right = self.parse_primary()?;
            let span = self.get_current_span();
            expr = Expression::BinaryOp {
                left: Box::new(SpannedNode::new(expr, span.clone())),
                op,
                right: Box::new(SpannedNode::new(right, span)),
            };
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expression> {
        match self.peek().clone() {
            Token::String(s) => {
                self.advance();
                // str interpolation
                if s.contains("${") {
                    Ok(Expression::Interpolation(self.parse_interpolation(&s)?))
                } else {
                    Ok(Expression::String(s))
                }
            }
            Token::Number(n) => {
                self.advance();
                Ok(Expression::Number(n))
            }
            Token::Bool(b) => {
                self.advance();
                Ok(Expression::Bool(b))
            }
            Token::Identifier(name) => {
                self.advance();

                if self.match_token(&Token::Dot) {
                    let field = self.parse_identifier()?;
                    if self.match_token(&Token::LeftParen) {
                        // function
                        let mut args = Vec::new();
                        if !self.check(&Token::RightParen) {
                            loop {
                                args.push(self.parse_expression()?);
                                if !self.match_token(&Token::Comma) {
                                    break;
                                }
                            }
                        }

                        self.consume_token(Token::RightParen)?;

                        Ok(Expression::FunctionCall {
                            module: name,
                            function: field,
                            args,
                        })
                    } else {
                        Ok(Expression::ModuleAccess {
                            module: name,
                            field,
                        })
                    }
                } else {
                    Ok(Expression::Variable(name))
                }
            }
            Token::LeftBracket => {
                self.advance();
                let mut elements = Vec::new();

                if !self.check(&Token::RightBracket) {
                    loop {
                        elements.push(self.parse_expression()?);
                        if !self.match_token(&Token::Comma) {
                            break;
                        }
                    }
                }

                self.consume_token(Token::RightBracket)?;
                Ok(Expression::Array(elements))
            }
            Token::Bang => {
                self.advance();
                let expr = self.parse_expression()?;
                Ok(Expression::UnaryOp {
                    op: UnaryOperator::Not,
                    expr: Box::new(expr),
                })
            }
            Token::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.consume_token(Token::RightParen)?;
                Ok(expr.value) // Unwrap the SpannedNode to get the Expression
            }
            Token::At => {
                self.advance();
                let recipe_name = self.parse_identifier()?;
                let mut args = HashMap::new();
                if self.match_token(&Token::LeftParen) {
                    if !self.check(&Token::RightParen) {
                        loop {
                            let arg_name = self.parse_identifier()?;
                            self.consume_token(Token::Colon)?;
                            let arg_value = self.parse_expression()?;
                            args.insert(arg_name, arg_value);
                            if !self.match_token(&Token::Comma) {
                                break;
                            }
                        }
                    }
                    self.consume_token(Token::RightParen)?;
                }

                Ok(Expression::RecipeRef {
                    recipe: recipe_name,
                    args,
                })
            }
            Token::Match => self.parse_match_expression(),
            e => Err(self.create_error("expression".to_string(), e)),
        }
    }

    pub fn parse_match_expression(&mut self) -> Result<Expression> {
        self.advance(); // consume 'match'
        let expr = self.parse_expression()?;
        self.consume_token(Token::LeftBrace)?;

        let mut arms = Vec::new();

        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            let arm_start = self.current;

            let pattern = self.parse_match_pattern()?;

            // Optional guard condition
            let guard = if self.match_token(&Token::If) {
                Some(self.parse_expression()?)
            } else {
                None
            };

            self.consume_token(Token::FatArrow)?;

            let body_expr = self.parse_expression()?;

            let arm_end = self.current;
            let arm_span = self.span_from_token_range(arm_start, arm_end);

            let arm = MatchExpressionArm {
                pattern,
                guard,
                expr: body_expr,
                bindings: HashMap::new(),
            };
            arms.push(SpannedNode::new(arm, arm_span));

            if !self.check(&Token::RightBrace) {
                self.consume_token(Token::Comma)?;
            }
        }

        self.consume_token(Token::RightBrace)?;

        Ok(Expression::Match {
            expr: Box::new(expr),
            arms,
        })
    }

    fn parse_interpolation(&self, s: &str) -> Result<Vec<InterpolationPart>> {
        let mut parts = Vec::new();
        let mut current = String::new();
        let mut chars = s.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '$' && chars.peek() == Some(&'{') {
                chars.next(); // consume '{'

                if !current.is_empty() {
                    parts.push(InterpolationPart::String(current.clone()));
                    current.clear();
                }

                let mut expr_text = String::new();
                let mut brace_count = 1;

                for ch in chars.by_ref() {
                    if ch == '{' {
                        brace_count += 1;
                    } else if ch == '}' {
                        brace_count -= 1;
                        if brace_count == 0 {
                            break;
                        }
                    }
                    expr_text.push(ch);
                }

                let expr = self.parse_interpolation_expression(&expr_text)?;
                let span = self.get_current_span();
                parts.push(InterpolationPart::Expression(SpannedNode::new(expr, span)));
            } else {
                current.push(ch);
            }
        }

        if !current.is_empty() {
            parts.push(InterpolationPart::String(current));
        }

        Ok(parts)
    }

    fn parse_interpolation_expression(&self, expr_text: &str) -> Result<Expression> {
        // TODO: This should be handled via a separate lexer
        let trimmed = expr_text.trim();

        // num
        if let Ok(num) = trimmed.parse::<f64>() {
            return Ok(Expression::Number(num));
        }

        // bool
        match trimmed {
            "true" => return Ok(Expression::Bool(true)),
            "false" => return Ok(Expression::Bool(false)),
            _ => {}
        }

        // module field or func
        if let Some(dot_idx) = trimmed.find('.') {
            let module = trimmed[..dot_idx].trim().to_string();
            let rest = &trimmed[dot_idx + 1..];

            // function call
            if let Some(paren_idx) = rest.find('(') {
                let function = rest[..paren_idx].trim().to_string();

                if rest.ends_with(')') {
                    let args_str = &rest[paren_idx + 1..rest.len() - 1];
                    let mut args = Vec::new();

                    if !args_str.is_empty() {
                        for arg in args_str.split(',') {
                            let expr = self.parse_interpolation_expression(arg.trim())?;
                            let span = self.get_current_span();
                            args.push(SpannedNode::new(expr, span));
                        }
                    }

                    return Ok(Expression::FunctionCall {
                        module,
                        function,
                        args,
                    });
                }
            } else {
                return Ok(Expression::ModuleAccess {
                    module,
                    field: rest.trim().to_string(),
                });
            }
        }

        // default to var
        Ok(Expression::Variable(trimmed.to_string()))
    }
}

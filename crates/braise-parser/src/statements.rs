use ::lexer::Token;
use core::{error::parser::Result, *};
use std::collections::HashMap;

use crate::Parser;

impl Parser {
    pub fn parse_statement(&mut self) -> Result<SpannedNode<Statement>> {
        let start_token = self.current;

        let statement = match self.peek() {
            Token::Run => self.parse_run_statement()?,
            Token::Exit => self.parse_exit_statement()?,
            Token::Print => self.parse_print_statement()?,
            Token::If => self.parse_if_statement()?,
            Token::Match => self.parse_match_statement()?,
            Token::For => self.parse_for_statement()?,
            Token::Let => self.parse_let_statement()?,
            Token::Identifier(name) => self.parse_assign_statement(name.clone())?,
            Token::Call => self.parse_call_statement()?,
            Token::Shell => self.parse_shell_statement()?,
            e => return Err(self.create_error("statement".to_string(), e.clone())),
        };

        let end_token = self.current;
        let span = self.span_from_token_range(start_token, end_token);

        Ok(SpannedNode::new(statement, span))
    }

    fn parse_shell_statement(&mut self) -> Result<Statement> {
        self.consume_token(Token::Shell)?;
        let name = self.parse_string()?;
        Ok(Statement::Shell { name })
    }

    fn parse_call_statement(&mut self) -> Result<Statement> {
        self.consume_token(Token::Call)?;
        let recipe = self.parse_expression()?;

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

        Ok(Statement::Call { recipe, args })
    }

    fn parse_assign_statement(&mut self, name: String) -> Result<Statement> {
        self.advance(); // consume identifier

        if self.match_token(&Token::Equals) {
            let value = self.parse_expression()?;
            Ok(Statement::Assign { name, value })
        } else {
            Err(self.create_error("assignment".to_string(), self.peek().clone()))
        }
    }

    fn parse_let_statement(&mut self) -> Result<Statement> {
        self.consume_token(Token::Let)?;
        let name = self.parse_identifier()?;
        self.consume_token(Token::Colon)?;
        let param_type = self.parse_param_type()?;
        let mut value = None;

        if self.match_token(&Token::Equals) {
            value = Some(self.parse_expression()?);
        }

        Ok(Statement::Let {
            name,
            value,
            param_type,
        })
    }

    fn parse_print_statement(&mut self) -> Result<Statement> {
        self.consume_token(Token::Print)?;
        let expr = self.parse_expression()?;
        Ok(Statement::Print(expr))
    }

    fn parse_exit_statement(&mut self) -> Result<Statement> {
        self.consume_token(Token::Exit)?;
        let expr = self.parse_expression()?;
        Ok(Statement::Exit(expr))
    }

    fn parse_run_statement(&mut self) -> Result<Statement> {
        self.consume_token(Token::Run)?;
        let expr = self.parse_expression()?;
        Ok(Statement::Run(expr))
    }

    fn parse_if_statement(&mut self) -> Result<Statement> {
        self.consume_token(Token::If)?;
        let condition = self.parse_expression()?;
        self.consume_token(Token::LeftBrace)?;

        let then_block = self.parse_statement_block()?;

        let else_block = if self.match_token(&Token::Else) {
            if self.match_token(&Token::If) {
                // else if => nested if
                let else_if_condition = self.parse_expression()?;
                self.consume_token(Token::LeftBrace)?;
                let else_if_then = self.parse_statement_block()?;

                let nested_if = Statement::If {
                    condition: else_if_condition,
                    then_block: else_if_then,
                    else_block: if self.match_token(&Token::Else) {
                        self.consume_token(Token::LeftBrace)?;
                        Some(self.parse_statement_block()?)
                    } else {
                        None
                    },
                };

                let nested_span = self.get_current_span();
                Some(vec![SpannedNode::new(nested_if, nested_span)])
            } else {
                self.consume_token(Token::LeftBrace)?;
                Some(self.parse_statement_block()?)
            }
        } else {
            None
        };

        Ok(Statement::If {
            condition,
            then_block,
            else_block,
        })
    }

    fn parse_for_statement(&mut self) -> Result<Statement> {
        self.consume_token(Token::For)?;
        let var = self.parse_identifier()?;
        self.consume_token(Token::In)?;
        let iterable = self.parse_expression()?;
        self.consume_token(Token::LeftBrace)?;

        let body = self.parse_statement_block()?;

        let is_async = self.match_token(&Token::Async);

        Ok(Statement::For {
            var,
            iterable,
            body,
            is_async,
        })
    }

    pub fn parse_match_statement(&mut self) -> Result<Statement> {
        self.consume_token(Token::Match)?;
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

            let body = if self.check(&Token::LeftBrace) {
                self.consume_token(Token::LeftBrace)?;
                self.parse_statement_block()?
            } else {
                vec![self.parse_statement()?]
            };

            let arm_end = self.current;
            let arm_span = self.span_from_token_range(arm_start, arm_end);

            let arm = MatchArm {
                pattern,
                guard,
                body,
                bindings: HashMap::new(),
            };
            arms.push(SpannedNode::new(arm, arm_span));

            if !self.check(&Token::RightBrace) {
                self.consume_token(Token::Comma)?;
            }
        }

        self.consume_token(Token::RightBrace)?;

        Ok(Statement::Match { expr, arms })
    }
}

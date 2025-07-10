use ::lexer::Token;
use core::{BraiseType, ast::*, error::parser::Result};
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
            Token::Try => self.parse_try_statement()?,
            Token::Throw => self.parse_throw_statement()?,
            e => {
                return Err(self
                    .create_error("statement".to_string(), e.clone())
                    .boxed());
            }
        };

        let end_token = self.current;
        let span = self.span_from_token_range(start_token, end_token);

        Ok(SpannedNode::new(statement, span))
    }

    fn parse_try_statement(&mut self) -> Result<Statement> {
        self.consume_token(Token::Try)?;
        self.consume_token(Token::LeftBrace)?;

        let try_block = self.parse_statement_block()?;

        let catch_block = if self.match_token(&Token::Catch) {
            let error_var = if self.check_identifier() {
                Some(self.parse_identifier()?)
            } else {
                None
            };

            self.consume_token(Token::LeftBrace)?;
            let catch_body = self.parse_statement_block()?;

            Some(CatchBlock::new(error_var, catch_body))
        } else {
            None
        };

        let finally_block = if self.match_token(&Token::Finally) {
            self.consume_token(Token::LeftBrace)?;
            Some(self.parse_statement_block()?)
        } else {
            None
        };

        if catch_block.is_none() && finally_block.is_none() {
            return Err(self
                .create_error("catch or finally block".to_string(), self.peek().clone())
                .boxed());
        }

        Ok(Statement::Try {
            try_block,
            catch_block,
            finally_block,
        })
    }

    /// Parse throw statement
    fn parse_throw_statement(&mut self) -> Result<Statement> {
        self.consume_token(Token::Throw)?;
        let expr = self.parse_expression()?;
        Ok(Statement::Throw(expr))
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
        self.advance();

        if self.match_token(&Token::Equals) {
            let value = self.parse_expression()?;
            Ok(Statement::Assign { name, value })
        } else {
            Err(self
                .create_error("assignment".to_string(), self.peek().clone())
                .boxed())
        }
    }

    /// Parse let statement with enhanced type support
    fn parse_let_statement(&mut self) -> Result<Statement> {
        self.consume_token(Token::Let)?;
        let name = self.parse_identifier()?;

        let declared_type = self.parse_type_annotation()?.unwrap_or(BraiseType::Any);

        let mut value = None;
        let mut inferred_type = None;

        if self.match_token(&Token::Equals) {
            let value_expr = self.parse_expression()?;

            if self.enable_type_checking {
                let expr_type = self.infer_expression_type(&value_expr.value);

                if !matches!(declared_type, BraiseType::Any)
                    && !expr_type.is_compatible_with(&declared_type)
                {
                    return Err(self
                        .create_type_error(
                            declared_type.to_string(),
                            expr_type.to_string(),
                            format!("let statement for variable '{name}'"),
                            &value_expr.span,
                        )
                        .boxed());
                }

                inferred_type = Some(expr_type);
            }

            value = Some(value_expr);
        } else if matches!(declared_type, BraiseType::Any) {
            return Err(self
                .create_error(
                    "type annotation or initial value".to_string(),
                    self.peek().clone(),
                )
                .boxed());
        }

        let final_type = if !matches!(declared_type, BraiseType::Any) {
            declared_type
        } else {
            inferred_type.clone().unwrap_or(BraiseType::Any)
        };

        Ok(Statement::Let {
            name,
            value,
            param_type: final_type,
            inferred_type,
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

    /// Parse match statement with enhanced pattern support
    pub fn parse_match_statement(&mut self) -> Result<Statement> {
        self.consume_token(Token::Match)?;
        let expr = self.parse_expression()?;
        self.consume_token(Token::LeftBrace)?;

        let mut arms = Vec::new();

        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            let arm_start = self.current;

            let pattern = self.parse_match_pattern()?;

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

            let bindings = Self::extract_pattern_bindings(&pattern)?;

            let arm = MatchArm {
                pattern,
                guard,
                body,
                bindings,
            };
            arms.push(SpannedNode::new(arm, arm_span));

            if !self.check(&Token::RightBrace) {
                self.consume_token(Token::Comma)?;
            }
        }

        self.consume_token(Token::RightBrace)?;

        Ok(Statement::Match { expr, arms })
    }

    /// Extract variable bindings from a match pattern with their types
    fn extract_pattern_bindings(pattern: &MatchPattern) -> Result<HashMap<String, BraiseType>> {
        let mut bindings = HashMap::new();

        match pattern {
            MatchPattern::Variable(name) => {
                bindings.insert(name.clone(), BraiseType::Any);
            }
            MatchPattern::Array { elements } => {
                for element in elements {
                    match &element.value {
                        ArrayPatternElement::Pattern(inner_pattern) => {
                            let inner_bindings = Self::extract_pattern_bindings(inner_pattern)?;
                            bindings.extend(inner_bindings);
                        }
                        ArrayPatternElement::Rest(Some(name)) => {
                            bindings
                                .insert(name.clone(), BraiseType::Array(Box::new(BraiseType::Any)));
                        }
                        _ => {}
                    }
                }
            }
            MatchPattern::Or(patterns) => {
                for pattern_node in patterns {
                    let inner_bindings = Self::extract_pattern_bindings(&pattern_node.value)?;
                    bindings.extend(inner_bindings);
                }
            }
            MatchPattern::Guard { pattern, .. } => {
                let inner_bindings = Self::extract_pattern_bindings(&pattern.value)?;
                bindings.extend(inner_bindings);
            }
            _ => {}
        }

        Ok(bindings)
    }

    /// Validate statement in current type context
    pub fn validate_statement_types(&mut self, statement: &mut Statement) -> Result<()> {
        if !self.enable_type_checking {
            return Ok(());
        }

        match statement {
            Statement::Let {
                name,
                value: Some(value),
                param_type,
                inferred_type,
            } => {
                let expr_type = self.infer_expression_type(&value.value);

                if !expr_type.is_compatible_with(param_type) {
                    return Err(self
                        .create_type_error(
                            param_type.to_string(),
                            expr_type.to_string(),
                            format!("variable declaration '{name}'"),
                            &value.span,
                        )
                        .boxed());
                }

                *inferred_type = Some(expr_type);
            }
            Statement::Assign { name, value } => {
                if let Some(var_type) = self.type_engine.get_variable_type(name) {
                    let var_type_clone = var_type.clone();
                    let expr_type = self.infer_expression_type(&value.value);

                    if !expr_type.is_compatible_with(&var_type_clone) {
                        return Err(self
                            .create_type_error(
                                var_type_clone.to_string(),
                                expr_type.to_string(),
                                format!("assignment to variable '{name}'"),
                                &value.span,
                            )
                            .boxed());
                    }
                } else {
                    return Err(self
                        .create_undefined_variable_error(name, &value.span)
                        .boxed());
                }
            }
            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::Result;
    use std::assert_matches::assert_matches;

    use super::*;
    use braise_errors::ParserError;
    use lexer::tokenize;

    fn parse_statement_string(input: &str) -> Result<Statement, Box<ParserError>> {
        let tokens = tokenize(input).unwrap();
        let mut parser = Parser::new(tokens, input.to_string().into(), "test.braise".to_string());
        let stmt = parser.parse_statement()?;
        Ok(stmt.value)
    }

    #[test]
    fn test_let_with_type_annotation() {
        let stmt = parse_statement_string("let name: string = \"hello\"").unwrap();

        if let Statement::Let {
            name,
            param_type,
            value,
            ..
        } = stmt
        {
            assert_eq!(name, "name");
            assert_eq!(param_type, BraiseType::String);
            assert!(value.is_some());
        } else {
            panic!("Expected let statement");
        }
    }

    #[test]
    fn test_let_with_type_inference() {
        let stmt = parse_statement_string("let count = 42").unwrap();

        if let Statement::Let {
            name, param_type, ..
        } = stmt
        {
            assert_eq!(name, "count");
            assert_eq!(param_type, BraiseType::Number);
        } else {
            panic!("Expected let statement");
        }
    }

    #[test]
    fn test_let_with_union_type() {
        let stmt = parse_statement_string("let value: string | number = \"hello\"").unwrap();

        if let Statement::Let {
            name, param_type, ..
        } = stmt
        {
            assert_eq!(name, "value");
            if let BraiseType::Union(types) = param_type {
                assert!(types.contains(&BraiseType::String));
                assert!(types.contains(&BraiseType::Number));
            } else {
                panic!("Expected union type");
            }
        } else {
            panic!("Expected let statement");
        }
    }

    #[test]
    fn test_let_with_optional_type() {
        let stmt = parse_statement_string("let maybe: string?").unwrap();

        if let Statement::Let {
            name, param_type, ..
        } = stmt
        {
            assert_eq!(name, "maybe");
            if let BraiseType::Optional(inner) = param_type {
                assert_eq!(*inner, BraiseType::String);
            } else {
                panic!("Expected optional type");
            }
        } else {
            panic!("Expected let statement");
        }
    }

    #[test]
    fn test_assignment_statement() {
        let stmt = parse_statement_string("existing = \"new value\"").unwrap();

        if let Statement::Assign { name, value } = stmt {
            assert_eq!(name, "existing");
            assert!(matches!(value.value, Expression::String(_)));
        } else {
            panic!("Expected assignment statement");
        }
    }

    #[test]
    fn test_match_statement_with_patterns() {
        let input = r#"
        match value {
            "option1" => { print "one" },
            "option2" => { print "two" },
            x if x > 10 => { print "large" },
            _ => { print "default" }
        }
        "#;

        let tokens = tokenize(input).unwrap();
        let mut parser = Parser::new(tokens, input.to_string().into(), "test.braise".to_string());
        let stmt = parser.parse_statement().unwrap();

        if let Statement::Match { arms, .. } = stmt.value {
            assert_eq!(arms.len(), 4);

            assert!(matches!(arms[0].value.pattern, MatchPattern::String(_)));
            assert!(matches!(arms[1].value.pattern, MatchPattern::String(_)));
            assert!(matches!(arms[2].value.pattern, MatchPattern::Guard { .. }));
            assert!(matches!(arms[3].value.pattern, MatchPattern::Wildcard));
        } else {
            panic!("Expected match statement");
        }
    }

    #[test]
    fn test_for_statement() {
        let stmt = parse_statement_string(r#"for item in items { print item }"#).unwrap();

        if let Statement::For {
            var,
            iterable,
            body,
            is_async,
        } = stmt
        {
            assert_eq!(var, "item");
            assert!(matches!(iterable.value, Expression::Variable(_)));
            assert_eq!(body.len(), 1);
            assert!(!is_async);
        } else {
            panic!("Expected for statement");
        }
    }

    #[test]
    fn test_if_statement() {
        let stmt =
            parse_statement_string(r#"if condition { print "true" } else { print "false" }"#)
                .unwrap();

        if let Statement::If {
            condition,
            then_block,
            else_block,
        } = stmt
        {
            assert!(matches!(condition.value, Expression::Variable(_)));
            assert_eq!(then_block.len(), 1);
            assert!(else_block.is_some());
            assert_eq!(else_block.unwrap().len(), 1);
        } else {
            panic!("Expected if statement");
        }
    }

    #[test]
    fn test_complete_recipe_with_types() -> miette::Result<()> {
        let input = r#"
        recipe "typed_example" {
            param name: string | number = "default"
            param count: number?
            param items: [string] = ["a", "b"]
            
            let message: string = match name {
                n if n > 10 => "large: ${n}",
                s => "value: ${s}"
            }
            
            for item in items {
                print "${item}: ${message}"
            }
            
            if count {
                print "Count is ${count}"
            }
        }
        "#;

        let tokens = tokenize(input)?;
        let mut parser = Parser::new(tokens, input.to_string().into(), "test.braise".to_string());
        let config = parser.parse();

        let config = config.map_err(|e| miette::miette!(e.to_string()))?;
        assert_eq!(config.recipes.len(), 1);

        let recipe = &config.recipes[0].value;

        assert_eq!(recipe.parameters.len(), 3);

        assert_matches!(&recipe.parameters[0].value.param_type,
            BraiseType::Optional(inner) if matches!(&**inner, BraiseType::Union(types) if types.contains(&BraiseType::String) && types.contains(&BraiseType::Number)
        ));

        assert!(recipe.parameters[1].value.optional);

        assert_matches!(
            &recipe.parameters[2].value.param_type,
            BraiseType::Optional(arr) if matches!(&**arr, BraiseType::Array(inner) if matches!(&**inner, BraiseType::String)
        ));

        assert_eq!(recipe.body.len(), 3); // let, for, if
        Ok(())
    }
}

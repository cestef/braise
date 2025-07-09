use ::lexer::{InterpolationSegment, Token, lex_interpolated_string};
use core::{BraiseType, ast::*, error::parser::Result};
use std::collections::HashMap;

use crate::Parser;

impl Parser {
    pub fn parse_expression(&mut self) -> Result<SpannedNode<Expression>> {
        let start_token = self.current;
        let expr = self.parse_conditional()?;
        let end_token = self.current;
        let span = self.span_from_token_range(start_token, end_token);

        let spanned_expr = SpannedNode::new(expr, span);

        Ok(spanned_expr)
    }
    #[tracing::instrument(skip(self))]
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

            let then_type = self.infer_expression_type(&then_expr.value);
            let else_type = self.infer_expression_type(&else_expr.value);
            let result_type = then_type.common_type(&else_type);

            Ok(Expression::Conditional {
                condition: Box::new(SpannedNode::new(condition, condition_span)),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
                result_type: Some(result_type),
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
                result_type: Some(BraiseType::Bool),
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
                result_type: Some(BraiseType::Bool),
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
                result_type: Some(BraiseType::Bool),
            };
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expression> {
        let mut expr = self.parse_add_sub()?;

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

            let right = self.parse_add_sub()?;
            let span = self.get_current_span();

            expr = Expression::BinaryOp {
                left: Box::new(SpannedNode::new(expr, span.clone())),
                op,
                right: Box::new(SpannedNode::new(right, span)),
                result_type: Some(BraiseType::Bool),
            };
        }

        Ok(expr)
    }

    fn parse_add_sub(&mut self) -> Result<Expression> {
        let mut expr = self.parse_mul_div_mod()?;

        while let Token::Plus | Token::Minus = self.peek() {
            let op = match self.peek() {
                Token::Plus => {
                    self.advance();
                    BinaryOperator::Plus
                }
                Token::Minus => {
                    self.advance();
                    BinaryOperator::Minus
                }
                _ => break,
            };

            let right = self.parse_mul_div_mod()?;
            let span = self.get_current_span();

            expr = Expression::BinaryOp {
                left: Box::new(SpannedNode::new(expr, span.clone())),
                op,
                right: Box::new(SpannedNode::new(right, span)),
                result_type: Some(BraiseType::Number),
            };
        }

        Ok(expr)
    }

    fn parse_mul_div_mod(&mut self) -> Result<Expression> {
        let mut expr = self.parse_unary()?;

        while let Token::Multiply | Token::Divide | Token::Modulus = self.peek() {
            let op = match self.peek() {
                Token::Multiply => {
                    self.advance();
                    BinaryOperator::Multiply
                }
                Token::Divide => {
                    self.advance();
                    BinaryOperator::Divide
                }
                Token::Modulus => {
                    self.advance();
                    BinaryOperator::Modulus
                }
                _ => break,
            };

            let right = self.parse_unary()?;
            let span = self.get_current_span();

            expr = Expression::BinaryOp {
                left: Box::new(SpannedNode::new(expr, span.clone())),
                op,
                right: Box::new(SpannedNode::new(right, span)),
                result_type: Some(BraiseType::Number),
            };
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expression> {
        match self.peek() {
            Token::Bang => {
                self.advance();
                let expr = self.parse_expression()?;
                Ok(Expression::UnaryOp {
                    op: UnaryOperator::Not,
                    expr: Box::new(expr),
                    result_type: Some(BraiseType::Bool),
                })
            }
            Token::Minus => {
                self.advance();
                let expr = self.parse_expression()?;
                Ok(Expression::UnaryOp {
                    op: UnaryOperator::Minus,
                    expr: Box::new(expr),
                    result_type: Some(BraiseType::Number),
                })
            }
            _ => self.parse_exponent(),
        }
    }

    fn parse_exponent(&mut self) -> Result<Expression> {
        let mut expr = self.parse_primary()?;

        while self.match_token(&Token::Exponent) {
            let right = self.parse_primary()?;
            let span = self.get_current_span();

            expr = Expression::BinaryOp {
                left: Box::new(SpannedNode::new(expr, span.clone())),
                op: BinaryOperator::Exponent,
                right: Box::new(SpannedNode::new(right, span)),
                result_type: Some(BraiseType::Number),
            };
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expression> {
        match self.peek().clone() {
            Token::String(s) => {
                self.advance();

                if s.contains("${") {
                    Ok(Expression::Interpolation(self.parse_interpolation(&s)?))
                } else {
                    Ok(Expression::String(s))
                }
            }
            Token::SingleQuotedString(s) => {
                self.advance();
                Ok(Expression::String(s))
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

                        let return_type = Some(self.get_builtin_function_type(&name, &field));

                        Ok(Expression::FunctionCall {
                            module: name,
                            function: field,
                            args,
                            return_type,
                        })
                    } else {
                        let field_type = Some(self.get_builtin_field_type(&name, &field));

                        Ok(Expression::ModuleAccess {
                            module: name,
                            field,
                            field_type,
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
            Token::LeftParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.consume_token(Token::RightParen)?;
                Ok(expr.value)
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

    /// Parse match expression with enhanced type inference
    pub fn parse_match_expression(&mut self) -> Result<Expression> {
        self.advance();
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

            let body_expr = self.parse_expression()?;

            let arm_end = self.current;
            let arm_span = self.span_from_token_range(arm_start, arm_end);

            let bindings = self.extract_pattern_bindings_for_expression(&pattern)?;

            let arm = MatchExpressionArm {
                pattern,
                guard,
                expr: body_expr,
                bindings,
            };
            let spanned = SpannedNode::new(arm, arm_span);
            arms.push(spanned);

            if !self.check(&Token::RightBrace) {
                self.consume_token(Token::Comma)?;
            }
        }

        self.consume_token(Token::RightBrace)?;

        let result_type = if !arms.is_empty() {
            let first_type = self.infer_expression_type(&arms[0].value.expr.value);
            let common_type = arms.iter().skip(1).fold(first_type, |acc, arm| {
                let arm_type = self.infer_expression_type(&arm.value.expr.value);
                acc.common_type(&arm_type)
            });
            Some(common_type)
        } else {
            Some(BraiseType::Any)
        };

        Ok(Expression::Match {
            expr: Box::new(expr),
            arms,
            result_type,
        })
    }

    /// Extract pattern bindings for expression match arms
    fn extract_pattern_bindings_for_expression(
        &self,
        pattern: &MatchPattern,
    ) -> Result<HashMap<String, BraiseType>> {
        let mut bindings = HashMap::new();

        match pattern {
            MatchPattern::Variable(name) => {
                bindings.insert(name.clone(), BraiseType::Any);
            }
            MatchPattern::Array { elements, rest } => {
                for element in elements {
                    match &element.value {
                        ArrayPatternElement::Pattern(inner_pattern) => {
                            let inner_bindings =
                                self.extract_pattern_bindings_for_expression(inner_pattern)?;
                            bindings.extend(inner_bindings);
                        }
                        ArrayPatternElement::Rest(Some(name)) => {
                            bindings
                                .insert(name.clone(), BraiseType::Array(Box::new(BraiseType::Any)));
                        }
                        _ => {}
                    }
                }
                if let Some(rest_name) = rest {
                    bindings.insert(
                        rest_name.clone(),
                        BraiseType::Array(Box::new(BraiseType::Any)),
                    );
                }
            }
            MatchPattern::Or(patterns) => {
                for pattern_node in patterns {
                    let inner_bindings =
                        self.extract_pattern_bindings_for_expression(&pattern_node.value)?;
                    bindings.extend(inner_bindings);
                }
            }
            MatchPattern::Guard { pattern, .. } => {
                let inner_bindings =
                    self.extract_pattern_bindings_for_expression(&pattern.value)?;
                bindings.extend(inner_bindings);
            }
            _ => {}
        }

        Ok(bindings)
    }

    /// Parse interpolated string using recursive lexing
    fn parse_interpolation(&mut self, s: &str) -> Result<InterpolatedStringExpr> {
        let current_span = self.get_current_span();
        let interpolated = lex_interpolated_string(s, &self.source, current_span.start.offset)
            .map_err(|_err| {
                self.create_error(
                    "interpolation parsing".to_string(),
                    Token::String(s.to_string()),
                )
            })?;

        let mut parts = Vec::new();

        for segment in interpolated.segments {
            match segment {
                InterpolationSegment::String(string_part) => {
                    parts.push(InterpolationPart::String(string_part));
                }
                InterpolationSegment::Expression(tokens) => {
                    // Create a new parser for the expression tokens
                    let mut expr_parser = Parser::new(
                        tokens,
                        self.source.clone(),
                        format!("interpolation_{}", current_span.start.offset),
                    );
                    expr_parser.enable_type_checking = self.enable_type_checking;

                    // Parse the expression
                    let expr = expr_parser.parse_expression()?;
                    parts.push(InterpolationPart::Expression(expr));
                }
            }
        }

        Ok(InterpolatedStringExpr {
            parts,
            is_recursive: true,
        })
    }

    /// Validate expression types after parsing
    pub fn validate_expression_types(&mut self, expr: &mut Expression) -> Result<()> {
        if !self.enable_type_checking {
            return Ok(());
        }

        match expr {
            Expression::BinaryOp {
                left, right, op, ..
            } => {
                self.validate_expression_types(&mut left.value)?;
                self.validate_expression_types(&mut right.value)?;

                let left_type = self.infer_expression_type(&left.value);
                let right_type = self.infer_expression_type(&right.value);

                match op {
                    BinaryOperator::Less
                    | BinaryOperator::LessEqual
                    | BinaryOperator::Greater
                    | BinaryOperator::GreaterEqual => {
                        if !matches!(left_type, BraiseType::Number | BraiseType::Any)
                            || !matches!(right_type, BraiseType::Number | BraiseType::Any)
                        {
                            return Err(self.create_type_error(
                                "number".to_string(),
                                format!("{left_type} and {right_type}"),
                                "comparison operation".to_string(),
                                &left.span,
                            ));
                        }
                    }
                    _ => {}
                }
            }

            Expression::UnaryOp { expr, .. } => {
                self.validate_expression_types(&mut expr.value)?;
            }

            Expression::Array(elements) => {
                for element in elements {
                    self.validate_expression_types(&mut element.value)?;
                }
            }

            Expression::FunctionCall { args, .. } => {
                for arg in args {
                    self.validate_expression_types(&mut arg.value)?;
                }
            }

            Expression::Conditional {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                self.validate_expression_types(&mut condition.value)?;
                self.validate_expression_types(&mut then_expr.value)?;
                self.validate_expression_types(&mut else_expr.value)?;

                let condition_type = self.infer_expression_type(&condition.value);
                if !condition_type.can_convert_from(&BraiseType::Bool)
                    && !matches!(condition_type, BraiseType::Bool | BraiseType::Any)
                {
                    return Err(self.create_type_error(
                        "boolean or boolean-convertible".to_string(),
                        condition_type.to_string(),
                        "conditional expression condition".to_string(),
                        &condition.span,
                    ));
                }
            }

            Expression::Match { expr, arms, .. } => {
                self.validate_expression_types(&mut expr.value)?;

                for arm in arms {
                    self.validate_expression_types(&mut arm.value.expr.value)?;
                    if let Some(ref mut guard) = arm.value.guard {
                        self.validate_expression_types(&mut guard.value)?;
                    }
                }
            }

            Expression::Interpolation(interpolated) => {
                for part in &mut interpolated.parts {
                    if let InterpolationPart::Expression(expr) = part {
                        self.validate_expression_types(&mut expr.value)?;
                    }
                }
            }

            _ => {}
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lexer::tokenize;

    fn parse_expression_string(input: &str) -> Result<Expression> {
        let tokens = tokenize(input).unwrap();
        let mut parser = Parser::new(tokens, input.to_string().into(), "test.braise".to_string());
        let expr = parser.parse_expression()?;
        Ok(expr.value)
    }

    #[test]
    fn test_conditional_expression() {
        let expr = parse_expression_string("if true { \"yes\" } else { \"no\" }").unwrap();

        if let Expression::Conditional {
            condition,
            then_expr,
            else_expr,
            result_type,
        } = expr
        {
            assert!(matches!(condition.value, Expression::Bool(true)));
            assert!(matches!(then_expr.value, Expression::String(_)));
            assert!(matches!(else_expr.value, Expression::String(_)));
            assert_eq!(result_type, Some(BraiseType::String));
        } else {
            panic!("Expected conditional expression");
        }
    }

    #[test]
    fn test_binary_operations_with_types() {
        let expr = parse_expression_string("5 > 3").unwrap();

        if let Expression::BinaryOp {
            left,
            op,
            right,
            result_type,
        } = expr
        {
            assert!(matches!(left.value, Expression::Number(5.0)));
            assert_eq!(op, BinaryOperator::Greater);
            assert!(matches!(right.value, Expression::Number(3.0)));
            assert_eq!(result_type, Some(BraiseType::Bool));
        } else {
            panic!("Expected binary operation");
        }
    }

    #[test]
    fn test_function_call_with_return_type() {
        let expr = parse_expression_string("cpu.count()").unwrap();

        if let Expression::FunctionCall {
            module,
            function,
            return_type,
            ..
        } = expr
        {
            assert_eq!(module, "cpu");
            assert_eq!(function, "count");
            assert_eq!(return_type, Some(BraiseType::Number));
        } else {
            panic!("Expected function call");
        }
    }

    #[test]
    fn test_module_access_with_field_type() {
        let expr = parse_expression_string("env.HOME").unwrap();

        if let Expression::ModuleAccess {
            module,
            field,
            field_type,
        } = expr
        {
            assert_eq!(module, "env");
            assert_eq!(field, "HOME");
            assert_eq!(field_type, Some(BraiseType::String));
        } else {
            panic!("Expected module access");
        }
    }

    #[test]
    fn test_array_expression() {
        let expr = parse_expression_string(r#"["a", "b", "c"]"#).unwrap();

        if let Expression::Array(elements) = expr {
            assert_eq!(elements.len(), 3);
            assert!(matches!(elements[0].value, Expression::String(_)));
            assert!(matches!(elements[1].value, Expression::String(_)));
            assert!(matches!(elements[2].value, Expression::String(_)));
        } else {
            panic!("Expected array expression");
        }
    }

    #[test]
    fn test_match_expression_with_types() {
        let expr = parse_expression_string(
            r#"
        match value {
            "a" => 1,
            "b" => 2,
            _ => 0
        }
        "#,
        )
        .unwrap();

        if let Expression::Match {
            expr,
            arms,
            result_type,
        } = expr
        {
            assert!(matches!(expr.value, Expression::Variable(_)));
            assert_eq!(arms.len(), 3);
            assert!(result_type.is_some());
        } else {
            panic!("Expected match expression");
        }
    }

    #[test]
    fn test_string_interpolation() {
        let expr = parse_expression_string(r#""Hello ${name}!""#).unwrap();

        if let Expression::Interpolation(interpolated) = expr {
            assert_eq!(interpolated.parts.len(), 3);
            assert!(matches!(
                interpolated.parts[0],
                InterpolationPart::String(_)
            ));
            assert!(matches!(
                interpolated.parts[1],
                InterpolationPart::Expression(_)
            ));
            assert!(matches!(
                interpolated.parts[2],
                InterpolationPart::String(_)
            ));
            assert!(interpolated.is_recursive);
        } else {
            panic!("Expected interpolation expression");
        }
    }

    #[test]
    fn test_unary_operation_with_type() {
        let expr = parse_expression_string("!true").unwrap();

        if let Expression::UnaryOp {
            op,
            expr,
            result_type,
        } = expr
        {
            assert_eq!(op, UnaryOperator::Not);
            assert!(matches!(expr.value, Expression::Bool(true)));
            assert_eq!(result_type, Some(BraiseType::Bool));
        } else {
            panic!("Expected unary operation");
        }
    }

    #[test]
    fn test_recipe_reference() {
        let expr = parse_expression_string(r#"@other(param1: "value", param2: 42)"#).unwrap();

        if let Expression::RecipeRef { recipe, args } = expr {
            assert_eq!(recipe, "other");
            assert_eq!(args.len(), 2);
            assert!(args.contains_key("param1"));
            assert!(args.contains_key("param2"));
        } else {
            panic!("Expected recipe reference");
        }
    }
}

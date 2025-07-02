use ::lexer::{SpannedToken, Token};
use core::{
    error::parser::{ParseError, Result},
    *,
};
use miette::SourceSpan;
use std::rc::Rc;

pub struct Parser {
    tokens: Vec<SpannedToken>,
    source: Rc<String>,
    // source_map: SourceMap,
    pub current: usize,
    file_id: FileId,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>, source: String, filename: String) -> Self {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        filename.hash(&mut hasher);
        let file_id = FileId(hasher.finish() as usize);
        // let source_map = SourceMap::new(source.clone(), file_id);

        Parser {
            tokens,
            current: 0,
            source: Rc::new(source),
            // source_map,
            file_id,
        }
    }

    fn create_error(&self, expected: String, found: Token) -> ParseError {
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

        ParseError::UnexpectedToken {
            expected,
            found: found.to_string(),
            code,
            span,
        }
    }

    fn get_current_span(&self) -> Span {
        if self.current < self.tokens.len() {
            let token_span = &self.tokens[self.current].span;
            Span::from_token_span(token_span, self.file_id)
        } else {
            // EOF span
            let end_pos = Position::new(1, 1, self.source.len());
            Span::new(end_pos, end_pos, self.file_id)
        }
    }

    fn span_from_token_range(&self, start_token: usize, end_token: usize) -> Span {
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

    pub fn parse(&mut self) -> Result<Config> {
        let mut recipes = Vec::new();
        let start_token = self.current;

        while !self.is_at_end() {
            recipes.push(self.parse_recipe()?);
        }

        let end_token = self.current;
        let span = self.span_from_token_range(start_token, end_token.max(1));

        Ok(Config { recipes, span })
    }

    fn parse_recipe(&mut self) -> Result<SpannedNode<Recipe>> {
        let start_token = self.current;
        self.consume_token(Token::Recipe)?;

        let name = self.parse_string()?;

        let dependencies = if self.match_token(&Token::Arrow) {
            self.consume_token(Token::LeftBracket)?;
            let mut deps = Vec::new();

            if !self.check(&Token::RightBracket) {
                loop {
                    deps.push(self.parse_string()?);
                    if !self.match_token(&Token::Comma) {
                        break;
                    }
                }
            }

            self.consume_token(Token::RightBracket)?;
            deps
        } else {
            Vec::new()
        };

        self.consume_token(Token::LeftBrace)?;

        let mut parameters = Vec::new();
        let mut body = Vec::new();

        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            if self.check(&Token::Param) {
                parameters.push(self.parse_parameter()?);
            } else {
                body.push(self.parse_statement()?);
            }
        }

        self.consume_token(Token::RightBrace)?;
        let end_token = self.current;

        let recipe = Recipe {
            name,
            dependencies,
            parameters,
            body,
        };

        let span = self.span_from_token_range(start_token, end_token);
        Ok(SpannedNode::new(recipe, span))
    }

    fn parse_statement(&mut self) -> Result<SpannedNode<Statement>> {
        let start_token = self.current;

        let statement = match self.peek() {
            Token::Run => self.parse_run_statement()?,
            Token::Exit => self.parse_exit_statement()?,
            Token::Print => self.parse_print_statement()?,
            Token::If => self.parse_if_statement()?,
            Token::Match => self.parse_match_statement()?,
            Token::For => self.parse_for_statement()?,
            e => return Err(self.create_error("statement".to_string(), e.clone())),
        };

        let end_token = self.current;
        let span = self.span_from_token_range(start_token, end_token);

        Ok(SpannedNode::new(statement, span))
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

    fn parse_match_statement(&mut self) -> Result<Statement> {
        self.consume_token(Token::Match)?;
        let expr = self.parse_expression()?;
        self.consume_token(Token::LeftBrace)?;

        let mut arms = Vec::new();

        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            let arm_start = self.current;

            let pattern = if self.check(&Token::String(String::new())) {
                MatchPattern::String(self.parse_string()?)
            } else if self.match_token(&Token::Identifier("_".to_string())) {
                MatchPattern::Wildcard
            } else {
                return Err(self.create_error("match pattern".to_string(), self.peek().clone()));
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
            let arm = MatchArm { pattern, body };
            arms.push(SpannedNode::new(arm, arm_span));

            if !self.check(&Token::RightBrace) {
                self.consume_token(Token::Comma)?;
            }
        }

        self.consume_token(Token::RightBrace)?;

        Ok(Statement::Match { expr, arms })
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

    fn parse_statement_block(&mut self) -> Result<Vec<SpannedNode<Statement>>> {
        let mut statements = Vec::new();

        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            statements.push(self.parse_statement()?);
        }

        self.consume_token(Token::RightBrace)?;
        Ok(statements)
    }

    fn parse_expression(&mut self) -> Result<SpannedNode<Expression>> {
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

            _ => Err(self.create_error("expression".to_string(), self.peek().clone())),
        }
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

                while let Some(ch) = chars.next() {
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

    fn parse_parameter(&mut self) -> Result<SpannedNode<Parameter>> {
        let start_token = self.current;

        self.consume_token(Token::Param)?;

        let name = self.parse_identifier()?;
        self.consume_token(Token::Colon)?;

        let param_type = self.parse_param_type()?;

        let default = if self.match_token(&Token::Equals) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        let end_token = self.current;
        let span = self.span_from_token_range(start_token, end_token);

        let parameter = Parameter {
            name,
            param_type,
            default,
        };

        Ok(SpannedNode::new(parameter, span))
    }

    fn parse_param_type(&mut self) -> Result<ParamType> {
        match self.peek() {
            Token::Identifier(name) => {
                let name_clone = name.clone();
                self.advance();
                match name_clone.as_str() {
                    "string" => Ok(ParamType::String),
                    "int" | "number" => Ok(ParamType::Number),
                    "bool" => Ok(ParamType::Bool),
                    _ => Err(ParseError::Other(format!("Unknown type: {}", name_clone))),
                }
            }
            Token::LeftBracket => {
                self.advance();

                // is an enum [value1, value2, ...] ?
                if self.check(&Token::String(String::new())) {
                    let mut values = Vec::new();

                    loop {
                        values.push(self.parse_string()?);
                        if !self.match_token(&Token::Comma) {
                            break;
                        }
                    }

                    self.consume_token(Token::RightBracket)?;
                    Ok(ParamType::Enum(values))
                } else {
                    // array [element_type]
                    let element_type = self.parse_param_type()?;
                    self.consume_token(Token::RightBracket)?;
                    Ok(ParamType::Array(Box::new(element_type)))
                }
            }
            _ => Err(self.create_error("parameter type".to_string(), self.peek().clone())),
        }
    }

    fn parse_identifier(&mut self) -> Result<String> {
        match self.peek() {
            Token::Identifier(s) => {
                let result = s.clone();
                self.advance();
                Ok(result)
            }
            _ => Err(self.create_error("identifier".to_string(), self.peek().clone())),
        }
    }

    fn parse_string(&mut self) -> Result<String> {
        match self.peek() {
            Token::String(s) => {
                let result = s.clone();
                self.advance();
                Ok(result)
            }
            _ => Err(self.create_error("string".to_string(), self.peek().clone())),
        }
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.tokens.len()
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.current)
            .map(|e| &e.token)
            .unwrap_or(&Token::EOF)
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn previous(&self) -> &Token {
        if self.current > 0 && self.current <= self.tokens.len() {
            &self.tokens[self.current - 1].token
        } else {
            &Token::EOF
        }
    }

    fn check(&self, token_type: &Token) -> bool {
        if self.is_at_end() {
            return false;
        }
        std::mem::discriminant(self.peek()) == std::mem::discriminant(token_type)
    }

    fn match_token(&mut self, token_type: &Token) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn consume_token(&mut self, expected: Token) -> Result<&Token> {
        if self.check(&expected) {
            Ok(self.advance())
        } else {
            Err(self.create_error(expected.to_string(), self.peek().clone()))
        }
    }
}

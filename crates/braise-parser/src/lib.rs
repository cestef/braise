#![feature(assert_matches)]

use ::lexer::{SpannedToken, Token};
use core::{
    BraiseType, TypeInferenceEngine, TypeValidator, ast::*, error::parser::Result,
    parser::ParseError, runtime::RuntimeError, *,
};
use std::sync::Arc;

mod expressions;
mod helpers;
mod match_patterns;
mod statements;
mod types;

#[derive(Debug, Clone)]
pub struct Parser {
    pub tokens: Vec<SpannedToken>,
    pub source: Arc<String>,
    pub current: usize,
    pub file_id: FileId,
    pub type_engine: TypeInferenceEngine,
    pub enable_type_checking: bool,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>, source: Arc<String>, filename: String) -> Self {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        filename.hash(&mut hasher);
        let file_id = FileId(hasher.finish() as usize);

        Parser {
            tokens,
            current: 0,
            source,
            file_id,
            type_engine: TypeInferenceEngine::new(),
            enable_type_checking: true,
        }
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

    /// Create parser with type checking disabled
    pub fn new_without_type_checking(
        tokens: Vec<SpannedToken>,
        source: Arc<String>,
        filename: String,
    ) -> Self {
        let mut parser = Self::new(tokens, source, filename);
        parser.enable_type_checking = false;
        parser
    }

    /// Enable or disable type checking
    pub fn with_type_checking(mut self, enabled: bool) -> Self {
        self.enable_type_checking = enabled;
        self
    }

    pub fn parse(&mut self) -> Result<Config> {
        let mut recipes = Vec::new();
        let mut shell = None;
        let start_token = self.current;

        while !self.is_at_end() {
            if self.match_token(&Token::Shell) {
                if shell.is_some() {
                    return Err(self.create_error(
                        "only one shell command is allowed".to_string(),
                        Token::Shell,
                    ));
                }
                let s = self.parse_string()?;
                shell = Some(s);
                continue;
            }
            recipes.push(self.parse_recipe()?);
        }

        let end_token = self.current;
        let span = self.span_from_token_range(start_token, end_token.max(1));

        let mut config = Config {
            recipes,
            span,
            shell,
        };

        if self.enable_type_checking {
            self.type_check_config(&mut config)?;
        }

        Ok(config)
    }

    /// Perform type checking on the entire configuration
    fn type_check_config(&mut self, config: &mut Config) -> Result<()> {
        for recipe in &config.recipes {
            self.register_recipe_signature(&recipe.value)?;
        }

        for recipe in &mut config.recipes {
            self.type_check_recipe(&mut recipe.value)?;
        }

        Ok(())
    }

    /// Register a recipe's signature (name and parameters) for type checking
    fn register_recipe_signature(&mut self, recipe: &Recipe) -> Result<()> {
        for param in &recipe.parameters {
            self.type_engine
                .define_variable(param.value.name.clone(), param.value.effective_type());
        }
        Ok(())
    }

    /// Perform type checking on a single recipe
    fn type_check_recipe(&mut self, recipe: &mut Recipe) -> Result<()> {
        let new_engine = self.type_engine.enter_scope();
        let original_engine = std::mem::replace(&mut self.type_engine, new_engine);

        for param in &recipe.parameters {
            self.type_engine
                .define_variable(param.value.name.clone(), param.value.effective_type());
        }

        for statement in &mut recipe.body {
            self.type_check_statement(&mut statement.value)?;
        }

        self.type_engine = original_engine;
        Ok(())
    }

    /// Type check a statement
    fn type_check_statement(&mut self, statement: &mut Statement) -> Result<()> {
        if !self.enable_type_checking {
            return Ok(());
        }

        match statement {
            Statement::Let {
                name,
                value,
                param_type,
                inferred_type,
            } => {
                let var_type = if let Some(value_expr) = value {
                    let expr_type = self.type_check_expression(value_expr)?;

                    if !expr_type.is_compatible_with(param_type) {
                        return Err(self.create_type_error(
                            param_type.to_string(),
                            expr_type.to_string(),
                            format!("variable declaration '{}'", name),
                            &value_expr.span,
                        ));
                    }

                    *inferred_type = Some(expr_type.clone());
                    expr_type
                } else {
                    param_type.clone()
                };

                self.type_engine.define_variable(name.clone(), var_type);
            }

            Statement::Assign { name, value } => {
                let expr_type = self.type_check_expression(value)?;

                if let Some(var_type) = self.type_engine.get_variable_type(name) {
                    if !expr_type.is_compatible_with(var_type) {
                        return Err(self.create_type_error(
                            var_type.to_string(),
                            expr_type.to_string(),
                            format!("assignment to variable '{}'", name),
                            &value.span,
                        ));
                    }
                } else {
                    return Err(self.create_undefined_variable_error(name, &value.span));
                }
            }

            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                let condition_type = self.type_check_expression(condition)?;
                let can_convert = condition_type.can_convert_from(&BraiseType::Bool);
                if !can_convert && !matches!(condition_type, BraiseType::Bool | BraiseType::Any) {
                    return Err(self.create_type_error(
                        "boolean or boolean-convertible".to_string(),
                        condition_type.to_string(),
                        "if condition".to_string(),
                        &condition.span,
                    ));
                }

                for stmt in then_block {
                    self.type_check_statement(&mut stmt.value)?;
                }

                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.type_check_statement(&mut stmt.value)?;
                    }
                }
            }

            Statement::For {
                var,
                iterable,
                body,
                ..
            } => {
                let iterable_type = self.type_check_expression(iterable)?;

                let element_type = match &iterable_type {
                    BraiseType::Array(element_type) => (**element_type).clone(),
                    BraiseType::Any => BraiseType::Any,
                    BraiseType::Optional(inner_type) => match inner_type.as_ref() {
                        BraiseType::Array(element_type) => (**element_type).clone(),
                        BraiseType::Any => BraiseType::Any,
                        _ => {
                            return Err(self.create_type_error(
                                "array".to_string(),
                                iterable_type.to_string(),
                                "for loop iterable".to_string(),
                                &iterable.span,
                            ));
                        }
                    },
                    _ => {
                        return Err(self.create_type_error(
                            "array".to_string(),
                            iterable_type.to_string(),
                            "for loop iterable".to_string(),
                            &iterable.span,
                        ));
                    }
                };

                let new_engine = self.type_engine.enter_scope();
                let original_engine = std::mem::replace(&mut self.type_engine, new_engine);
                self.type_engine.define_variable(var.clone(), element_type);

                for stmt in body {
                    self.type_check_statement(&mut stmt.value)?;
                }

                self.type_engine = original_engine;
            }

            Statement::Match { expr, arms } => {
                let expr_type = self.type_check_expression(expr)?;

                let patterns: Vec<MatchPattern> =
                    arms.iter().map(|arm| arm.value.pattern.clone()).collect();

                if let Err(e) = TypeValidator::check_match_exhaustiveness(&expr_type, &patterns) {
                    return Err(self.create_error_from_runtime_error(e, &expr.span));
                }

                for arm in arms {
                    let new_engine = self.type_engine.enter_scope();
                    let original_engine = std::mem::replace(&mut self.type_engine, new_engine);

                    self.bind_pattern_variables(&arm.value.pattern, &expr_type)?;
                    for stmt in &mut arm.value.body {
                        self.type_check_statement(&mut stmt.value)?;
                    }

                    self.type_engine = original_engine;
                }
            }

            Statement::Run(expr) | Statement::Print(expr) | Statement::Exit(expr) => {
                self.type_check_expression(expr)?;
            }

            Statement::Call { recipe, args } => {
                self.type_check_expression(recipe)?;
                for (_, arg) in args {
                    self.type_check_expression(arg)?;
                }
                // TODO: Validate recipe call arguments against recipe signature
            }

            Statement::Shell { .. } => {}
        }

        Ok(())
    }

    /// Bind pattern variables to their types in the current scope
    fn bind_pattern_variables(
        &mut self,
        pattern: &MatchPattern,
        expr_type: &BraiseType,
    ) -> Result<()> {
        match pattern {
            MatchPattern::Variable(name) => {
                self.type_engine
                    .define_variable(name.clone(), expr_type.clone());
            }
            MatchPattern::Guard { pattern, .. } => {
                self.bind_pattern_variables(&pattern.value, expr_type)?;
                // TODO: check condition?
            }
            MatchPattern::Array { .. } => todo!(),
            _ => {}
        }
        Ok(())
    }

    /// Type check an expression and return its type
    fn type_check_expression(
        &mut self,
        expression: &mut Spanned<Expression>,
    ) -> Result<BraiseType> {
        if !self.enable_type_checking {
            return Ok(BraiseType::Any);
        }

        let expr_type = match &mut expression.value {
            Expression::String(_) => BraiseType::String,
            Expression::Number(_) => BraiseType::Number,
            Expression::Bool(_) => BraiseType::Bool,

            Expression::Variable(name) => self
                .type_engine
                .get_variable_type(name)
                .cloned()
                .ok_or_else(|| self.create_undefined_variable_error(name, &expression.span))?,

            Expression::Array(elements) => {
                if elements.is_empty() {
                    BraiseType::Array(Box::new(BraiseType::Any))
                } else {
                    let first_type = self.type_check_expression(&mut elements[0])?;
                    let mut common_type = first_type;

                    for elem in elements.iter_mut().skip(1) {
                        let elem_type = self.type_check_expression(elem)?;
                        common_type = common_type.common_type(&elem_type);
                    }

                    BraiseType::Array(Box::new(common_type))
                }
            }

            Expression::FunctionCall {
                module,
                function,
                args,
                return_type,
            } => {
                for arg in args {
                    self.type_check_expression(arg)?;
                }

                let func_type = self.get_builtin_function_type(module, function);
                *return_type = Some(func_type.clone());
                func_type
            }

            Expression::ModuleAccess {
                module,
                field,
                field_type,
            } => {
                let field_type_resolved = self.get_builtin_field_type(module, field);
                *field_type = Some(field_type_resolved.clone());
                field_type_resolved
            }

            Expression::BinaryOp {
                left,
                op,
                right,
                result_type,
            } => {
                let left_type = self.type_check_expression(left)?;
                let right_type = self.type_check_expression(right)?;

                let result_type_resolved = op.result_type(&left_type, &right_type);
                *result_type = Some(result_type_resolved.clone());
                result_type_resolved
            }

            Expression::UnaryOp {
                op,
                expr,
                result_type,
            } => {
                let operand_type = self.type_check_expression(expr)?;
                let result_type_resolved = op.result_type(&operand_type);
                *result_type = Some(result_type_resolved.clone());
                result_type_resolved
            }

            Expression::Conditional {
                condition,
                then_expr,
                else_expr,
                result_type,
            } => {
                let condition_type = self.type_check_expression(condition)?;
                let then_type = self.type_check_expression(then_expr)?;
                let else_type = self.type_check_expression(else_expr)?;

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

                let result_type_resolved = then_type.common_type(&else_type);
                *result_type = Some(result_type_resolved.clone());
                result_type_resolved
            }

            Expression::Interpolation(parts) => {
                for part in parts {
                    if let InterpolationPart::Expression(expr) = part {
                        self.type_check_expression(expr)?;
                    }
                }
                BraiseType::String
            }

            Expression::RecipeRef { .. } => BraiseType::Recipe,

            Expression::Match {
                expr,
                arms,
                result_type,
            } => {
                let expr_type = self.type_check_expression(expr)?;

                if arms.is_empty() {
                    return Err(self.create_error(
                        "match expression must have at least one arm".to_string(),
                        Token::Match,
                    ));
                }

                let patterns: Vec<MatchPattern> =
                    arms.iter().map(|arm| arm.value.pattern.clone()).collect();

                if let Err(e) = TypeValidator::check_match_exhaustiveness(&expr_type, &patterns) {
                    return Err(self.create_error_from_runtime_error(e, &expr.span));
                }

                let mut original_engine = self.type_engine.clone();

                let new_scope = self.type_engine.enter_scope();
                self.type_engine = new_scope;
                self.bind_pattern_variables(&arms[0].value.pattern, &expr_type)?;
                let first_arm_type = self.type_check_expression(&mut arms[0].value.expr)?;

                let mut result_type_resolved = first_arm_type;

                for arm in arms.iter_mut().skip(1) {
                    let new_scope = original_engine.enter_scope();
                    self.type_engine = new_scope;
                    self.bind_pattern_variables(&arm.value.pattern, &expr_type)?;
                    let arm_type = self.type_check_expression(&mut arm.value.expr)?;
                    result_type_resolved = result_type_resolved.common_type(&arm_type);
                }

                self.type_engine = original_engine;

                *result_type = Some(result_type_resolved.clone());
                result_type_resolved
            }
        };

        Ok(expr_type)
    }

    /// Get the return type of a builtin function
    fn get_builtin_function_type(&self, module: &str, function: &str) -> BraiseType {
        match (module, function) {
            ("env", "get") => BraiseType::String,
            ("env", "has") => BraiseType::Bool,
            ("cpu", "count") => BraiseType::Number,
            ("cpu", "physical_count") => BraiseType::Number,
            ("git", "branch") => BraiseType::String,
            ("git", "commit_hash") => BraiseType::String,
            ("git", "commit_hash_short") => BraiseType::String,
            ("git", "is_clean") => BraiseType::Bool,
            ("git", "is_dirty") => BraiseType::Bool,
            ("git", "tag") => BraiseType::String,
            ("fs", "exists") => BraiseType::Bool,
            ("fs", "is_file") => BraiseType::Bool,
            ("fs", "is_dir") => BraiseType::Bool,
            _ => BraiseType::Any,
        }
    }

    /// Get the type of a builtin field
    fn get_builtin_field_type(&self, module: &str, field: &str) -> BraiseType {
        match (module, field) {
            ("env", "HOME") => BraiseType::String,
            ("env", "PWD") => BraiseType::String,
            ("env", "CI") => BraiseType::Bool,
            ("cpu", "arch") => BraiseType::String,
            _ => BraiseType::Any,
        }
    }

    /// Create a type error with better formatting
    fn create_type_error(
        &self,
        expected: String,
        got: String,
        context: String,
        span: &Span,
    ) -> ParseError {
        let suggestion = self.suggest_type_fix(&expected, &got);
        let mut message = format!(
            "Type mismatch in {}: expected {}, got {}",
            context, expected, got
        );

        if let Some(suggestion) = suggestion {
            message.push_str(&format!(". {}", suggestion));
        }

        ParseError::InvalidExpression {
            expression: message,
            code: self.source.as_ref().clone(),
            span: span.into(),
        }
    }

    /// Create an undefined variable error
    fn create_undefined_variable_error(&self, name: &str, span: &Span) -> ParseError {
        ParseError::InvalidExpression {
            expression: format!("Undefined variable: '{}'", name),
            code: self.source.as_ref().clone(),
            span: span.into(),
        }
    }

    /// Convert a runtime error to a parse error
    fn create_error_from_runtime_error(&self, error: RuntimeError, span: &Span) -> ParseError {
        ParseError::InvalidExpression {
            expression: error.to_string(),
            code: self.source.as_ref().clone(),
            span: span.into(),
        }
    }

    /// Suggest type conversion fixes
    fn suggest_type_fix(&self, expected: &str, got: &str) -> Option<String> {
        match (expected, got) {
            ("number", s) if s.starts_with("string") => {
                Some("Try using a numeric string like \"42\"".to_string())
            }
            ("boolean or boolean-convertible", s) if s.starts_with("string") => {
                Some("Use \"true\"/\"false\", \"1\"/\"0\", or \"yes\"/\"no\"".to_string())
            }
            ("array", _) => Some("Use array syntax like [\"item1\", \"item2\"]".to_string()),
            _ => None,
        }
    }
}

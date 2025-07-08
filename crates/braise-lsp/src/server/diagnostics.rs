use crate::server::text_document::TextDocumentProvider;
use crate::utils::span_to_range;

use super::Document;
use braise_core::BraiseType;
use braise_core::ast::*;
use braise_errors::{BraiseError, ParserError};
use miette::SourceSpan;
use std::collections::HashMap;
use tower_lsp::Client;
use tower_lsp::lsp_types::*;
use url::Url;

pub struct DiagnosticsProvider {
    client: Client,
}

impl DiagnosticsProvider {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    pub async fn publish_parse_error(&self, uri: &Url, error: &BraiseError) {
        let diagnostic = self.error_to_diagnostic(error);
        self.publish_diagnostics(uri, vec![diagnostic]).await;
    }

    pub async fn publish_diagnostics(&self, uri: &Url, diagnostics: Vec<Diagnostic>) {
        self.client
            .publish_diagnostics(uri.clone(), diagnostics, None)
            .await;
    }

    pub async fn clear_diagnostics(&self, uri: &Url) {
        self.client
            .publish_diagnostics(uri.clone(), Vec::new(), None)
            .await;
    }

    pub fn validate_ast(&self, ast: &Config, doc: &Document) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        let mut recipe_names = HashMap::new();
        for recipe in &ast.recipes {
            let name = &recipe.value.name;
            if let Some(first_span) = recipe_names.get(name) {
                diagnostics.push(Diagnostic {
                    range: span_to_range(&recipe.span),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("duplicate_recipe".to_string())),
                    message: format!("Duplicate recipe name '{name}'"),
                    source: Some("braise".to_string()),
                    related_information: Some(vec![DiagnosticRelatedInformation {
                        location: Location {
                            uri: doc.uri.clone(),
                            range: span_to_range(first_span),
                        },
                        message: "First definition here".to_string(),
                    }]),
                    ..Default::default()
                });
            } else {
                recipe_names.insert(name.clone(), recipe.span.clone());
            }
        }

        self.check_circular_dependencies(ast, &mut diagnostics);

        self.check_undefined_dependencies(ast, &mut diagnostics);

        for recipe in &ast.recipes {
            self.validate_recipe_parameters(&recipe.value, doc, &mut diagnostics);
        }

        for recipe in &ast.recipes {
            for statement in &recipe.value.body {
                self.validate_statement(&statement.value, doc, &mut diagnostics);
            }
        }

        diagnostics
    }

    #[allow(dead_code)]
    pub fn check_match_exhaustiveness(patterns: &[MatchPattern], value_type: &BraiseType) -> bool {
        for _pattern in patterns {}

        match value_type {
            BraiseType::Bool => {
                let has_true = patterns
                    .iter()
                    .any(|p| matches!(p, MatchPattern::Bool(true)));
                let has_false = patterns
                    .iter()
                    .any(|p| matches!(p, MatchPattern::Bool(false)));
                has_true && has_false
            }

            BraiseType::Enum(variants) => {
                for variant in variants {
                    let covered = patterns
                        .iter()
                        .any(|p| matches!(p, MatchPattern::String(s) if s == variant));
                    if !covered {
                        return false;
                    }
                }
                true
            }

            _ => false,
        }
    }

    fn check_circular_dependencies(&self, ast: &Config, diagnostics: &mut Vec<Diagnostic>) {
        fn has_cycle(
            recipe: &str,
            dependencies: &HashMap<String, Vec<String>>,
            visited: &mut Vec<String>,
        ) -> bool {
            if visited.contains(&recipe.to_string()) {
                return true;
            }

            visited.push(recipe.to_string());

            if let Some(deps) = dependencies.get(recipe) {
                for dep in deps {
                    if has_cycle(dep, dependencies, visited) {
                        return true;
                    }
                }
            }

            visited.pop();
            false
        }

        let mut dependencies = HashMap::new();
        for recipe in &ast.recipes {
            dependencies.insert(recipe.value.name.clone(), recipe.value.dependencies.clone());
        }

        for recipe in &ast.recipes {
            let mut visited = Vec::new();
            if has_cycle(&recipe.value.name, &dependencies, &mut visited) {
                diagnostics.push(Diagnostic {
                    range: span_to_range(&recipe.span),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("circular_dependency".to_string())),
                    message: format!(
                        "Circular dependency detected in recipe '{}'",
                        recipe.value.name
                    ),
                    source: Some("braise".to_string()),
                    ..Default::default()
                });
            }
        }
    }

    fn check_undefined_dependencies(&self, ast: &Config, diagnostics: &mut Vec<Diagnostic>) {
        let recipe_names: std::collections::HashSet<String> =
            ast.recipes.iter().map(|r| r.value.name.clone()).collect();

        for recipe in &ast.recipes {
            for dep in &recipe.value.dependencies {
                if !recipe_names.contains(dep) {
                    diagnostics.push(Diagnostic {
                        range: span_to_range(&recipe.span),
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: Some(NumberOrString::String("undefined_dependency".to_string())),
                        message: format!("Undefined dependency '{dep}'"),
                        source: Some("braise".to_string()),
                        ..Default::default()
                    });
                }
            }
        }
    }

    fn validate_recipe_parameters(
        &self,
        recipe: &Recipe,
        doc: &Document,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let mut param_names = HashMap::new();

        for param in &recipe.parameters {
            let name = &param.value.name;
            if let Some(first_span) = param_names.get(name) {
                diagnostics.push(Diagnostic {
                    range: span_to_range(&param.span),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("duplicate_parameter".to_string())),
                    message: format!("Duplicate parameter name '{name}'"),
                    source: Some("braise".to_string()),
                    related_information: Some(vec![DiagnosticRelatedInformation {
                        location: Location {
                            uri: doc.uri.clone(),
                            range: span_to_range(first_span),
                        },
                        message: "First definition here".to_string(),
                    }]),
                    ..Default::default()
                });
            } else {
                param_names.insert(name.clone(), param.span.clone());
            }
            let param_inner_type = if let BraiseType::Optional(inner_type) = &param.value.param_type
            {
                inner_type.clone()
            } else {
                Box::new(param.value.param_type.clone())
            };
            if let Some(ref default_expr) = param.value.default
                && !self.is_expression_compatible_with_type(doc, default_expr, &param_inner_type)
            {
                diagnostics.push(Diagnostic {
                    range: span_to_range(&default_expr.span),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("type_mismatch".to_string())),
                    message: format!(
                        "Default value type doesn't match parameter type '{}'",
                        param.value.param_type
                    ),
                    source: Some("braise".to_string()),
                    ..Default::default()
                });
            }
        }
    }

    fn validate_statement(
        &self,
        statement: &Statement,
        doc: &Document,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        match statement {
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                self.validate_expression(condition, diagnostics);
                for stmt in then_block {
                    self.validate_statement(&stmt.value, doc, diagnostics);
                }
                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.validate_statement(&stmt.value, doc, diagnostics);
                    }
                }
            }
            Statement::Match { expr, arms } => {
                self.validate_expression(expr, diagnostics);
                for arm in arms {
                    for stmt in &arm.value.body {
                        self.validate_statement(&stmt.value, doc, diagnostics);
                    }
                }
            }
            Statement::For { iterable, body, .. } => {
                self.validate_expression(iterable, diagnostics);
                for stmt in body {
                    self.validate_statement(&stmt.value, doc, diagnostics);
                }
            }
            Statement::Run(expr) | Statement::Print(expr) | Statement::Exit(expr) => {
                self.validate_expression(expr, diagnostics);
            }
            Statement::Let {
                value, param_type, ..
            } => {
                if let Some(expr) = value {
                    self.validate_expression(expr, diagnostics);
                    if !self.is_expression_compatible_with_type(doc, expr, param_type) {
                        diagnostics.push(Diagnostic {
                            range: span_to_range(&expr.span),
                            severity: Some(DiagnosticSeverity::ERROR),
                            code: Some(NumberOrString::String("type_mismatch".to_string())),
                            message: format!(
                                "Expression type doesn't match parameter type '{param_type}'"
                            ),
                            source: Some("braise".to_string()),
                            ..Default::default()
                        });
                    }
                }
            }
            Statement::Assign { value, name } => {
                self.validate_expression(value, diagnostics);

                if let Some(ref ast) = doc.ast {
                    if let Some(recipe) = TextDocumentProvider::find_recipe_at_position(
                        ast,
                        Position {
                            line: value.span.start.line,
                            character: value.span.start.column,
                        },
                    ) {
                        if !Self::is_variable_defined(name, &recipe.value) {
                            diagnostics.push(Diagnostic {
                                range: span_to_range(&value.span),
                                severity: Some(DiagnosticSeverity::ERROR),
                                code: Some(NumberOrString::String(
                                    "undefined_variable".to_string(),
                                )),
                                message: format!("Variable '{name}' is not defined"),
                                source: Some("braise".to_string()),
                                ..Default::default()
                            });
                        }
                    } else {
                        diagnostics.push(Diagnostic {
                            range: span_to_range(&value.span),
                            severity: Some(DiagnosticSeverity::ERROR),
                            code: Some(NumberOrString::String("undefined_variable".to_string())),
                            message: format!("Variable '{name}' is not defined"),
                            source: Some("braise".to_string()),
                            ..Default::default()
                        });
                    }
                }
            }
            Statement::Call { recipe, args } => {
                if let Some(ref ast) = doc.ast
                    && let Some(recipe_ref) = TextDocumentProvider::find_recipe_at_position(
                        ast,
                        Position {
                            line: recipe.span.start.line,
                            character: recipe.span.start.column,
                        },
                    )
                {
                    for (arg_name, arg_expr) in args {
                        if !Self::is_variable_defined(arg_name, &recipe_ref.value) {
                            diagnostics.push(Diagnostic {
                                range: span_to_range(&arg_expr.span),
                                severity: Some(DiagnosticSeverity::ERROR),
                                code: Some(NumberOrString::String(
                                    "undefined_variable".to_string(),
                                )),
                                message: format!("Variable '{arg_name}' is not defined"),
                                source: Some("braise".to_string()),
                                ..Default::default()
                            });
                        } else if let Some(param_type) =
                            Self::get_variable_type(arg_name, &recipe_ref.value)
                            && !self.is_expression_compatible_with_type(doc, arg_expr, &param_type)
                        {
                            diagnostics.push(Diagnostic {
                                        range: span_to_range(&arg_expr.span),
                                        severity: Some(DiagnosticSeverity::ERROR),
                                        code: Some(NumberOrString::String(
                                            "type_mismatch".to_string(),
                                        )),
                                        message: format!(
                                            "Argument '{arg_name}' type doesn't match parameter type '{param_type}'"
                                        ),
                                        source: Some("braise".to_string()),
                                        ..Default::default()
                                    });
                        }
                    }
                }
            }
            Statement::Shell { .. } => {}
        }
    }

    pub fn is_variable_defined(name: &str, recipe: &Recipe) -> bool {
        if recipe.parameters.iter().any(|p| p.value.name == name) {
            return true;
        }

        for stmt in &recipe.body {
            if let Statement::Let { name: var_name, .. } = &stmt.value
                && var_name == name
            {
                return true;
            }
        }

        false
    }

    pub fn get_variable_type(name: &str, recipe: &Recipe) -> Option<BraiseType> {
        for param in &recipe.parameters {
            if param.value.name == name {
                return Some(param.value.param_type.clone());
            }
        }

        for stmt in &recipe.body {
            if let Statement::Let {
                name: var_name,
                param_type,
                ..
            } = &stmt.value
                && var_name == name
            {
                return Some(param_type.clone());
            }
        }

        None
    }

    fn validate_expression(
        &self,
        expr: &SpannedNode<Expression>,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        match &expr.value {
            Expression::FunctionCall {
                module,
                function,
                args,
                ..
            } => {
                if !self.is_valid_builtin_function(module, function) {
                    diagnostics.push(Diagnostic {
                        range: span_to_range(if args.is_empty() {
                            &expr.span
                        } else {
                            &args[0].span
                        }),
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: Some(NumberOrString::String("invalid_function".to_string())),
                        message: format!("Invalid function call '{module}.{function}'"),
                        source: Some("braise".to_string()),
                        ..Default::default()
                    });
                }
                for arg in args {
                    self.validate_expression(arg, diagnostics);
                }
            }
            Expression::ModuleAccess { module, field, .. } => {
                if !self.is_valid_builtin_field(module, field) {
                    // TODO: need spans for module fields
                }
            }
            Expression::BinaryOp { left, right, .. } => {
                self.validate_expression(left, diagnostics);
                self.validate_expression(right, diagnostics);
            }
            Expression::UnaryOp { expr, .. } => {
                self.validate_expression(expr, diagnostics);
            }
            Expression::Conditional {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                self.validate_expression(condition, diagnostics);
                self.validate_expression(then_expr, diagnostics);
                self.validate_expression(else_expr, diagnostics);
            }
            Expression::Array(elements) => {
                for elem in elements {
                    self.validate_expression(elem, diagnostics);
                }
            }
            Expression::Interpolation(parts) => {
                for part in parts {
                    if let InterpolationPart::Expression(expr) = part {
                        self.validate_expression(expr, diagnostics);
                    }
                }
            }
            _ => {}
        }
    }

    fn is_expression_compatible_with_type(
        &self,
        doc: &Document,
        expr: &SpannedNode<Expression>,
        param_type: &BraiseType,
    ) -> bool {
        match (&expr.value, param_type) {
            (Expression::String(_), BraiseType::String) => true,
            (Expression::Number(_), BraiseType::Number) => true,
            (Expression::Bool(_), BraiseType::Bool) => true,
            (Expression::Array(elements), BraiseType::Array(element_type)) => elements
                .iter()
                .all(|elem| self.is_expression_compatible_with_type(doc, elem, element_type)),
            (Expression::String(s), BraiseType::Enum(variants)) => variants.contains(s),
            (
                Expression::Conditional {
                    condition,
                    then_expr,
                    else_expr,
                    ..
                },
                e,
            ) => {
                self.is_expression_compatible_with_type(doc, condition, &BraiseType::Bool)
                    && self.is_expression_compatible_with_type(doc, then_expr, e)
                    && self.is_expression_compatible_with_type(doc, else_expr, e)
            }
            (
                Expression::FunctionCall {
                    module, function, ..
                },
                _,
            ) => self.is_valid_builtin_function(module, function),
            (Expression::ModuleAccess { module, field, .. }, _) => {
                self.is_valid_builtin_field(module, field)
            }
            (Expression::Variable(name), e) => {
                if let Some(ref ast) = doc.ast
                    && let Some(recipe) = TextDocumentProvider::find_recipe_at_position(
                        ast,
                        Position {
                            line: expr.span.start.line,
                            character: expr.span.start.column,
                        },
                    )
                {
                    if let Some(param_type) = Self::get_variable_type(&name, &recipe.value) {
                        return param_type.is_compatible_with(e);
                    } else {
                        return false;
                    }
                }
                false
            }
            (Expression::RecipeRef { .. }, BraiseType::Recipe) => true,
            (Expression::Match { arms, .. }, e) => arms
                .iter()
                .all(|arm| self.is_expression_compatible_with_type(doc, &arm.value.expr, e)),
            (_, BraiseType::Union(ïnner_types)) => ïnner_types
                .iter()
                .any(|inner_type| self.is_expression_compatible_with_type(doc, expr, inner_type)),
            (Expression::Interpolation(_), BraiseType::String) => true,
            (Expression::BinaryOp { left, right, .. }, e) => {
                self.is_expression_compatible_with_type(doc, left, e)
                    && self.is_expression_compatible_with_type(doc, right, e)
            }
            e => {
                dbg!(e);
                false
            }
        }
    }

    fn is_valid_builtin_function(&self, module: &str, function: &str) -> bool {
        match module {
            "env" => matches!(function, "get" | "has"),
            "cpu" => matches!(function, "count" | "physical_count"),
            "git" => matches!(
                function,
                "is_dirty" | "is_clean" | "branch" | "commit_hash" | "commit_hash_short" | "tag"
            ),
            "fs" => matches!(function, "exists" | "is_file" | "is_dir"),
            _ => false,
        }
    }

    fn is_valid_builtin_field(&self, module: &str, field: &str) -> bool {
        match module {
            "env" => matches!(field, "HOME" | "PWD" | "CI"),
            "cpu" => matches!(field, "arch"),
            _ => false,
        }
    }

    fn offset_to_position(&self, offset: usize, text: &str) -> Position {
        let mut line = 0;
        let mut character = 0;

        for (i, c) in text.char_indices() {
            if i == offset {
                return Position {
                    line: line as u32,
                    character: character as u32,
                };
            }
            if c == '\n' {
                line += 1;
                character = 0;
            } else {
                character += 1;
            }
        }

        Position {
            line: line as u32,
            character: character as u32,
        }
    }

    fn source_span_to_range(&self, span: &SourceSpan, code: &str) -> Range {
        let start = self.offset_to_position(span.offset(), code);
        let mut end = self.offset_to_position(span.offset() + span.len(), code);

        end.character = end.character.saturating_sub(1);
        Range { start, end }
    }

    fn error_to_diagnostic(&self, error: &BraiseError) -> Diagnostic {
        match error {
            BraiseError::Lexer {
                span,
                code,
                message,
            } => Diagnostic {
                range: self.source_span_to_range(span, code),
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(NumberOrString::String("lexer_error".to_string())),
                message: message.clone(),
                source: Some("braise".to_string()),
                ..Default::default()
            },
            BraiseError::Parser(parse_error) => match parse_error {
                ParserError::UnexpectedToken {
                    expected,
                    found,
                    code,
                    span,
                } => Diagnostic {
                    range: self.source_span_to_range(span, code),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("unexpected_token".to_string())),
                    message: format!("Unexpected token: expected {expected}, found {found}"),
                    source: Some("braise".to_string()),
                    ..Default::default()
                },
                ParserError::InvalidExpression {
                    reason: expression,
                    code,
                    span,
                } => Diagnostic {
                    range: self.source_span_to_range(span, code),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("invalid_expression".to_string())),
                    message: format!("Invalid expression: {expression}"),
                    source: Some("braise".to_string()),
                    ..Default::default()
                },
                ParserError::Other {
                    message,
                    code,
                    span,
                } => Diagnostic {
                    range: if let (Some(code), Some(span)) = (code, span) {
                        self.source_span_to_range(span, code)
                    } else {
                        Range {
                            start: Position {
                                line: 0,
                                character: 0,
                            },
                            end: Position {
                                line: 0,
                                character: 0,
                            },
                        }
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("parse_error".to_string())),
                    message: self.strip_colors(message),
                    source: Some("braise".to_string()),
                    ..Default::default()
                },
                ParserError::NonExhaustiveMatch {
                    code,
                    span,
                    missing,
                } => Diagnostic {
                    range: self.source_span_to_range(span, code),
                    severity: Some(DiagnosticSeverity::WARNING),
                    code: Some(NumberOrString::String("non_exhaustive_match".to_string())),
                    message: format!(
                        "Non-exhaustive match: {}",
                        missing
                            .as_deref()
                            .unwrap_or("consider adding a wildcard pattern '_'")
                    ),
                    source: Some("braise".to_string()),
                    ..Default::default()
                },
                ParserError::InvalidFunctionSignature { reason, code, span } => Diagnostic {
                    range: self.source_span_to_range(span, code),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String(
                        "invalid_function_signature".to_string(),
                    )),
                    message: format!("Invalid function signature: {reason}"),
                    source: Some("braise".to_string()),
                    ..Default::default()
                },
                ParserError::InvalidTypeAnnotation { reason, code, span } => Diagnostic {
                    range: self.source_span_to_range(span, code),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String(
                        "invalid_type_annotation".to_string(),
                    )),
                    message: format!("Invalid type annotation: {reason}"),
                    source: Some("braise".to_string()),
                    ..Default::default()
                },
                ParserError::DuplicateParameter {
                    name, code, span, ..
                } => Diagnostic {
                    range: self.source_span_to_range(span, code),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("duplicate_parameter".to_string())),
                    message: format!("Duplicate parameter name: '{name}'"),
                    source: Some("braise".to_string()),
                    ..Default::default()
                },
                ParserError::InvalidDependency { reason, code, span } => Diagnostic {
                    range: self.source_span_to_range(span, code),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("invalid_dependency".to_string())),
                    message: format!("Invalid recipe dependency: {reason}"),
                    source: Some("braise".to_string()),
                    ..Default::default()
                },
            },
            _ => Diagnostic {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 0,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(NumberOrString::String("unknown_error".to_string())),
                message: self.strip_colors(&error.to_string()),
                source: Some("braise".to_string()),
                ..Default::default()
            },
        }
    }

    fn strip_colors(&self, text: &str) -> String {
        let re = regex::Regex::new(r"\x1B\[[0-9;]*[mK]").unwrap();
        re.replace_all(text, "").to_string()
    }
}

use super::Document;
use braise_core::{ast::*, error::BraiseError};
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

        // Check for duplicate recipe names
        let mut recipe_names = HashMap::new();
        for recipe in &ast.recipes {
            let name = &recipe.value.name;
            if let Some(first_span) = recipe_names.get(name) {
                diagnostics.push(Diagnostic {
                    range: self.span_to_range(&recipe.span, doc),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("duplicate_recipe".to_string())),
                    message: format!("Duplicate recipe name '{}'", name),
                    source: Some("braise".to_string()),
                    related_information: Some(vec![DiagnosticRelatedInformation {
                        location: Location {
                            uri: doc.uri.clone(),
                            range: self.span_to_range(first_span, doc),
                        },
                        message: "First definition here".to_string(),
                    }]),
                    ..Default::default()
                });
            } else {
                recipe_names.insert(name.clone(), recipe.span.clone());
            }
        }

        self.check_circular_dependencies(ast, doc, &mut diagnostics);

        self.check_undefined_dependencies(ast, doc, &mut diagnostics);

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

    fn check_circular_dependencies(
        &self,
        ast: &Config,
        doc: &Document,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
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
                    range: self.span_to_range(&recipe.span, doc),
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

    fn check_undefined_dependencies(
        &self,
        ast: &Config,
        doc: &Document,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        let recipe_names: std::collections::HashSet<String> =
            ast.recipes.iter().map(|r| r.value.name.clone()).collect();

        for recipe in &ast.recipes {
            for dep in &recipe.value.dependencies {
                if !recipe_names.contains(dep) {
                    diagnostics.push(Diagnostic {
                        range: self.span_to_range(&recipe.span, doc),
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: Some(NumberOrString::String("undefined_dependency".to_string())),
                        message: format!("Undefined dependency '{}'", dep),
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
                    range: self.span_to_range(&param.span, doc),
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("duplicate_parameter".to_string())),
                    message: format!("Duplicate parameter name '{}'", name),
                    source: Some("braise".to_string()),
                    related_information: Some(vec![DiagnosticRelatedInformation {
                        location: Location {
                            uri: doc.uri.clone(),
                            range: self.span_to_range(first_span, doc),
                        },
                        message: "First definition here".to_string(),
                    }]),
                    ..Default::default()
                });
            } else {
                param_names.insert(name.clone(), param.span.clone());
            }

            // Validate default value type compatibility
            if let Some(ref default_expr) = param.value.default {
                if !self.is_expression_compatible_with_type(
                    &default_expr.value,
                    &param.value.param_type,
                ) {
                    diagnostics.push(Diagnostic {
                        range: self.span_to_range(&default_expr.span, doc),
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
                self.validate_expression(&condition.value, doc, diagnostics);
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
                self.validate_expression(&expr.value, doc, diagnostics);
                for arm in arms {
                    for stmt in &arm.value.body {
                        self.validate_statement(&stmt.value, doc, diagnostics);
                    }
                }
            }
            Statement::For { iterable, body, .. } => {
                self.validate_expression(&iterable.value, doc, diagnostics);
                for stmt in body {
                    self.validate_statement(&stmt.value, doc, diagnostics);
                }
            }
            Statement::Run(expr) | Statement::Print(expr) | Statement::Exit(expr) => {
                self.validate_expression(&expr.value, doc, diagnostics);
            }
        }
    }

    fn validate_expression(
        &self,
        expr: &Expression,
        doc: &Document,
        diagnostics: &mut Vec<Diagnostic>,
    ) {
        match expr {
            Expression::FunctionCall {
                module,
                function,
                args,
            } => {
                if !self.is_valid_builtin_function(module, function) {
                    diagnostics.push(Diagnostic {
                        range: self.span_to_range(&args[0].span, doc),
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: Some(NumberOrString::String("invalid_function".to_string())),
                        message: format!("Invalid function call '{}.{}'", module, function),
                        source: Some("braise".to_string()),
                        ..Default::default()
                    });
                }
                for arg in args {
                    self.validate_expression(&arg.value, doc, diagnostics);
                }
            }
            Expression::ModuleAccess { module, field } => {
                if !self.is_valid_builtin_field(module, field) {
                    // TODO: need spans for module fields
                }
            }
            Expression::BinaryOp { left, right, .. } => {
                self.validate_expression(&left.value, doc, diagnostics);
                self.validate_expression(&right.value, doc, diagnostics);
            }
            Expression::UnaryOp { expr, .. } => {
                self.validate_expression(&expr.value, doc, diagnostics);
            }
            Expression::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                self.validate_expression(&condition.value, doc, diagnostics);
                self.validate_expression(&then_expr.value, doc, diagnostics);
                self.validate_expression(&else_expr.value, doc, diagnostics);
            }
            Expression::Array(elements) => {
                for elem in elements {
                    self.validate_expression(&elem.value, doc, diagnostics);
                }
            }
            Expression::Interpolation(parts) => {
                for part in parts {
                    if let InterpolationPart::Expression(expr) = part {
                        self.validate_expression(&expr.value, doc, diagnostics);
                    }
                }
            }
            _ => {} // Other expressions are fine
        }
    }

    fn is_expression_compatible_with_type(
        &self,
        expr: &Expression,
        param_type: &ParamType,
    ) -> bool {
        match (expr, param_type) {
            (Expression::String(_), ParamType::String) => true,
            (Expression::Number(_), ParamType::Number) => true,
            (Expression::Bool(_), ParamType::Bool) => true,
            (Expression::Array(elements), ParamType::Array(element_type)) => elements
                .iter()
                .all(|elem| self.is_expression_compatible_with_type(&elem.value, element_type)),
            (Expression::String(s), ParamType::Enum(variants)) => variants.contains(s),
            (
                Expression::Conditional {
                    condition,
                    then_expr,
                    else_expr,
                },
                e,
            ) => {
                self.is_expression_compatible_with_type(&condition.value, &ParamType::Bool)
                    && self.is_expression_compatible_with_type(&then_expr.value, e)
                    && self.is_expression_compatible_with_type(&else_expr.value, e)
            }
            (
                Expression::FunctionCall {
                    module, function, ..
                },
                _,
            ) => self.is_valid_builtin_function(module, function),
            (Expression::ModuleAccess { module, field }, _) => {
                self.is_valid_builtin_field(&module, &field)
            }
            _ => false,
        }
    }

    fn is_valid_builtin_function(&self, module: &str, function: &str) -> bool {
        // TODO: can we do better?
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
        // TODO: can we do better?
        match module {
            "env" => matches!(field, "HOME" | "PWD" | "CI"),
            "cpu" => matches!(field, "arch"),
            _ => false,
        }
    }

    fn error_to_diagnostic(&self, error: &BraiseError) -> Diagnostic {
        match error {
            BraiseError::LexerError { span, .. } => Diagnostic {
                range: Range {
                    start: Position {
                        line: 0,
                        character: span.offset() as u32,
                    },
                    end: Position {
                        line: 0,
                        character: (span.offset() + span.len()) as u32,
                    },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: Some(NumberOrString::String("lexer_error".to_string())),
                message: "Unexpected token".to_string(),
                source: Some("braise".to_string()),
                ..Default::default()
            },
            BraiseError::ParserError(parse_error) => Diagnostic {
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
                code: Some(NumberOrString::String("parser_error".to_string())),
                message: self.strip_colors(&parse_error.to_string()),
                source: Some("braise".to_string()),
                ..Default::default()
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

    fn span_to_range(&self, span: &braise_core::Span, _doc: &Document) -> Range {
        let start_line = span.start.line.saturating_sub(1) as usize;
        let start_char = span.start.column.saturating_sub(1) as u32;
        let end_line = span.end.line.saturating_sub(1) as usize;
        let end_char = span.end.column.saturating_sub(1) as u32;

        Range {
            start: Position {
                line: start_line as u32,
                character: start_char,
            },
            end: Position {
                line: end_line as u32,
                character: end_char,
            },
        }
    }
}

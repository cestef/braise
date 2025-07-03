#![allow(deprecated)]
use super::Document;
use braise_core::ast::*;
use std::collections::HashMap;
use tower_lsp::lsp_types::*;
use url::Url;

pub struct SymbolProvider;

impl SymbolProvider {
    pub fn new() -> Self {
        Self
    }

    pub async fn provide_document_symbols(&self, doc: &Document) -> Vec<DocumentSymbol> {
        let mut symbols = Vec::new();

        if let Some(ref ast) = doc.ast {
            for recipe in &ast.recipes {
                let recipe_symbol = self.create_recipe_symbol(recipe, doc);
                symbols.push(recipe_symbol);
            }
        }

        symbols
    }

    pub async fn provide_workspace_symbols(
        &self,
        documents: &HashMap<Url, Document>,
        query: &str,
    ) -> Vec<SymbolInformation> {
        let mut symbols = Vec::new();

        for (uri, doc) in documents {
            if let Some(ref ast) = doc.ast {
                for recipe in &ast.recipes {
                    if query.is_empty() || recipe.value.name.contains(query) {
                        symbols.push(SymbolInformation {
                            name: recipe.value.name.clone(),
                            kind: SymbolKind::FUNCTION,
                            tags: None,
                            deprecated: Some(false),
                            location: Location {
                                uri: uri.clone(),
                                range: self.span_to_range(&recipe.span, doc),
                            },
                            container_name: None,
                        });
                    }

                    for param in &recipe.value.parameters {
                        let param_name = format!("{}.{}", recipe.value.name, param.value.name);
                        if query.is_empty()
                            || param_name.contains(query)
                            || param.value.name.contains(query)
                        {
                            symbols.push(SymbolInformation {
                                name: param.value.name.clone(),
                                kind: SymbolKind::VARIABLE,
                                tags: None,
                                deprecated: Some(false),
                                location: Location {
                                    uri: uri.clone(),
                                    range: self.span_to_range(&param.span, doc),
                                },
                                container_name: Some(recipe.value.name.clone()),
                            });
                        }
                    }
                }
            }
        }

        symbols
    }

    fn create_recipe_symbol(&self, recipe: &SpannedNode<Recipe>, doc: &Document) -> DocumentSymbol {
        let mut children = Vec::new();

        for param in &recipe.value.parameters {
            children.push(DocumentSymbol {
                name: param.value.name.clone(),
                detail: Some(format!("{}", param.value.param_type)),
                kind: SymbolKind::VARIABLE,
                tags: None,
                deprecated: Some(false),
                range: self.span_to_range(&param.span, doc),
                selection_range: self.span_to_range(&param.span, doc),
                children: None,
            });
        }

        for (i, statement) in recipe.value.body.iter().enumerate() {
            let statement_symbol = self.create_statement_symbol(statement, i, doc);
            if let Some(symbol) = statement_symbol {
                children.push(symbol);
            }
        }

        let detail = if recipe.value.dependencies.is_empty() {
            format!("{} parameters", recipe.value.parameters.len())
        } else {
            format!(
                "{} parameters, depends on: {}",
                recipe.value.parameters.len(),
                recipe.value.dependencies.join(", ")
            )
        };

        DocumentSymbol {
            name: recipe.value.name.clone(),
            detail: Some(detail),
            kind: SymbolKind::FUNCTION,
            tags: None,
            deprecated: Some(false),
            range: self.span_to_range(&recipe.span, doc),
            selection_range: self.span_to_range(&recipe.span, doc),
            children: if children.is_empty() {
                None
            } else {
                Some(children)
            },
        }
    }

    fn create_statement_symbol(
        &self,
        statement: &SpannedNode<Statement>,
        index: usize,
        doc: &Document,
    ) -> Option<DocumentSymbol> {
        let (name, kind, detail) = match &statement.value {
            Statement::Run(expr) => {
                let command_preview = self.get_expression_preview(&expr.value);
                (
                    format!("run #{}", index + 1),
                    SymbolKind::METHOD,
                    Some(format!("Command: {}", command_preview)),
                )
            }
            Statement::Print(expr) => {
                let message_preview = self.get_expression_preview(&expr.value);
                (
                    format!("print #{}", index + 1),
                    SymbolKind::METHOD,
                    Some(format!("Message: {}", message_preview)),
                )
            }
            Statement::If { condition, .. } => {
                let condition_preview = self.get_expression_preview(&condition.value);
                (
                    format!("if #{}", index + 1),
                    SymbolKind::CONSTANT,
                    Some(format!("Condition: {}", condition_preview)),
                )
            }
            Statement::Match { expr, arms } => {
                let expr_preview = self.get_expression_preview(&expr.value);
                (
                    format!("match #{}", index + 1),
                    SymbolKind::CONSTANT,
                    Some(format!("Expression: {}, {} arms", expr_preview, arms.len())),
                )
            }
            Statement::For { var, iterable, .. } => {
                let iterable_preview = self.get_expression_preview(&iterable.value);
                (
                    format!("for #{}", index + 1),
                    SymbolKind::CONSTANT,
                    Some(format!("Variable: {}, Iterable: {}", var, iterable_preview)),
                )
            }
            Statement::Exit(expr) => {
                let code_preview = self.get_expression_preview(&expr.value);
                (
                    format!("exit #{}", index + 1),
                    SymbolKind::METHOD,
                    Some(format!("Exit code: {}", code_preview)),
                )
            }
            Statement::Let {
                name,
                value,
                param_type,
            } => {
                let value_preview = if let Some(value) = value {
                    Some(self.get_expression_preview(&value.value))
                } else {
                    None
                };
                (
                    format!("let {} #{}", name, index + 1),
                    SymbolKind::VARIABLE,
                    Some(format!(
                        "Type: {}, Value: {}",
                        param_type,
                        value_preview.unwrap_or_default()
                    )),
                )
            }
            Statement::Assign { name, value } => {
                let value_preview = self.get_expression_preview(&value.value);
                (
                    format!("assign {} #{}", name, index + 1),
                    SymbolKind::VARIABLE,
                    Some(format!("Value: {}", value_preview)),
                )
            }
            Statement::Call { recipe, args } => {
                let args_preview: Vec<String> = args
                    .iter()
                    .map(|(k, arg)| format!("{k}: {}", self.get_expression_preview(&arg.value)))
                    .collect();
                let args_str = if args_preview.is_empty() {
                    String::new()
                } else {
                    format!("({})", args_preview.join(", "))
                };
                let recipe_preview = self.get_expression_preview(&recipe.value);
                (
                    format!("call {}{}", recipe_preview, args_str),
                    SymbolKind::FUNCTION,
                    Some(format!("Recipe: {}", recipe_preview)),
                )
            }
        };

        Some(DocumentSymbol {
            name,
            detail,
            kind,
            tags: None,
            deprecated: Some(false),
            range: self.span_to_range(&statement.span, doc),
            selection_range: self.span_to_range(&statement.span, doc),
            children: None,
        })
    }

    fn get_expression_preview(&self, expr: &Expression) -> String {
        match expr {
            Expression::String(s) => format!("\"{}\"", s),
            Expression::Number(n) => n.to_string(),
            Expression::Bool(b) => b.to_string(),
            Expression::Variable(name) => name.clone(),
            Expression::FunctionCall {
                module, function, ..
            } => {
                format!("{}.{}()", module, function)
            }
            Expression::ModuleAccess { module, field } => {
                format!("{}.{}", module, field)
            }
            Expression::Interpolation(parts) => {
                let preview: Vec<String> = parts
                    .iter()
                    .take(3)
                    .map(|part| match part {
                        InterpolationPart::String(s) => s.clone(),
                        InterpolationPart::Expression(expr) => {
                            format!("${{{}}}", self.get_expression_preview(&expr.value))
                        }
                    })
                    .collect();

                let result = preview.join("");
                if parts.len() > 3 {
                    format!("\"{}...\"", result)
                } else {
                    format!("\"{}\"", result)
                }
            }
            Expression::Array(elements) => {
                if elements.is_empty() {
                    "[]".to_string()
                } else if elements.len() == 1 {
                    format!("[{}]", self.get_expression_preview(&elements[0].value))
                } else {
                    format!("[{}, ...]", self.get_expression_preview(&elements[0].value))
                }
            }
            Expression::BinaryOp { left, op, right } => {
                let op_str = match op {
                    BinaryOperator::Equal => "==",
                    BinaryOperator::NotEqual => "!=",
                    BinaryOperator::Less => "<",
                    BinaryOperator::LessEqual => "<=",
                    BinaryOperator::Greater => ">",
                    BinaryOperator::GreaterEqual => ">=",
                    BinaryOperator::And => "&&",
                    BinaryOperator::Or => "||",
                };
                format!(
                    "{} {} {}",
                    self.get_expression_preview(&left.value),
                    op_str,
                    self.get_expression_preview(&right.value)
                )
            }
            Expression::UnaryOp { op, expr } => {
                let op_str = match op {
                    UnaryOperator::Not => "!",
                };
                format!("{}{}", op_str, self.get_expression_preview(&expr.value))
            }
            Expression::Conditional { condition, .. } => {
                format!("if {}", self.get_expression_preview(&condition.value))
            }
            Expression::RecipeRef { recipe, args } => {
                let args_preview: Vec<String> = args
                    .iter()
                    .map(|(k, arg)| format!("{k}: {}", self.get_expression_preview(&arg.value)))
                    .collect();
                if args_preview.is_empty() {
                    recipe.clone()
                } else {
                    format!("@{}({})", recipe, args_preview.join(", "))
                }
            }
        }
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

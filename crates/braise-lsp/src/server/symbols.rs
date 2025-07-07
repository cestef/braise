#![allow(deprecated)]
use crate::utils::{get_expression_preview, span_to_range};

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
                let recipe_symbol = self.create_recipe_symbol(recipe);
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
                                range: span_to_range(&recipe.span),
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
                                    range: span_to_range(&param.span),
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

    fn create_recipe_symbol(&self, recipe: &SpannedNode<Recipe>) -> DocumentSymbol {
        let mut children = Vec::new();

        for param in &recipe.value.parameters {
            children.push(DocumentSymbol {
                name: param.value.name.clone(),
                detail: Some(format!("{}", param.value.param_type)),
                kind: SymbolKind::VARIABLE,
                tags: None,
                deprecated: Some(false),
                range: span_to_range(&param.span),
                selection_range: span_to_range(&param.span),
                children: None,
            });
        }

        for (i, statement) in recipe.value.body.iter().enumerate() {
            let statement_symbol = self.create_statement_symbol(statement, i);
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
            range: span_to_range(&recipe.span),
            selection_range: span_to_range(&recipe.span),
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
    ) -> Option<DocumentSymbol> {
        let (name, kind, detail) = match &statement.value {
            Statement::Run(expr) => {
                let command_preview = get_expression_preview(&expr.value);
                (
                    format!("run #{}", index + 1),
                    SymbolKind::METHOD,
                    Some(format!("Command: {command_preview}")),
                )
            }
            Statement::Print(expr) => {
                let message_preview = get_expression_preview(&expr.value);
                (
                    format!("print #{}", index + 1),
                    SymbolKind::METHOD,
                    Some(format!("Message: {message_preview}")),
                )
            }
            Statement::If { condition, .. } => {
                let condition_preview = get_expression_preview(&condition.value);
                (
                    format!("if #{}", index + 1),
                    SymbolKind::CONSTANT,
                    Some(format!("Condition: {condition_preview}")),
                )
            }
            Statement::Match { expr, arms } => {
                let expr_preview = get_expression_preview(&expr.value);
                (
                    format!("match #{}", index + 1),
                    SymbolKind::CONSTANT,
                    Some(format!("Expression: {}, {} arms", expr_preview, arms.len())),
                )
            }
            Statement::For { var, iterable, .. } => {
                let iterable_preview = get_expression_preview(&iterable.value);
                (
                    format!("for #{}", index + 1),
                    SymbolKind::CONSTANT,
                    Some(format!("Variable: {var}, Iterable: {iterable_preview}")),
                )
            }
            Statement::Exit(expr) => {
                let code_preview = get_expression_preview(&expr.value);
                (
                    format!("exit #{}", index + 1),
                    SymbolKind::METHOD,
                    Some(format!("Exit code: {code_preview}")),
                )
            }
            Statement::Let {
                name,
                value,
                param_type,
                ..
            } => {
                let value_preview = value
                    .as_ref()
                    .map(|value| get_expression_preview(&value.value));
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
                let value_preview = get_expression_preview(&value.value);
                (
                    format!("assign {} #{}", name, index + 1),
                    SymbolKind::VARIABLE,
                    Some(format!("Value: {value_preview}")),
                )
            }
            Statement::Call { recipe, args } => {
                let args_preview: Vec<String> = args
                    .iter()
                    .map(|(k, arg)| format!("{k}: {}", get_expression_preview(&arg.value)))
                    .collect();
                let args_str = if args_preview.is_empty() {
                    String::new()
                } else {
                    format!("({})", args_preview.join(", "))
                };
                let recipe_preview = get_expression_preview(&recipe.value);
                (
                    format!("call {recipe_preview}{args_str}"),
                    SymbolKind::FUNCTION,
                    Some(format!("Recipe: {recipe_preview}")),
                )
            }
            Statement::Shell { name } => (
                format!("shell #{}", index + 1),
                SymbolKind::METHOD,
                Some(format!("Set Shell: {name}")),
            ),
        };

        Some(DocumentSymbol {
            name,
            detail,
            kind,
            tags: None,
            deprecated: Some(false),
            range: span_to_range(&statement.span),
            selection_range: span_to_range(&statement.span),
            children: None,
        })
    }
}

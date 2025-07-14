use braise_errors::RuntimeError;
use core::{BraiseError, Result, ast::*};
use lexer::tokenize;
use owo_colors::OwoColorize;
use parser::Parser as BraiseParser;

pub fn show_recipe_info(contents: &str, file: &str, recipe_name: &str) -> Result<()> {
    let tokens = tokenize(contents)?;
    let mut parser = BraiseParser::new(&tokens, contents, file.to_string());
    let ast = parser.parse().map_err(|e| BraiseError::from(*e))?;

    let recipe = ast
        .recipes
        .iter()
        .find(|r| r.value.name == recipe_name)
        .ok_or_else(|| {
            BraiseError::Runtime(RuntimeError::undefined_recipe(recipe_name.to_string()).boxed())
        })?;

    println!("{}\n", recipe.value.name.cyan().bold());

    if !recipe.value.dependencies.is_empty() {
        println!(
            "  {} {}",
            "Dependencies".dimmed(),
            recipe
                .value
                .dependencies
                .iter()
                .map(|d| d.blue().to_string())
                .collect::<Vec<_>>()
                .join(&format!(" {} ", "→".dimmed()))
        );
        println!();
    }

    if !recipe.value.parameters.is_empty() {
        println!("  {}", "Parameters".dimmed());
        for param in &recipe.value.parameters {
            let default = if let Some(ref default_expr) = param.value.default {
                format!(
                    " {} {}",
                    "=".dimmed(),
                    format_expression_preview(&default_expr.value).green()
                )
            } else {
                String::new()
            };
            let optional_marker = if param.value.optional || param.value.default.is_some() {
                "?"
            } else {
                ""
            };
            println!(
                "    {}{}: {}{}",
                param.value.name.bold(),
                optional_marker.dimmed(),
                param.value.param_type.to_string().blue(),
                default
            );
        }
        println!();
    }

    println!(
        "  {} {}",
        "Steps".dimmed(),
        recipe.value.body.len().to_string().bold()
    );

    Ok(())
}

fn format_expression_preview(expr: &Expression) -> String {
    match expr {
        Expression::String(s) => format!("\"{s}\""),
        Expression::Number(n) => n.to_string(),
        Expression::Bool(b) => b.to_string(),
        Expression::Variable(name) => name.clone(),
        Expression::FunctionCall {
            module, function, ..
        } => {
            format!("{module}.{function}()")
        }
        Expression::ModuleAccess { module, field, .. } => {
            format!("{module}.{field}")
        }
        Expression::Array(elements) => {
            if elements.len() <= 2 {
                let items: Vec<String> = elements
                    .iter()
                    .map(|e| format_expression_preview(&e.value))
                    .collect();
                format!("[{}]", items.join(", "))
            } else {
                format!("[{} items]", elements.len())
            }
        }
        _ => "...".to_string(),
    }
}

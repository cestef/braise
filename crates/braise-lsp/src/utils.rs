use braise_core::{BinaryOperator, Expression, InterpolationPart, UnaryOperator};
use tower_lsp::lsp_types::{Position, Range};

pub fn span_to_range(span: &braise_core::Span) -> Range {
    let start_line = span.start.line.saturating_sub(1) as usize;
    let start_char = span.start.column.saturating_sub(1);
    let end_line = span.end.line.saturating_sub(1) as usize;
    let end_char = span.end.column.saturating_sub(1);

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

pub fn get_expression_preview(expr: &Expression) -> String {
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
        Expression::ModuleAccess { module, field } => {
            format!("{module}.{field}")
        }
        Expression::Interpolation(parts) => {
            let preview: Vec<String> = parts
                .iter()
                .take(3)
                .map(|part| match part {
                    InterpolationPart::String(s) => s.clone(),
                    InterpolationPart::Expression(expr) => {
                        format!("${{{}}}", get_expression_preview(&expr.value))
                    }
                })
                .collect();

            let result = preview.join("");
            if parts.len() > 3 {
                format!("\"{result}...\"")
            } else {
                format!("\"{result}\"")
            }
        }
        Expression::Array(elements) => {
            if elements.is_empty() {
                "[]".to_string()
            } else if elements.len() == 1 {
                format!("[{}]", get_expression_preview(&elements[0].value))
            } else {
                format!("[{}, ...]", get_expression_preview(&elements[0].value))
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
                get_expression_preview(&left.value),
                op_str,
                get_expression_preview(&right.value)
            )
        }
        Expression::UnaryOp { op, expr } => {
            let op_str = match op {
                UnaryOperator::Not => "!",
            };
            format!("{}{}", op_str, get_expression_preview(&expr.value))
        }
        Expression::Conditional { condition, .. } => {
            format!("if {}", get_expression_preview(&condition.value))
        }
        Expression::RecipeRef { recipe, args } => {
            let args_preview: Vec<String> = args
                .iter()
                .map(|(k, arg)| format!("{k}: {}", get_expression_preview(&arg.value)))
                .collect();
            if args_preview.is_empty() {
                recipe.clone()
            } else {
                format!("@{}({})", recipe, args_preview.join(", "))
            }
        }
        Expression::Match { expr, arms } => {
            let expr_preview = get_expression_preview(&expr.value);
            let arms_count = arms.len();
            if arms_count == 0 {
                format!("match {expr_preview} {{}}")
            } else {
                format!("match {expr_preview} with {arms_count} arms")
            }
        }
    }
}

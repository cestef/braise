use core::{BraiseType, TypedValue};
use std::{collections::HashMap, sync::Arc};

use braise_lexer::tokenize;
use braise_parser::Parser;
use braise_runtime::{Runtime, StringExecutor};

#[cfg(test)]
mod simple;

#[cfg(test)]
mod advanced;

pub fn execute_recipe(
    code: &str,
    recipe: &str,
    params: HashMap<String, TypedValue>,
) -> Result<String, Box<dyn std::error::Error>> {
    let tokens = tokenize(code)?;
    let source = Arc::new(code.to_string());
    let mut parser = Parser::new(tokens, source.clone(), "test.braise".to_string());
    let ast = parser.parse()?;
    let runtime = Runtime::new(ast, source).with_executor(StringExecutor::new(false));
    runtime.execute_recipe(recipe, params)?;
    Ok(runtime.executor.output().unwrap_or_default())
}

pub fn string_param(value: &str) -> TypedValue {
    TypedValue::new(value.to_string(), BraiseType::String)
}

pub fn number_param(value: f64) -> TypedValue {
    TypedValue::new(value, BraiseType::Number)
}

pub fn bool_param(value: bool) -> TypedValue {
    TypedValue::new(value, BraiseType::Bool)
}

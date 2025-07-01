use crate::parser::ast::{
    BinaryOperator, Config, Expression, InterpolationPart, MatchPattern, ParamType, Parameter,
    Recipe, Statement, UnaryOperator,
};
use std::collections::HashMap;
use std::process::Command;

pub mod error;
pub use error::*;

mod value;
use value::*;

mod context;
use context::*;

mod modules;
use modules::*;

pub struct Runtime {
    config: Config,
    builtins: BuiltinModules,
    dry_run: bool,
}

impl Runtime {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            builtins: BuiltinModules::new(),
            dry_run: false,
        }
    }

    pub fn with_dry_run(config: Config) -> Self {
        Self {
            config,
            builtins: BuiltinModules::new(),
            dry_run: true,
        }
    }

    pub fn execute_recipe(&self, name: &str, user_params: HashMap<String, String>) -> Result<()> {
        let recipe = self
            .config
            .recipes
            .iter()
            .find(|r| r.name == name)
            .ok_or_else(|| RuntimeError::UndefinedRecipe(name.to_string()))?;

        // First, execute dependencies
        for dep in &recipe.dependencies {
            self.execute_recipe(dep, HashMap::new())?;
        }

        // Resolve parameters
        let mut context = self.resolve_parameters(recipe, user_params)?;

        if self.dry_run {
            println!("🔍 Dry run for recipe: {}", name);
        } else {
            println!("🔥 Running recipe: {}", name);
        }

        // Execute recipe body
        for statement in &recipe.body {
            self.execute_statement(statement, &mut context)?;
        }

        Ok(())
    }

    fn resolve_parameters(
        &self,
        recipe: &Recipe,
        user_params: HashMap<String, String>,
    ) -> Result<ExecutionContext> {
        let mut context = ExecutionContext::new();

        for param in &recipe.parameters {
            let value = if let Some(user_value) = user_params.get(&param.name) {
                // User provided a value - validate and convert it
                self.validate_and_convert_parameter(param, user_value)?
            } else if let Some(default_expr) = &param.default {
                // Use default value
                self.evaluate_expression(default_expr, &context)?
            } else {
                return Err(RuntimeError::InvalidParameter {
                    name: param.name.clone(),
                    expected: format!("{:?}", param.param_type),
                    got: "no value provided and no default".to_string(),
                });
            };

            context.set(param.name.clone(), value);
        }

        Ok(context)
    }

    fn validate_and_convert_parameter(&self, param: &Parameter, user_value: &str) -> Result<Value> {
        match &param.param_type {
            ParamType::String => Ok(Value::String(user_value.to_string())),
            ParamType::Number => user_value.parse::<f64>().map(Value::Number).map_err(|_| {
                RuntimeError::InvalidParameter {
                    name: param.name.clone(),
                    expected: "number".to_string(),
                    got: user_value.to_string(),
                }
            }),
            ParamType::Bool => match user_value.to_lowercase().as_str() {
                "true" | "1" | "yes" => Ok(Value::Bool(true)),
                "false" | "0" | "no" => Ok(Value::Bool(false)),
                _ => Err(RuntimeError::InvalidParameter {
                    name: param.name.clone(),
                    expected: "boolean (true/false)".to_string(),
                    got: user_value.to_string(),
                }),
            },
            ParamType::Enum(variants) => {
                if variants.contains(&user_value.to_string()) {
                    Ok(Value::String(user_value.to_string()))
                } else {
                    Err(RuntimeError::InvalidParameter {
                        name: param.name.clone(),
                        expected: format!("one of: {}", variants.join(", ")),
                        got: user_value.to_string(),
                    })
                }
            }
            ParamType::Array(_) => {
                // Simple comma-separated parsing for now
                let items: Vec<Value> = user_value
                    .split(',')
                    .map(|s| Value::String(s.trim().to_string()))
                    .collect();
                Ok(Value::Array(items))
            }
        }
    }

    fn execute_statement(
        &self,
        statement: &Statement,
        context: &mut ExecutionContext,
    ) -> Result<()> {
        match statement {
            Statement::Run(expr) => {
                let command_str = self.evaluate_expression(expr, context)?.to_string();
                if self.dry_run {
                    self.show_command(&command_str);
                } else {
                    self.run_command(&command_str)?;
                }
            }
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                let condition_value = self.evaluate_expression(condition, context)?;
                if condition_value.to_bool() {
                    for stmt in then_block {
                        self.execute_statement(stmt, context)?;
                    }
                } else if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.execute_statement(stmt, context)?;
                    }
                }
            }
            Statement::Match { expr, arms } => {
                let match_value = self.evaluate_expression(expr, context)?;
                let match_str = match_value.to_string();

                for arm in arms {
                    let matches = match &arm.pattern {
                        MatchPattern::String(pattern) => pattern == &match_str,
                        MatchPattern::Wildcard => true,
                    };

                    if matches {
                        for stmt in &arm.body {
                            self.execute_statement(stmt, context)?;
                        }
                        break; // Only execute first matching arm
                    }
                }
            }
            Statement::For {
                var,
                iterable,
                body,
                is_async,
            } => {
                let iterable_value = self.evaluate_expression(iterable, context)?;

                if let Value::Array(items) = iterable_value {
                    if *is_async {
                        if self.dry_run {
                            println!("  → Would run {} iterations in parallel", items.len());
                        } else {
                            println!("  → Running {} iterations in parallel", items.len());
                        }
                        // For now, just run sequentially - true async would need tokio
                        for item in items {
                            let mut loop_context = context.clone();
                            loop_context.set(var.clone(), item);
                            for stmt in body {
                                self.execute_statement(stmt, &mut loop_context)?;
                            }
                        }
                    } else {
                        for item in items {
                            context.set(var.clone(), item);
                            for stmt in body {
                                self.execute_statement(stmt, context)?;
                            }
                        }
                    }
                } else {
                    return Err(RuntimeError::TypeError(format!(
                        "Cannot iterate over {:?}",
                        iterable_value
                    )));
                }
            }
            Statement::Exit(code_expr) => {
                let code_value = self.evaluate_expression(code_expr, context)?;
                let exit_code = code_value.to_number()? as i32;
                if self.dry_run {
                    println!("  ⚡ exit {}", exit_code);
                } else {
                    return Err(RuntimeError::Exit(exit_code));
                }
            }
        }
        Ok(())
    }

    fn evaluate_expression(&self, expr: &Expression, context: &ExecutionContext) -> Result<Value> {
        match expr {
            Expression::String(s) => Ok(Value::String(s.clone())),
            Expression::Number(n) => Ok(Value::Number(*n)),
            Expression::Bool(b) => Ok(Value::Bool(*b)),
            Expression::Variable(name) => context
                .get(name)
                .cloned()
                .ok_or_else(|| RuntimeError::UndefinedVariable(name.clone())),
            Expression::FunctionCall {
                module,
                function,
                args,
            } => {
                let arg_values: Result<Vec<Value>> = args
                    .iter()
                    .map(|arg| self.evaluate_expression(arg, context))
                    .collect();
                let arg_values = arg_values?;

                self.builtins.call_function(module, function, arg_values)
            }
            Expression::ModuleAccess { module, field } => self.builtins.get_field(module, field),
            Expression::Interpolation(parts) => {
                let mut result = String::new();
                for part in parts {
                    match part {
                        InterpolationPart::String(s) => result.push_str(s),
                        InterpolationPart::Expression(expr) => {
                            let value = self.evaluate_expression(expr, context)?;
                            result.push_str(&value.to_string());
                        }
                    }
                }
                Ok(Value::String(result))
            }
            Expression::Array(elements) => {
                let values: Result<Vec<Value>> = elements
                    .iter()
                    .map(|elem| self.evaluate_expression(elem, context))
                    .collect();
                Ok(Value::Array(values?))
            }
            Expression::BinaryOp { left, op, right } => {
                let left_val = self.evaluate_expression(left, context)?;
                let right_val = self.evaluate_expression(right, context)?;
                self.evaluate_binary_op(&left_val, op, &right_val)
            }
            Expression::UnaryOp { op, expr } => {
                let value = self.evaluate_expression(expr, context)?;
                match op {
                    UnaryOperator::Not => Ok(Value::Bool(!value.to_bool())),
                }
            }
            Expression::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                let condition_value = self.evaluate_expression(condition, context)?;
                if condition_value.to_bool() {
                    self.evaluate_expression(then_expr, context)
                } else {
                    self.evaluate_expression(else_expr, context)
                }
            }
        }
    }

    fn evaluate_binary_op(
        &self,
        left: &Value,
        op: &BinaryOperator,
        right: &Value,
    ) -> Result<Value> {
        match op {
            BinaryOperator::Equal => Ok(Value::Bool(self.values_equal(left, right))),
            BinaryOperator::NotEqual => Ok(Value::Bool(!self.values_equal(left, right))),
            BinaryOperator::Less => {
                let left_num = left.to_number()?;
                let right_num = right.to_number()?;
                Ok(Value::Bool(left_num < right_num))
            }
            BinaryOperator::LessEqual => {
                let left_num = left.to_number()?;
                let right_num = right.to_number()?;
                Ok(Value::Bool(left_num <= right_num))
            }
            BinaryOperator::Greater => {
                let left_num = left.to_number()?;
                let right_num = right.to_number()?;
                Ok(Value::Bool(left_num > right_num))
            }
            BinaryOperator::GreaterEqual => {
                let left_num = left.to_number()?;
                let right_num = right.to_number()?;
                Ok(Value::Bool(left_num >= right_num))
            }
            BinaryOperator::And => Ok(Value::Bool(left.to_bool() && right.to_bool())),
            BinaryOperator::Or => Ok(Value::Bool(left.to_bool() || right.to_bool())),
        }
    }

    fn values_equal(&self, left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => {
                a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| self.values_equal(x, y))
            }
            // Type coercion for mixed comparisons
            (Value::String(s), Value::Number(n)) | (Value::Number(n), Value::String(s)) => {
                s.parse::<f64>().map(|parsed| parsed == *n).unwrap_or(false)
            }
            _ => false,
        }
    }

    fn show_command(&self, command: &str) {
        println!("  ⚡ {}", command);
    }

    fn run_command(&self, command: &str) -> Result<()> {
        println!("  → {}", command);

        let mut cmd = if cfg!(target_os = "windows") {
            let mut cmd = Command::new("cmd");
            cmd.args(["/C", command]);
            cmd
        } else {
            let mut cmd = Command::new("sh");
            cmd.args(["-c", command]);
            cmd
        };

        let output = cmd
            .output()
            .map_err(|e| RuntimeError::Other(format!("Failed to execute command: {}", e)))?;

        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stderr.is_empty() {
                eprintln!("Command failed: {}", stderr);
            }
            return Err(RuntimeError::CommandFailed {
                command: command.to_string(),
                exit_code,
            });
        }

        // Print command output
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.trim().is_empty() {
            print!("{}", stdout);
        }

        Ok(())
    }
}

// Add clone for ExecutionContext
impl Clone for ExecutionContext {
    fn clone(&self) -> Self {
        Self {
            variables: self.variables.clone(),
        }
    }
}

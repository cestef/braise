use core::ast::*;
use core::error::runtime::*;
use rayon::prelude::*;
use std::collections::HashMap;
use std::process::Command;
use std::sync::{Arc, Mutex};

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

    pub fn with_dry_run(mut self) -> Self {
        self.dry_run = true;
        self
    }

    pub fn execute_recipe(&self, name: &str, user_params: HashMap<String, String>) -> Result<()> {
        let recipe = self
            .config
            .recipes
            .iter()
            .find(|r| r.value.name == name)
            .ok_or_else(|| RuntimeError::UndefinedRecipe(name.to_string()))?;

        for dep in &recipe.value.dependencies {
            self.execute_recipe(dep, HashMap::new())?;
        }

        let mut context = self.resolve_parameters(&recipe.value, user_params)?;

        if self.dry_run {
            println!("🔍 Dry run for recipe: {}", name);
        } else {
            println!("🔥 Running recipe: {}", name);
        }

        for statement in &recipe.value.body {
            self.execute_statement(&statement.value, &mut context)?;
        }

        Ok(())
    }

    fn validate_and_convert_parameter(&self, param: &Parameter, user_value: &str) -> Result<Value> {
        if let Err(validation_error) = param.param_type.validate_string_value(user_value) {
            return Err(RuntimeError::InvalidParameter {
                name: param.name.clone(),
                expected: param.param_type.to_string(),
                got: format!("{} ({})", user_value, validation_error),
            });
        }

        match &param.param_type {
            ParamType::String => Ok(Value::String(user_value.to_string())),

            ParamType::Number => user_value.parse::<f64>().map(Value::Number).map_err(|_| {
                RuntimeError::InvalidParameter {
                    name: param.name.clone(),
                    expected: "number".to_string(),
                    got: user_value.to_string(),
                }
            }),

            ParamType::Bool => {
                let bool_value = match user_value.to_lowercase().as_str() {
                    "true" | "1" | "yes" => true,
                    "false" | "0" | "no" => false,
                    _ => {
                        return Err(RuntimeError::InvalidParameter {
                            name: param.name.clone(),
                            expected: "boolean (true/false)".to_string(),
                            got: user_value.to_string(),
                        });
                    }
                };
                Ok(Value::Bool(bool_value))
            }

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

            ParamType::Array(element_type) => {
                let items: Result<Vec<Value>> = user_value
                    .split(',')
                    .map(|s| {
                        let trimmed = s.trim();
                        let temp_param = Parameter {
                            name: format!("{}_element", param.name),
                            param_type: (**element_type).clone(),
                            default: None,
                        };
                        self.validate_and_convert_parameter(&temp_param, trimmed)
                    })
                    .collect();

                match items {
                    Ok(values) => Ok(Value::Array(values)),
                    Err(e) => Err(RuntimeError::InvalidParameter {
                        name: param.name.clone(),
                        expected: format!("array of {}", element_type),
                        got: format!("{} (error in array element: {})", user_value, e),
                    }),
                }
            }
        }
    }

    fn resolve_parameters(
        &self,
        recipe: &Recipe,
        user_params: HashMap<String, String>,
    ) -> Result<ExecutionContext> {
        let mut context = ExecutionContext::new();
        let mut provided_params = user_params.clone();

        for param in &recipe.parameters {
            let value = if let Some(user_value) = provided_params.remove(&param.value.name) {
                self.validate_and_convert_parameter(&param.value, &user_value)?
            } else if let Some(default_expr) = &param.value.default {
                self.evaluate_expression(&default_expr.value, &context)?
            } else {
                return Err(RuntimeError::InvalidParameter {
                    name: param.value.name.clone(),
                    expected: param.value.param_type.to_string(),
                    got: "no value provided and no default".to_string(),
                });
            };

            self.validate_value_type(&value, &param.value.param_type, &param.value.name)?;

            context.set(param.value.name.clone(), value);
        }

        if !provided_params.is_empty() {
            let unused: Vec<String> = provided_params.keys().cloned().collect();
            return Err(RuntimeError::Other(format!(
                "Unknown parameters: {}. Available parameters: {}",
                unused.join(", "),
                recipe
                    .parameters
                    .iter()
                    .map(|p| format!("{}: {}", p.value.name, p.value.param_type))
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }

        Ok(context)
    }

    fn validate_value_type(
        &self,
        value: &Value,
        expected_type: &ParamType,
        param_name: &str,
    ) -> Result<()> {
        let matches = match (value, expected_type) {
            (Value::String(_), ParamType::String) => true,
            (Value::Number(_), ParamType::Number) => true,
            (Value::Bool(_), ParamType::Bool) => true,
            (Value::Array(arr), ParamType::Array(element_type)) => arr.iter().all(|elem| {
                self.validate_value_type(elem, element_type, param_name)
                    .is_ok()
            }),
            (Value::String(s), ParamType::Enum(variants)) => variants.contains(s),
            _ => false,
        };

        if !matches {
            return Err(RuntimeError::TypeError(format!(
                "Parameter '{}' expected type {}, but got {} value: {}",
                param_name,
                expected_type,
                value.type_name(),
                value.to_string()
            )));
        }

        Ok(())
    }

    fn execute_statement(
        &self,
        statement: &Statement,
        context: &mut ExecutionContext,
    ) -> Result<()> {
        match statement {
            Statement::Run(expr) => {
                let command_str = self.evaluate_expression(&expr.value, context)?.to_string();
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
                let condition_value = self.evaluate_expression(&condition.value, context)?;
                if condition_value.to_bool() {
                    for stmt in then_block {
                        self.execute_statement(&stmt.value, context)?;
                    }
                } else if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.execute_statement(&stmt.value, context)?;
                    }
                }
            }
            Statement::Match { expr, arms } => {
                let match_value = self.evaluate_expression(&expr.value, context)?;
                let match_str = match_value.to_string();

                for arm in arms {
                    let matches = match &arm.value.pattern {
                        MatchPattern::String(pattern) => pattern == &match_str,
                        MatchPattern::Wildcard => true,
                    };

                    if matches {
                        for stmt in &arm.value.body {
                            self.execute_statement(&stmt.value, context)?;
                        }
                        break;
                    }
                }
            }
            Statement::For {
                var,
                iterable,
                body,
                is_async,
            } => {
                let iterable_value = self.evaluate_expression(&iterable.value, context)?;

                if let Value::Array(items) = iterable_value {
                    if *is_async {
                        if self.dry_run {
                            println!("  → Would run {} iterations in parallel", items.len());
                        } else {
                            println!("  → Running {} iterations in parallel", items.len());
                        }

                        let context_arc = Arc::new(Mutex::new(context.clone()));
                        let self_arc = Arc::new(self);

                        let errors = Arc::new(Mutex::new(Vec::new()));

                        items.par_iter().for_each(|item| {
                            let context_clone = context_arc.clone();
                            let self_clone = self_arc.clone();
                            let errors_clone = errors.clone();
                            let var_clone = var.clone();

                            let mut loop_context;
                            {
                                let guard = context_clone.lock().unwrap();
                                loop_context = guard.clone();
                            }
                            loop_context.set(var_clone, item.clone());

                            for stmt in body {
                                if let Err(e) =
                                    self_clone.execute_statement(&stmt.value, &mut loop_context)
                                {
                                    let mut errors_guard = errors_clone.lock().unwrap();
                                    errors_guard.push(e);
                                    break;
                                }
                            }
                        });

                        let errors_guard = errors.lock().unwrap();
                        if let Some(first_error) = errors_guard.first() {
                            return Err(first_error.clone());
                        }
                    } else {
                        for item in items {
                            context.set(var.clone(), item);
                            for stmt in body {
                                self.execute_statement(&stmt.value, context)?;
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
            Statement::Print(expr) => {
                let value = self.evaluate_expression(&expr.value, context)?;
                if self.dry_run {
                    println!("  📣 {}", value.to_string());
                } else {
                    println!("{}", value.to_string());
                }
            }
            Statement::Exit(expr) => {
                let exit_code = {
                    let value = self.evaluate_expression(&expr.value, context)?;
                    value.to_number()? as i32
                };

                if self.dry_run {
                    println!("  ⚡ exit {}", exit_code);
                } else {
                    return Err(RuntimeError::Exit(exit_code));
                }
            }
            Statement::Let {
                name,
                value,
                param_type,
            } => {
                let value = if let Some(expr) = value {
                    self.evaluate_expression(&expr.value, context)?
                } else {
                    Value::default_for_type(param_type)
                };

                self.validate_value_type(&value, param_type, name)?;

                context.set(name.clone(), value);
            }
            Statement::Assign { name, value } => {
                let value = self.evaluate_expression(&value.value, context)?;
                self.validate_value_type(&value, &ParamType::String, name)?;
                if !context.contains(name) {
                    return Err(RuntimeError::UndefinedVariable(name.clone()));
                }
                context.set(name.clone(), value);
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
                    .map(|arg| self.evaluate_expression(&arg.value, context))
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
                            let value = self.evaluate_expression(&expr.value, context)?;
                            result.push_str(&value.to_string());
                        }
                    }
                }
                Ok(Value::String(result))
            }
            Expression::Array(elements) => {
                let values: Result<Vec<Value>> = elements
                    .iter()
                    .map(|elem| self.evaluate_expression(&elem.value, context))
                    .collect();
                Ok(Value::Array(values?))
            }
            Expression::BinaryOp { left, op, right } => {
                let left_val = self.evaluate_expression(&left.value, context)?;
                let right_val = self.evaluate_expression(&right.value, context)?;
                self.evaluate_binary_op(&left_val, op, &right_val)
            }
            Expression::UnaryOp { op, expr } => {
                let value = self.evaluate_expression(&expr.value, context)?;
                match op {
                    UnaryOperator::Not => Ok(Value::Bool(!value.to_bool())),
                }
            }
            Expression::Conditional {
                condition,
                then_expr,
                else_expr,
            } => {
                let condition_value = self.evaluate_expression(&condition.value, context)?;
                if condition_value.to_bool() {
                    self.evaluate_expression(&then_expr.value, context)
                } else {
                    self.evaluate_expression(&else_expr.value, context)
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
            // mixed
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

        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.trim().is_empty() {
            print!("{}", stdout);
        }

        Ok(())
    }
}

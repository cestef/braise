use core::ast::*;
use core::error::runtime::*;
use owo_colors::OwoColorize;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

mod value;
pub use value::*;

mod context;
use context::*;

mod modules;
use modules::*;

mod executor;
pub use executor::*;

pub struct Runtime {
    config: Config,
    builtins: BuiltinModules,
    pub executor: Box<dyn Executor>,
    dry_run: bool,
}

impl Runtime {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            builtins: BuiltinModules::new(),
            executor: Box::new(executor::DefaultExecutor::new(false)),
            dry_run: false,
        }
    }

    pub fn with_executor<E: Executor + 'static>(mut self, executor: E) -> Self {
        self.executor = Box::new(executor);
        self
    }

    pub fn with_dry_run(mut self) -> Self {
        self.executor = Box::new(executor::DefaultExecutor::new(true));
        self.dry_run = true;
        self
    }

    pub fn execute_recipe(&self, name: &str, user_params: HashMap<String, Value>) -> Result<()> {
        let mut stack = HashSet::new();
        self._execute_recipe(name, user_params, &mut stack)
    }

    fn _execute_recipe(
        &self,
        name: &str,
        user_params: HashMap<String, Value>,
        stack: &mut HashSet<String>,
    ) -> Result<()> {
        if !stack.insert(name.to_string()) {
            return Err(RuntimeError::CircularDependency {
                recipe: name.to_string(),
                stack: stack.iter().cloned().collect(),
            });
        }

        let recipe = self
            .config
            .recipes
            .iter()
            .find(|r| r.value.name == name)
            .ok_or_else(|| RuntimeError::UndefinedRecipe(name.to_string()))?;

        for dep in &recipe.value.dependencies {
            self._execute_recipe(dep, HashMap::new(), stack)?; // TODO: maybe pass args to deps ?
        }

        let mut context = self.resolve_parameters(&recipe.value, user_params)?;

        if self.dry_run {
            println!("🔍 Dry run for recipe: {name}");
        } else {
            println!("🔥 Running recipe: {name}");
        }

        for statement in &recipe.value.body {
            self.execute_statement(&statement.value, &mut context)?;
        }

        Ok(())
    }

    fn resolve_parameters(
        &self,
        recipe: &Recipe,
        user_params: HashMap<String, Value>,
    ) -> Result<ExecutionContext> {
        let mut context = ExecutionContext::new();
        let mut provided_params = user_params.clone();

        for param in &recipe.parameters {
            let value = if let Some(user_value) = provided_params.remove(&param.value.name) {
                user_value
            } else if let Some(default_expr) = &param.value.default {
                self.evaluate_expression(&default_expr.value, &context)?
            } else {
                return Err(RuntimeError::MissingRequiredParameter {
                    name: param.value.name.clone(),
                    expected_type: param.value.param_type.to_string(),
                });
            };

            let value = match value {
                Value::String(s) => {
                    Value::from_str(&s, &param.value.name, &param.value.param_type)?
                }
                e => e,
            };

            context.set(param.value.name.clone(), value);
        }

        if !provided_params.is_empty() {
            let unused: Vec<String> = provided_params.keys().cloned().collect();
            return Err(RuntimeError::Other(format!(
                "\n  Unknown parameters: {}.\n  Available parameters: {}",
                unused
                    .iter()
                    .map(|p| p.bold().red().to_string())
                    .collect::<Vec<_>>()
                    .join(", "),
                recipe
                    .parameters
                    .iter()
                    .map(|p| format!("{}: {}", p.value.name.bold(), p.value.param_type.dimmed()))
                    .collect::<Vec<_>>()
                    .join(", ")
            )));
        }

        Ok(context)
    }

    fn try_match_type(
        &self,
        value: &Value,
        expected_type: &ParamType,
        param_name: &str,
    ) -> Result<Value> {
        let converted_value = value.convert_to_type(expected_type, param_name)?;

        Ok(converted_value)
    }

    fn execute_statement(
        &self,
        statement: &Statement,
        context: &mut ExecutionContext,
    ) -> Result<()> {
        match statement {
            Statement::Run(expr) => {
                let value = self.evaluate_expression(&expr.value, context)?;
                let command_str = value.to_string();
                if self.dry_run {
                    self.show_command(&command_str);
                } else {
                    self.run_command(
                        &command_str,
                        context.shell.as_ref().or(self.config.shell.as_ref()),
                    )?;
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
                self.execute_match_statement(expr, arms, context)?;
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
                    return Err(RuntimeError::TypeError {
                        expected: "iterable".to_string(),
                        got: iterable_value.type_name().to_string(),
                        context: format!("For loop variable '{var}'"),
                    });
                }
            }
            Statement::Print(expr) => {
                let value = self.evaluate_expression(&expr.value, context)?;
                if self.dry_run {
                    println!("  📣 {value}");
                } else {
                    println!("{value}");
                }
            }
            Statement::Exit(expr) => {
                let exit_code = {
                    let value = self.evaluate_expression(&expr.value, context)?;
                    value.to_number()? as i32
                };

                if self.dry_run {
                    println!("  ⚡ exit {exit_code}");
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

                let value = self.try_match_type(&value, param_type, name)?;

                context.set(name.clone(), value);
            }
            Statement::Assign { name, value } => {
                let value = self.evaluate_expression(&value.value, context)?;
                // TODO: validate type against existing variable type
                if !context.contains(name) {
                    return Err(RuntimeError::UndefinedVariable(name.clone()));
                }
                context.set(name.clone(), value);
            }
            Statement::Call { recipe, args } => {
                let recipe_value = self.evaluate_expression(&recipe.value, context)?;
                let mut resolved_args = HashMap::new();

                let recipe_name = match &recipe_value {
                    Value::Recipe(name, predefined_args) => {
                        if !predefined_args.is_empty() {
                            resolved_args = predefined_args.clone();
                        }
                        name.clone()
                    }
                    _ => recipe_value.to_string(),
                };

                if resolved_args.is_empty() {
                    for (arg_name, arg_expr) in args {
                        let value = self.evaluate_expression(&arg_expr.value, context)?;
                        resolved_args.insert(arg_name.clone(), value);
                    }
                }

                if self.dry_run {
                    println!("  ⚡ Call recipe: {recipe_name} with args: {resolved_args:?}");
                } else {
                    self.execute_recipe(&recipe_name, resolved_args)?;
                }
            }
            Statement::Shell { name } => {
                context.set_shell(name.clone());
            }
        }
        Ok(())
    }
    fn execute_match_statement(
        &self,
        expr: &SpannedNode<Expression>,
        arms: &[SpannedNode<MatchArm>],
        context: &mut ExecutionContext,
    ) -> Result<()> {
        let match_value = self.evaluate_expression(&expr.value, context)?;

        for arm in arms {
            let pattern_match = self.match_pattern(&arm.value.pattern, &match_value, context)?;

            if pattern_match.matched {
                if let Some(ref guard) = arm.value.guard {
                    let mut guard_context = context.clone();
                    for (name, value) in &pattern_match.bindings {
                        guard_context.set(name.clone(), value.clone());
                    }

                    let guard_result = self.evaluate_expression(&guard.value, &guard_context)?;
                    if !guard_result.to_bool() {
                        continue;
                    }
                }

                for (name, value) in pattern_match.bindings {
                    context.set(name, value);
                }

                for stmt in &arm.value.body {
                    self.execute_statement(&stmt.value, context)?;
                }
                return Ok(());
            }
        }

        Err(RuntimeError::MatchNoArm {
            value: match_value.to_string(),
        })
    }

    fn evaluate_match_expression(
        &self,
        expr: &SpannedNode<Expression>,
        arms: &[SpannedNode<MatchExpressionArm>],
        context: &ExecutionContext,
    ) -> Result<Value> {
        let match_value = self.evaluate_expression(&expr.value, context)?;

        for arm in arms {
            let pattern_match = self.match_pattern(&arm.value.pattern, &match_value, context)?;
            if pattern_match.matched {
                // Check guard condition if present
                if let Some(ref guard) = arm.value.guard {
                    // Create temporary context with pattern bindings
                    let mut guard_context = context.clone();
                    for (name, value) in &pattern_match.bindings {
                        guard_context.set(name.clone(), value.clone());
                    }

                    let guard_result = self.evaluate_expression(&guard.value, &guard_context)?;
                    if !guard_result.to_bool() {
                        continue; // Guard failed, try next arm
                    }
                }

                let mut expr_context = context.clone();
                for (name, value) in pattern_match.bindings {
                    expr_context.set(name, value);
                }

                return self.evaluate_expression(&arm.value.expr.value, &expr_context);
            }
        }

        // No pattern matched
        Err(RuntimeError::MatchNoArm {
            value: match_value.to_string(),
        })
    }

    /// Core pattern matching implementation
    fn match_pattern(
        &self,
        pattern: &MatchPattern,
        value: &Value,
        context: &ExecutionContext,
    ) -> Result<PatternMatch<Value>> {
        match pattern {
            MatchPattern::String(s) => Ok(PatternMatch {
                matched: value.to_string() == *s,
                bindings: HashMap::new(),
            }),

            MatchPattern::Number(n) => match value.to_number() {
                Ok(val) => Ok(PatternMatch {
                    matched: (val - n).abs() < f64::EPSILON,
                    bindings: HashMap::new(),
                }),
                Err(_) => Ok(PatternMatch {
                    matched: false,
                    bindings: HashMap::new(),
                }),
            },

            MatchPattern::Bool(b) => Ok(PatternMatch {
                matched: value.to_bool() == *b,
                bindings: HashMap::new(),
            }),

            MatchPattern::Wildcard => Ok(PatternMatch {
                matched: true,
                bindings: HashMap::new(),
            }),

            MatchPattern::Variable(name) => {
                let mut bindings = HashMap::new();
                bindings.insert(name.clone(), value.clone());
                Ok(PatternMatch {
                    matched: true,
                    bindings,
                })
            }

            MatchPattern::Or(patterns) => {
                for pattern_node in patterns {
                    let result = self.match_pattern(&pattern_node.value, value, context)?;
                    if result.matched {
                        return Ok(result);
                    }
                }
                Ok(PatternMatch {
                    matched: false,
                    bindings: HashMap::new(),
                })
            }

            MatchPattern::Range {
                start,
                end,
                inclusive,
            } => match value.to_number() {
                Ok(val) => {
                    let mut matched = true;

                    if let Some(start_val) = start {
                        matched = matched && val >= *start_val;
                    }

                    if let Some(end_val) = end {
                        if *inclusive {
                            matched = matched && val <= *end_val;
                        } else {
                            matched = matched && val < *end_val;
                        }
                    }

                    Ok(PatternMatch {
                        matched,
                        bindings: HashMap::new(),
                    })
                }
                Err(_) => Ok(PatternMatch {
                    matched: false,
                    bindings: HashMap::new(),
                }),
            },

            MatchPattern::Array { elements, rest } => {
                if let Value::Array(array_values) = value {
                    self.match_array_pattern(elements, rest.as_ref(), array_values, context)
                } else {
                    Ok(PatternMatch {
                        matched: false,
                        bindings: HashMap::new(),
                    })
                }
            }

            MatchPattern::Guard { pattern, condition } => {
                let base_match = self.match_pattern(&pattern.value, value, context)?;
                if !base_match.matched {
                    return Ok(base_match);
                }

                // Create context with pattern bindings for guard evaluation
                let mut guard_context = context.clone();
                for (name, binding_value) in &base_match.bindings {
                    guard_context.set(name.clone(), binding_value.clone());
                }

                let guard_result = self.evaluate_expression(&condition.value, &guard_context)?;
                Ok(PatternMatch {
                    matched: guard_result.to_bool(),
                    bindings: base_match.bindings,
                })
            }

            MatchPattern::Type(param_type) => {
                let matched = matches!(
                    (param_type, value),
                    (ParamType::String, Value::String(_))
                        | (ParamType::Number, Value::Number(_))
                        | (ParamType::Bool, Value::Bool(_))
                        | (ParamType::Array(_), Value::Array(_))
                        | (ParamType::Recipe, Value::Recipe(_, _))
                );

                Ok(PatternMatch {
                    matched,
                    bindings: HashMap::new(),
                })
            }
        }
    }

    /// Match array patterns with element patterns and rest patterns
    fn match_array_pattern(
        &self,
        elements: &[SpannedNode<ArrayPatternElement>],
        rest: Option<&String>,
        array_values: &[Value],
        context: &ExecutionContext,
    ) -> Result<PatternMatch<Value>> {
        let mut bindings = HashMap::new();
        let mut array_index = 0;
        let mut matched = true;

        for element_node in elements {
            match &element_node.value {
                ArrayPatternElement::Pattern(pattern) => {
                    if array_index >= array_values.len() {
                        matched = false;
                        break;
                    }

                    let element_match =
                        self.match_pattern(pattern, &array_values[array_index], context)?;

                    if !element_match.matched {
                        matched = false;
                        break;
                    }

                    bindings.extend(element_match.bindings);
                    array_index += 1;
                }

                ArrayPatternElement::Wildcard => {
                    if array_index >= array_values.len() {
                        matched = false;
                        break;
                    }

                    array_index += 1;
                }

                ArrayPatternElement::Rest(rest_name) => {
                    let remaining: Vec<Value> = array_values[array_index..].to_vec();

                    if let Some(name) = rest_name {
                        bindings.insert(name.clone(), Value::Array(remaining));
                    }

                    array_index = array_values.len();
                    break;
                }
            }
        }

        if rest.is_none() && array_index != array_values.len() {
            matched = false;
        }

        if let Some(rest_name) = rest {
            let remaining: Vec<Value> = array_values[array_index..].to_vec();
            bindings.insert(rest_name.clone(), Value::Array(remaining));
        }

        Ok(PatternMatch { matched, bindings })
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
            Expression::RecipeRef { recipe, args } => {
                let mut arg_values = HashMap::new();
                for (k, arg) in args {
                    let value = self.evaluate_expression(&arg.value, context)?;
                    arg_values.insert(k.clone(), value);
                }

                Ok(Value::Recipe(recipe.clone(), arg_values))
            }
            Expression::Match { expr, arms } => self.evaluate_match_expression(expr, arms, context),
        }
    }

    fn evaluate_binary_op(
        &self,
        left: &Value,
        op: &BinaryOperator,
        right: &Value,
    ) -> Result<Value> {
        match op {
            BinaryOperator::Equal => Ok(Value::Bool(Self::values_equal(left, right))),
            BinaryOperator::NotEqual => Ok(Value::Bool(!Self::values_equal(left, right))),
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

    fn values_equal(left: &Value, right: &Value) -> bool {
        match (left, right) {
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Array(a), Value::Array(b)) => {
                a.len() == b.len()
                    && a.iter()
                        .zip(b.iter())
                        .all(|(x, y)| Self::values_equal(x, y))
            }
            // mixed
            (Value::String(s), Value::Number(n)) | (Value::Number(n), Value::String(s)) => {
                s.parse::<f64>().map(|parsed| parsed == *n).unwrap_or(false)
            }
            _ => false,
        }
    }

    fn show_command(&self, command: &str) {
        println!("  ⚡ {command}");
    }

    fn run_command(&self, command: &str, shell: Option<&String>) -> Result<()> {
        println!("  → {command}");

        self.executor.run(command, shell)
    }
}

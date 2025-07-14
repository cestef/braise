use braise_cache::{CacheConfig, CacheManager};
use core::error::TypeError;
use core::error::runtime::*;
use core::{BraiseType, Spanned, TypedValue, ValueData, ast::*};
use log::{debug, trace, warn};
use owo_colors::OwoColorize;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

mod context;
use context::*;

mod modules;
pub use modules::*;

mod executor;
pub use executor::{Executor, ShellConfig, ShellMode, StringExecutor};

pub struct Runtime {
    config: Config,
    builtins: BuiltinModules,
    pub executor: Box<dyn Executor>,
    pub cache_manager: Option<CacheManager>,
    dry_run: bool,
    quiet: bool,
    source: Arc<String>,
    stack: Arc<RwLock<HashSet<String>>>,
}

impl Runtime {
    pub fn new(config: Config, source: Arc<String>) -> Self {
        Self {
            config,
            builtins: BuiltinModules::new(),
            executor: Box::new(executor::PersistentShellExecutor::new(false, None)),
            cache_manager: None,
            dry_run: false,
            quiet: false,
            source,
            stack: Default::default(),
        }
    }

    pub fn with_shell_config(mut self, shell_config: executor::ShellConfig) -> Self {
        self.executor = Box::new(executor::ConfigurableShellExecutor::new(shell_config));
        self
    }

    pub fn with_executor<E: Executor + 'static>(mut self, executor: E) -> Self {
        self.executor = Box::new(executor);
        self
    }

    pub fn with_dry_run(mut self) -> Self {
        self.executor = Box::new(executor::PersistentShellExecutor::new(true, None));
        self.dry_run = true;
        self
    }

    pub fn with_quiet(mut self) -> Self {
        self.quiet = true;
        self
    }

    pub fn with_cache(mut self, cache_config: CacheConfig) -> Self {
        match CacheManager::new(cache_config.clone()) {
            Ok(cache_manager) => {
                debug!("Cache enabled for runtime");

                // Wrap the executor with caching if command caching is enabled
                if cache_config.command_cache {
                    let caching_executor = executor::CachingExecutor::new(
                        std::mem::replace(
                            &mut self.executor,
                            Box::new(executor::PersistentShellExecutor::new(false, None)),
                        ),
                        cache_manager.clone(),
                        self.quiet,
                    );
                    self.executor = Box::new(caching_executor);
                    debug!("Command caching enabled");
                }

                self.cache_manager = Some(cache_manager);
            }
            Err(e) => {
                warn!("Failed to initialize cache manager: {e}");
                debug!("Runtime will continue without caching");
            }
        }
        self
    }

    pub fn disable_cache(mut self) -> Self {
        self.cache_manager = None;
        self
    }

    pub fn execute_recipe(
        &self,
        name: &str,
        user_params: HashMap<String, TypedValue>,
    ) -> Result<()> {
        debug!(
            "Starting execution of recipe '{}' with {} parameters",
            name,
            user_params.len()
        );
        trace!("Recipe parameters: {user_params:?}");

        {
            let mut stack = self.stack.write().unwrap();
            debug!("Current execution stack: {stack:?}");
            if !stack.insert(name.to_string()) {
                warn!("Circular dependency detected for recipe '{name}'");
                return Err(RuntimeError::circular_dependency(
                    name.to_string(),
                    stack.iter().cloned().collect(),
                )
                .boxed());
            }
        }

        let result = self.execute_recipe_impl(name, user_params);

        {
            let mut stack = self.stack.write().unwrap();
            stack.remove(name);
            debug!("Cleaned up recipe '{name}' from execution stack");
        }

        match &result {
            Ok(_) => debug!("Successfully completed recipe '{name}'"),
            Err(e) => debug!("Recipe '{name}' failed with error: {e}"),
        }

        result
    }

    fn execute_recipe_impl(
        &self,
        name: &str,
        user_params: HashMap<String, TypedValue>,
    ) -> Result<()> {
        let recipe = self
            .config
            .recipes
            .iter()
            .find(|r| r.value.name == name)
            .ok_or_else(|| RuntimeError::undefined_recipe(name.to_string()))?;

        debug!(
            "Found recipe '{}' with {} dependencies and {} statements",
            name,
            recipe.value.dependencies.len(),
            recipe.value.body.len()
        );

        // Check cache if available and cache files are specified
        if let Some(cache_manager) = &self.cache_manager
            && !recipe.value.cache.is_empty()
        {
            if let Ok(Some(cached_result)) =
                self.try_get_cached_recipe(name, &user_params, &recipe.value, cache_manager)
            {
                debug!("Recipe '{name}' found in cache");
                let cache_files = recipe.value.cache.join(", ");
                if !self.quiet {
                    println!(
                        "  {} {} ({})",
                        "→".cyan(),
                        "Using cached result".dimmed(),
                        cache_files.dimmed()
                    );
                }
                return cached_result;
            } else {
                debug!("Recipe '{name}' not found in cache, executing");
                let cache_files = recipe.value.cache.join(", ");
                if !self.quiet {
                    println!(
                        "  {} {} ({})",
                        "→".dimmed(),
                        "Caching recipe result".dimmed(),
                        cache_files.dimmed()
                    );
                }
            }
        }

        if !recipe.value.dependencies.is_empty() {
            debug!("Executing dependencies: {:?}", recipe.value.dependencies);
        }
        for dep in &recipe.value.dependencies {
            debug!("Executing dependency: {dep}");
            self.execute_recipe(dep, HashMap::new())?;
        }

        let mut context = self.resolve_parameters(&recipe.value, &user_params)?;
        debug!("Resolved {} context variables", context.len());

        if !self.quiet {
            println!("{}", name.cyan().bold());
        }

        debug!("Executing {} statements", recipe.value.body.len());
        let execution_result = (|| {
            for (i, statement) in recipe.value.body.iter().enumerate() {
                trace!("Executing statement {}: {:?}", i + 1, statement.value);
                self.execute_statement(statement, &mut context)?;
            }
            Ok::<(), Box<RuntimeError>>(())
        })();

        // Cache the result if cache manager is available and cache files are specified
        if let Some(cache_manager) = &self.cache_manager
            && !recipe.value.cache.is_empty()
        {
            let _ = self.try_cache_recipe_result(
                name,
                &user_params,
                &recipe.value,
                &execution_result,
                cache_manager,
            );
        }

        execution_result
    }

    fn resolve_parameters(
        &self,
        recipe: &Recipe,
        user_params: &HashMap<String, TypedValue>,
    ) -> Result<ExecutionContext> {
        let mut context = ExecutionContext::new();
        let mut provided_params = user_params.clone();

        for param in &recipe.parameters {
            let value = if let Some(user_value) = provided_params.remove(&param.value.name) {
                user_value
                    .convert_to(&param.value.param_type)
                    .map_err(RuntimeError::type_error)?
            } else if let Some(default_expr) = &param.value.default {
                let evaluated = self.evaluate_expression(default_expr, &context)?;

                evaluated.convert_to(&param.value.param_type).map_err(|e| {
                    RuntimeError::type_error_with_context(
                        e,
                        self.source.to_string(),
                        (&default_expr.span).into(),
                    )
                })?
            } else if param.value.optional {
                param.value.param_type.default_value()
            } else {
                return Err(RuntimeError::missing_required_parameter(
                    param.value.name.clone(),
                    param.value.param_type.to_string(),
                )
                .boxed());
            };

            context.set(param.value.name.clone(), value);
        }

        if !provided_params.is_empty() {
            let unused: Vec<String> = provided_params.keys().cloned().collect();
            return Err(RuntimeError::other(format!(
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
            ))
            .boxed());
        }

        Ok(context)
    }

    fn try_match_type(
        &self,
        value: &TypedValue,
        expected_type: &BraiseType,
    ) -> Result<TypedValue, Box<TypeError>> {
        let converted_value = value.convert_to(expected_type)?;

        Ok(converted_value)
    }

    fn execute_statement(
        &self,
        statement: &Spanned<Statement>,
        context: &mut ExecutionContext,
    ) -> Result<()> {
        match &statement.value {
            Statement::Run(expr) => {
                let value = self.evaluate_expression(expr, context)?;
                let command_str = value.to_string();
                debug!("Executing command: {command_str}");
                if self.dry_run {
                    debug!("Dry run mode - showing command");
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
                let condition_value = self.evaluate_expression(condition, context)?;
                debug!("If condition evaluated to: {}", condition_value.to_bool());
                if condition_value.to_bool() {
                    debug!("Executing then block with {} statements", then_block.len());
                    for stmt in then_block {
                        self.execute_statement(stmt, context)?;
                    }
                } else if let Some(else_stmts) = else_block {
                    debug!("Executing else block with {} statements", else_stmts.len());
                    for stmt in else_stmts {
                        self.execute_statement(stmt, context)?;
                    }
                }
            }
            Statement::Match { expr, arms } => {
                debug!("Executing match statement with {} arms", arms.len());
                self.execute_match_statement(expr, arms, context)?;
            }
            Statement::For {
                var,
                iterable,
                body,
                is_async,
            } => {
                let iterable_value = self.evaluate_expression(iterable, context)?;

                if let ValueData::Array(items) = iterable_value.value {
                    debug!(
                        "For loop over {} items, variable: {}, async: {}",
                        items.len(),
                        var,
                        is_async
                    );
                    if *is_async {
                        debug!("Running parallel for loop");
                        if !self.quiet {
                            if self.dry_run {
                                println!(
                                    "  {} Would run {} iterations in parallel",
                                    "‖".dimmed(),
                                    items.len().to_string().dimmed()
                                );
                            } else {
                                println!(
                                    "  {} Running {} iterations in parallel",
                                    "‖".yellow(),
                                    items.len().to_string().bold()
                                );
                            }
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
                                    self_clone.execute_statement(stmt, &mut loop_context)
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
                                self.execute_statement(stmt, context)?;
                            }
                        }
                    }
                } else {
                    return Err(RuntimeError::type_error_with_context(
                        TypeError::mismatch(
                            "iterable",
                            iterable_value.value_type.to_string(),
                            format!("For loop variable '{var}'"),
                        )
                        .boxed(),
                        self.source.to_string(),
                        (&iterable.span).into(),
                    )
                    .boxed());
                }
            }
            Statement::Print(expr) => {
                let value = self.evaluate_expression(expr, context)?;
                if self.dry_run {
                    if !self.quiet {
                        println!("  {} {}", "print".dimmed(), value.to_string().italic());
                    }
                } else {
                    println!("{value}");
                }
            }
            Statement::Exit(expr) => {
                let exit_code = {
                    let value = self.evaluate_expression(expr, context)?;
                    value.to_number().map_err(|_| {
                        RuntimeError::type_error_with_context(
                            TypeError::mismatch(
                                "number",
                                value.value_type.to_string(),
                                "exit code",
                            )
                            .boxed(),
                            self.source.to_string(),
                            (&expr.span).into(),
                        )
                    })? as i32
                };

                if self.dry_run {
                    println!(
                        "  {} {}",
                        "exit".red().bold(),
                        exit_code.to_string().dimmed()
                    );
                } else {
                    return Err(RuntimeError::exit(exit_code).boxed());
                }
            }
            Statement::Let {
                name,
                value: orig_value,
                param_type,
                ..
            } => {
                let value = if let Some(expr) = orig_value {
                    self.evaluate_expression(expr, context)?
                } else {
                    TypedValue::new(ValueData::None, param_type.clone())
                };

                let value = self.try_match_type(&value, param_type).map_err(|e| {
                    RuntimeError::type_error_with_context(
                        e.boxed(),
                        self.source.to_string(),
                        if let Some(expr) = orig_value {
                            (&expr.span).into()
                        } else {
                            (&statement.span).into()
                        },
                    )
                })?;

                context.set(name.clone(), value);
            }
            Statement::Assign { name, value } => {
                let value = self.evaluate_expression(value, context)?;
                if !context.contains(name) {
                    return Err(RuntimeError::undefined_variable(name.clone()).boxed());
                }
                context.set(name.clone(), value);
            }
            Statement::Call { recipe, args } => {
                let recipe_value = self.evaluate_expression(recipe, context)?;
                let mut resolved_args = HashMap::new();

                let recipe_name = match &recipe_value.value {
                    ValueData::Recipe(name, predefined_args) => {
                        if !predefined_args.is_empty() {
                            resolved_args = predefined_args.clone();
                        }
                        name.clone()
                    }
                    _ => recipe_value.to_string(),
                };

                if resolved_args.is_empty() {
                    for (arg_name, arg_expr) in args {
                        let value = self.evaluate_expression(arg_expr, context)?;
                        resolved_args.insert(arg_name.clone(), value);
                    }
                }

                if self.dry_run {
                    println!(
                        "  {} {} {}",
                        "call".purple().bold(),
                        recipe_name.cyan(),
                        format!("{resolved_args:?}").dimmed()
                    );
                } else {
                    self.execute_recipe(&recipe_name, resolved_args)?;
                }
            }
            Statement::Shell { name } => {
                context.set_shell(name.clone());
            }
            Statement::Try {
                try_block,
                catch_block,
                finally_block,
            } => {
                self.execute_try_statement(try_block, catch_block, finally_block, context)?;
            }

            Statement::Throw(expr) => {
                let error_value = self.evaluate_expression(expr, context)?;
                let (message, code) = match &error_value.value {
                    ValueData::Error { message, code } => (message.clone(), *code),
                    _ => (error_value.to_string(), None),
                };

                return Err(RuntimeError::thrown_with_context(
                    message,
                    code,
                    self.source.to_string(),
                    (&expr.span).into(),
                )
                .boxed());
            }
        }
        Ok(())
    }

    fn execute_try_statement(
        &self,
        try_block: &[SpannedNode<Statement>],
        catch_block: &Option<CatchBlock>,
        finally_block: &Option<Vec<SpannedNode<Statement>>>,
        context: &mut ExecutionContext,
    ) -> Result<()> {
        let mut try_result = Ok(());

        for stmt in try_block {
            if let Err(e) = self.execute_statement(stmt, context) {
                try_result = Err(e);
                break;
            }
        }

        if let (Err(error), Some(catch)) = (&try_result, catch_block) {
            let mut catch_context = context.clone();

            if let Some(ref error_var) = catch.error_var {
                let error_value = self.create_error_value(error);
                catch_context.set(error_var.clone(), error_value);
            }

            debug!("Executing catch block for error: {error}");

            // Execute catch block
            for stmt in &catch.body {
                self.execute_statement(stmt, &mut catch_context)?;
            }

            // Update original context with catch context changes (except error var)
            if let Some(ref error_var) = catch.error_var {
                catch_context.variables.remove(error_var);
            }
            *context = catch_context;

            // Clear the error since it was caught
            try_result = Ok(());
        }

        if let Some(finally_stmts) = finally_block {
            debug!("Executing finally block");
            for stmt in finally_stmts {
                // Finally block errors should not be suppressed
                self.execute_statement(stmt, context)?;
            }
        }

        // Return the original try result (which may have been cleared by catch)
        try_result
    }

    fn create_error_value(&self, error: &RuntimeError) -> TypedValue {
        let (message, code) = match error {
            RuntimeError::CommandFailed {
                command, exit_code, ..
            } => (format!("Command failed: {command}"), Some(*exit_code)),
            RuntimeError::Exit { code, message, .. } => (
                message.as_deref().unwrap_or("Exit called").to_string(),
                Some(*code),
            ),
            _ => (error.to_string(), None),
        };

        TypedValue::new(ValueData::Error { message, code }, BraiseType::Error)
    }

    fn execute_match_statement(
        &self,
        expr: &SpannedNode<Expression>,
        arms: &[SpannedNode<MatchArm>],
        context: &mut ExecutionContext,
    ) -> Result<()> {
        let match_value = self.evaluate_expression(expr, context)?;

        for arm in arms {
            let pattern_match = self.match_pattern(&arm.value.pattern, &match_value, context)?;

            if pattern_match.matched {
                if let Some(ref guard) = arm.value.guard {
                    let mut guard_context = context.clone();
                    for (name, value) in &pattern_match.bindings {
                        guard_context.set(name.clone(), value.clone());
                    }

                    let guard_result = self.evaluate_expression(guard, &guard_context)?;
                    if !guard_result.to_bool() {
                        continue;
                    }
                }

                for (name, value) in pattern_match.bindings {
                    context.set(name, value);
                }

                for stmt in &arm.value.body {
                    self.execute_statement(stmt, context)?;
                }
                return Ok(());
            }
        }

        Err(RuntimeError::match_no_arm(match_value.to_string()).boxed())
    }

    fn evaluate_match_expression(
        &self,
        expr: &SpannedNode<Expression>,
        arms: &[SpannedNode<MatchExpressionArm>],
        context: &ExecutionContext,
    ) -> Result<TypedValue> {
        let match_value = self.evaluate_expression(expr, context)?;

        for arm in arms {
            let pattern_match = self.match_pattern(&arm.value.pattern, &match_value, context)?;
            if pattern_match.matched {
                dbg!(&pattern_match.bindings);
                if let Some(ref guard) = arm.value.guard {
                    let mut guard_context = context.clone();
                    for (name, value) in &pattern_match.bindings {
                        guard_context.set(name.clone(), value.clone());
                    }

                    let guard_result = self.evaluate_expression(guard, &guard_context)?;
                    if !guard_result.to_bool() {
                        continue;
                    }
                }

                let mut expr_context = context.clone();
                for (name, value) in pattern_match.bindings {
                    dbg!(&value);
                    expr_context.set(name, value);
                }

                return self.evaluate_expression(&arm.value.expr, &expr_context);
            }
        }

        Err(RuntimeError::match_no_arm(match_value.to_string()).boxed())
    }

    /// Core pattern matching implementation
    fn match_pattern(
        &self,
        pattern: &MatchPattern,
        value: &TypedValue,
        context: &ExecutionContext,
    ) -> Result<PatternMatch<TypedValue>> {
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

            MatchPattern::Array { elements } => {
                if let ValueData::Array(ref array_values) = value.value {
                    self.match_array_pattern(elements, array_values, context)
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

                let mut guard_context = context.clone();
                for (name, binding_value) in &base_match.bindings {
                    guard_context.set(name.clone(), binding_value.clone());
                }

                let guard_result = self.evaluate_expression(condition, &guard_context)?;
                Ok(PatternMatch {
                    matched: guard_result.to_bool(),
                    bindings: base_match.bindings,
                })
            }

            MatchPattern::Type(param_type) => {
                let matched = param_type.is_compatible_with(&value.value_type);

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
        array_values: &[TypedValue],
        context: &ExecutionContext,
    ) -> Result<PatternMatch<TypedValue>> {
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
                    let remaining: Vec<TypedValue> = array_values[array_index..].to_vec();

                    if let Some(name) = rest_name {
                        bindings.insert(
                            name.clone(),
                            TypedValue::new(
                                remaining,
                                BraiseType::Array(Box::new(BraiseType::Any)),
                            ),
                        );
                    }

                    array_index = array_values.len();
                    break;
                }
            }
        }

        if array_index != array_values.len() {
            matched = false;
        }

        Ok(PatternMatch { matched, bindings })
    }

    fn evaluate_expression(
        &self,
        expr: &Spanned<Expression>,
        context: &ExecutionContext,
    ) -> Result<TypedValue> {
        match &expr.value {
            Expression::String(s) => Ok(TypedValue::new(s.clone(), BraiseType::String)),
            Expression::Number(n) => Ok(TypedValue::new(*n, BraiseType::Number)),
            Expression::Bool(b) => Ok(TypedValue::new(*b, BraiseType::Bool)),
            Expression::Variable(name) => context
                .get(name)
                .cloned()
                .ok_or_else(|| RuntimeError::undefined_variable(name.clone()).boxed()),
            Expression::FunctionCall {
                module,
                function,
                args,
                ..
            } => {
                let arg_values: Result<Vec<TypedValue>> = args
                    .iter()
                    .map(|arg| self.evaluate_expression(arg, context))
                    .collect();
                let arg_values = arg_values?;

                self.builtins.call_function(module, function, arg_values)
            }
            Expression::ModuleAccess { module, field, .. } => {
                self.builtins.get_field(module, field)
            }
            Expression::Interpolation(interpolated) => {
                let mut result = String::new();
                for part in &interpolated.parts {
                    match part {
                        InterpolationPart::String(s) => result.push_str(s),
                        InterpolationPart::Expression(expr) => {
                            let value = self.evaluate_expression(expr, context)?;
                            result.push_str(&value.to_string());
                        }
                    }
                }
                Ok(TypedValue::new(result, BraiseType::String))
            }
            Expression::Array(elements) => {
                let values: Result<Vec<TypedValue>> = elements
                    .iter()
                    .map(|elem| self.evaluate_expression(elem, context))
                    .collect();
                let values = values?;
                let array_type = if elements.is_empty() {
                    BraiseType::Array(Box::new(BraiseType::Any))
                } else {
                    let value_types: Vec<BraiseType> =
                        values.iter().map(|v| v.value_type.clone()).collect();
                    let common_type = value_types
                        .into_iter()
                        .reduce(|acc, t| acc.common_type(&t))
                        .unwrap_or(BraiseType::Any);

                    BraiseType::Array(Box::new(common_type))
                };
                Ok(TypedValue::new(values, array_type))
            }
            Expression::BinaryOp {
                left, op, right, ..
            } => {
                let left_val = self.evaluate_expression(left, context)?;
                let right_val = self.evaluate_expression(right, context)?;
                self.evaluate_binary_op(&left_val, op, &right_val)
                    .map_err(|e| {
                        RuntimeError::type_error_with_context(
                            e,
                            self.source.to_string(),
                            (&expr.span).into(),
                        )
                        .boxed()
                    })
            }
            Expression::UnaryOp { op, expr, .. } => {
                let value = self.evaluate_expression(expr, context)?;
                match op {
                    UnaryOperator::Not => Ok(TypedValue::new(!value.to_bool(), BraiseType::Bool)),
                    UnaryOperator::Minus => {
                        let num_value = value.to_number().map_err(|e| {
                            RuntimeError::type_error_with_context(
                                e.boxed(),
                                self.source.to_string(),
                                (&expr.span).into(),
                            )
                        })?;
                        Ok(TypedValue::new(-num_value, BraiseType::Number))
                    }
                }
            }
            Expression::Conditional {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                let condition_value = self.evaluate_expression(condition, context)?;
                if condition_value.to_bool() {
                    self.evaluate_expression(then_expr, context)
                } else {
                    self.evaluate_expression(else_expr, context)
                }
            }
            Expression::RecipeRef { recipe, args } => {
                let mut arg_values = HashMap::new();
                for (k, arg) in args {
                    let value = self.evaluate_expression(arg, context)?;
                    arg_values.insert(k.clone(), value);
                }

                Ok(TypedValue::new(
                    (recipe.clone(), arg_values),
                    BraiseType::Recipe,
                ))
            }
            Expression::Match { expr, arms, .. } => {
                self.evaluate_match_expression(expr, arms, context)
            }
        }
    }

    fn evaluate_binary_op(
        &self,
        left: &TypedValue,
        op: &BinaryOperator,
        right: &TypedValue,
    ) -> Result<TypedValue, Box<TypeError>> {
        match op {
            BinaryOperator::Equal => Ok(TypedValue::new(
                Self::values_equal(left, right),
                BraiseType::Bool,
            )),
            BinaryOperator::NotEqual => Ok(TypedValue::new(
                !Self::values_equal(left, right),
                BraiseType::Bool,
            )),
            BinaryOperator::Less => {
                let left_num = left.to_number()?;
                let right_num = right.to_number()?;
                Ok(TypedValue::new(left_num < right_num, BraiseType::Bool))
            }
            BinaryOperator::LessEqual => {
                let left_num = left.to_number()?;
                let right_num = right.to_number()?;
                Ok(TypedValue::new(left_num <= right_num, BraiseType::Bool))
            }
            BinaryOperator::Greater => {
                let left_num = left.to_number()?;
                let right_num = right.to_number()?;
                Ok(TypedValue::new(left_num > right_num, BraiseType::Bool))
            }
            BinaryOperator::GreaterEqual => {
                let left_num = left.to_number()?;
                let right_num = right.to_number()?;
                Ok(TypedValue::new(left_num >= right_num, BraiseType::Bool))
            }
            BinaryOperator::And => Ok(TypedValue::new(
                left.to_bool() && right.to_bool(),
                BraiseType::Bool,
            )),
            BinaryOperator::Or => Ok(TypedValue::new(
                left.to_bool() || right.to_bool(),
                BraiseType::Bool,
            )),
            BinaryOperator::Plus => {
                let left_num = left.to_number()?;
                let right_num = right.to_number()?;
                Ok(TypedValue::new(left_num + right_num, BraiseType::Number))
            }
            BinaryOperator::Minus => {
                let left_num = left.to_number()?;
                let right_num = right.to_number()?;
                Ok(TypedValue::new(left_num - right_num, BraiseType::Number))
            }
            BinaryOperator::Multiply => {
                let left_num = left.to_number()?;
                let right_num = right.to_number()?;
                Ok(TypedValue::new(left_num * right_num, BraiseType::Number))
            }
            BinaryOperator::Divide => {
                let left_num = left.to_number()?;
                let right_num = right.to_number()?;
                if right_num == 0.0 {
                    return Err(TypeError::division_by_zero().boxed());
                }
                Ok(TypedValue::new(left_num / right_num, BraiseType::Number))
            }
            BinaryOperator::Modulus => {
                let left_num = left.to_number()?;
                let right_num = right.to_number()?;
                if right_num == 0.0 {
                    return Err(TypeError::division_by_zero().boxed());
                }
                Ok(TypedValue::new(left_num % right_num, BraiseType::Number))
            }
            BinaryOperator::Exponent => {
                let left_num = left.to_number()?;
                let right_num = right.to_number()?;
                Ok(TypedValue::new(
                    left_num.powf(right_num),
                    BraiseType::Number,
                ))
            }
        }
    }

    fn values_equal(left: &TypedValue, right: &TypedValue) -> bool {
        if left.value_type == right.value_type {
            return left.value == right.value;
        }
        if matches!(
            (&left.value_type, &right.value_type),
            (BraiseType::String, BraiseType::Number) | (BraiseType::Number, BraiseType::String)
        ) {
            if let Ok(left_num) = left.to_number()
                && let Ok(right_num) = right.to_number()
            {
                return left_num == right_num;
            }
            let left_str = left.to_string();
            if let Ok(right_num) = right.to_number() {
                return left_str.parse::<f64>() == Ok(right_num);
            }
        }
        false
    }

    fn show_command(&self, command: &str) {
        if !self.quiet {
            println!("  {} {}", "→".dimmed(), command.italic());
        }
    }

    fn run_command(&self, command: &str, shell: Option<&String>) -> Result<()> {
        if !self.quiet {
            println!("  {} {}", "→".blue(), command);
        }

        self.executor.run(command, shell)
    }

    // Cache helper methods
    fn try_get_cached_recipe(
        &self,
        name: &str,
        user_params: &HashMap<String, TypedValue>,
        recipe: &Recipe,
        cache_manager: &CacheManager,
    ) -> Result<Option<Result<()>>> {
        // Generate file hashes for cache key
        let file_hashes = self.generate_file_hashes_for_recipe(recipe)?;

        match cache_manager.get_recipe_result(name, user_params, &file_hashes) {
            Ok(Some(cached_result)) => {
                debug!("Cache hit for recipe '{name}'");
                Ok(Some(
                    cached_result.map_err(|e| RuntimeError::other(e).boxed()),
                ))
            }
            Ok(None) => {
                debug!("Cache miss for recipe '{name}'");
                Ok(None)
            }
            Err(e) => {
                warn!("Cache lookup error for recipe '{name}': {e}");
                Ok(None)
            }
        }
    }

    fn try_cache_recipe_result(
        &self,
        name: &str,
        user_params: &HashMap<String, TypedValue>,
        recipe: &Recipe,
        result: &Result<()>,
        cache_manager: &CacheManager,
    ) -> Result<()> {
        // Generate file hashes for cache key
        let file_hashes = self.generate_file_hashes_for_recipe(recipe)?;

        // Create cache dependencies for the recipe
        let dependencies = self.generate_cache_dependencies_for_recipe(recipe);

        // Convert the result for caching
        let cache_result = match result {
            Ok(_) => Ok(()),
            Err(e) => Err(e.to_string()),
        };

        match cache_manager.set_recipe_result(
            name,
            user_params,
            &file_hashes,
            cache_result,
            dependencies,
        ) {
            Ok(_) => {
                debug!("Cached result for recipe '{name}'");
            }
            Err(e) => {
                warn!("Failed to cache result for recipe '{name}': {e}");
            }
        }

        Ok(())
    }

    fn generate_file_hashes_for_recipe(&self, recipe: &Recipe) -> Result<HashMap<PathBuf, String>> {
        use braise_cache::CacheKeyGenerator;
        use std::path::PathBuf;

        let mut file_hashes = HashMap::new();

        // Use recipe's cache directive files if specified, otherwise use common files
        let cache_files = if !recipe.cache.is_empty() {
            recipe.cache.iter().map(PathBuf::from).collect()
        } else {
            // Default common files when no cache directive is specified
            vec![PathBuf::from("Braisefile"), PathBuf::from(".braise")]
        };

        for file_path in cache_files {
            if file_path.exists() {
                match CacheKeyGenerator::hash_file_metadata(&file_path) {
                    Ok(hash) => {
                        file_hashes.insert(file_path, hash);
                    }
                    Err(e) => {
                        debug!("Failed to hash file {file_path:?}: {e}");
                    }
                }
            }
        }

        Ok(file_hashes)
    }

    fn generate_cache_dependencies_for_recipe(
        &self,
        recipe: &Recipe,
    ) -> Vec<braise_cache::CacheDependency> {
        use braise_cache::{CacheDependency, DependencyType};
        use std::path::PathBuf;

        let mut dependencies = Vec::new();

        // Use recipe's cache directive files if specified, otherwise use common files
        let cache_files = if !recipe.cache.is_empty() {
            recipe.cache.iter().map(PathBuf::from).collect()
        } else {
            // Default common files when no cache directive is specified
            vec![PathBuf::from("Braisefile"), PathBuf::from(".braise")]
        };

        for file_path in cache_files {
            if file_path.exists() {
                dependencies.push(CacheDependency {
                    dependency_type: DependencyType::File,
                    path: file_path,
                    hash: String::new(), // Hash will be generated when needed
                });
            }
        }

        dependencies
    }
}

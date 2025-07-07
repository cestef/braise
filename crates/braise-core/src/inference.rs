use crate::{
    BraiseType, Result, TypeChecker, ast::*, error::types::TypeError, runtime::RuntimeError,
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
/// Type inference engine for Braise expressions and statements
pub struct TypeInferenceEngine {
    type_checker: TypeChecker,
    builtin_types: HashMap<String, HashMap<String, BraiseType>>,
}

impl TypeInferenceEngine {
    pub fn new() -> Self {
        let mut builtin_types = HashMap::new();

        let mut env_types = HashMap::new();
        env_types.insert("get".to_string(), BraiseType::String);
        env_types.insert("has".to_string(), BraiseType::Bool);
        env_types.insert("HOME".to_string(), BraiseType::String);
        env_types.insert("PWD".to_string(), BraiseType::String);
        env_types.insert("CI".to_string(), BraiseType::Bool);
        builtin_types.insert("env".to_string(), env_types);

        let mut cpu_types = HashMap::new();
        cpu_types.insert("count".to_string(), BraiseType::Number);
        cpu_types.insert("physical_count".to_string(), BraiseType::Number);
        cpu_types.insert("arch".to_string(), BraiseType::String);
        builtin_types.insert("cpu".to_string(), cpu_types);

        let mut git_types = HashMap::new();
        git_types.insert("branch".to_string(), BraiseType::String);
        git_types.insert("commit_hash".to_string(), BraiseType::String);
        git_types.insert("commit_hash_short".to_string(), BraiseType::String);
        git_types.insert("is_clean".to_string(), BraiseType::Bool);
        git_types.insert("is_dirty".to_string(), BraiseType::Bool);
        git_types.insert("tag".to_string(), BraiseType::String);
        builtin_types.insert("git".to_string(), git_types);

        let mut fs_types = HashMap::new();
        fs_types.insert("exists".to_string(), BraiseType::Bool);
        fs_types.insert("is_file".to_string(), BraiseType::Bool);
        fs_types.insert("is_dir".to_string(), BraiseType::Bool);
        builtin_types.insert("fs".to_string(), fs_types);

        Self {
            type_checker: TypeChecker::new(),
            builtin_types,
        }
    }

    /// Infer types for an entire configuration
    pub fn infer_config(&mut self, config: &mut Config) -> Result<()> {
        for recipe in &config.recipes {
            for param in &recipe.value.parameters {
                self.type_checker
                    .define_variable(param.value.name.clone(), param.value.effective_type());
            }
        }

        for recipe in &mut config.recipes {
            self.infer_recipe(&mut recipe.value)?;
        }

        Ok(())
    }

    /// Infer types for a recipe
    pub fn infer_recipe(&mut self, recipe: &mut Recipe) -> Result<()> {
        let mut recipe_checker = self.type_checker.enter_scope();

        for param in &recipe.parameters {
            recipe_checker.define_variable(param.value.name.clone(), param.value.effective_type());
        }

        for statement in &mut recipe.body {
            self.infer_statement_with_checker(&mut statement.value, &mut recipe_checker)?;
        }

        Ok(())
    }

    /// Infer types for a statement
    fn infer_statement_with_checker(
        &mut self,
        statement: &mut Statement,
        type_checker: &mut TypeChecker,
    ) -> Result<()> {
        match statement {
            Statement::Let {
                name,
                value,
                param_type,
                inferred_type,
            } => {
                let var_type = if let Some(value_expr) = value {
                    let expr_type =
                        self.infer_expression_with_checker(&mut value_expr.value, type_checker)?;

                    if !expr_type.is_compatible_with(param_type) {
                        return Err(RuntimeError::TypeError {
                            source: TypeError::Mismatch {
                                expected: param_type.to_string(),
                                got: expr_type.to_string(),
                                context: format!("variable '{}'", name),
                            },
                            code: None, // TODO: source code ?
                            span: Some((&value_expr.span).into()),
                        }
                        .into());
                    }

                    *inferred_type = Some(expr_type.clone());
                    expr_type
                } else {
                    param_type.clone()
                };

                type_checker.define_variable(name.clone(), var_type);
            }

            Statement::Assign { name, value } => {
                let expr_type =
                    self.infer_expression_with_checker(&mut value.value, type_checker)?;

                if let Some(var_type) = type_checker.get_variable_type(name) {
                    if !expr_type.is_compatible_with(var_type) {
                        return Err(RuntimeError::TypeError {
                            source: TypeError::Mismatch {
                                expected: var_type.to_string(),
                                got: expr_type.to_string(),
                                context: format!("assignment to variable '{}'", name),
                            },
                            code: None, // TODO: source code ?
                            span: Some((&value.span).into()),
                        }
                        .into());
                    }
                } else {
                    return Err(RuntimeError::UndefinedVariable(name.clone()).into());
                }
            }

            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                let condition_type =
                    self.infer_expression_with_checker(&mut condition.value, type_checker)?;

                if !condition_type.can_convert_from(&BraiseType::Bool) {
                    return Err(RuntimeError::TypeError {
                        source: TypeError::Mismatch {
                            expected: "boolean-convertible".to_string(),
                            got: condition_type.to_string(),
                            context: "if condition".to_string(),
                        },
                        code: None, // TODO: source code ?
                        span: Some((&condition.span).into()),
                    }
                    .into());
                }

                for stmt in then_block {
                    self.infer_statement_with_checker(&mut stmt.value, type_checker)?;
                }

                if let Some(else_stmts) = else_block {
                    for stmt in else_stmts {
                        self.infer_statement_with_checker(&mut stmt.value, type_checker)?;
                    }
                }
            }

            Statement::For {
                var,
                iterable,
                body,
                ..
            } => {
                let iterable_type =
                    self.infer_expression_with_checker(&mut iterable.value, type_checker)?;

                let element_type = match iterable_type {
                    BraiseType::Array(element_type) => *element_type,
                    _ => {
                        return Err(RuntimeError::TypeError {
                            source: TypeError::Mismatch {
                                expected: "array".to_string(),
                                got: iterable_type.to_string(),
                                context: "for loop iterable".to_string(),
                            },
                            code: None, // TODO: source code ?
                            span: Some((&iterable.span).into()),
                        }
                        .into());
                    }
                };

                let mut loop_checker = type_checker.enter_scope();
                loop_checker.define_variable(var.clone(), element_type);

                for stmt in body {
                    self.infer_statement_with_checker(&mut stmt.value, &mut loop_checker)?;
                }
            }

            Statement::Match { expr, arms } => {
                let _expr_type =
                    self.infer_expression_with_checker(&mut expr.value, type_checker)?;

                for arm in arms {
                    // TODO: Validate pattern types against expression type
                    for stmt in &mut arm.value.body {
                        self.infer_statement_with_checker(&mut stmt.value, type_checker)?;
                    }
                }
            }

            Statement::Run(expr) | Statement::Print(expr) | Statement::Exit(expr) => {
                self.infer_expression_with_checker(&mut expr.value, type_checker)?;
            }

            Statement::Call { recipe, args } => {
                self.infer_expression_with_checker(&mut recipe.value, type_checker)?;
                for (_, arg) in args {
                    self.infer_expression_with_checker(&mut arg.value, type_checker)?;
                }
            }

            Statement::Shell { .. } => {}
        }

        Ok(())
    }

    /// Infer the type of an expression
    fn infer_expression_with_checker(
        &mut self,
        expression: &mut Expression,
        type_checker: &TypeChecker,
    ) -> Result<BraiseType> {
        let expr_type = match expression {
            Expression::String(_) => BraiseType::String,
            Expression::Number(_) => BraiseType::Number,
            Expression::Bool(_) => BraiseType::Bool,

            Expression::Variable(name) => type_checker
                .get_variable_type(name)
                .cloned()
                .ok_or_else(|| RuntimeError::UndefinedVariable(name.clone()))?,

            Expression::Array(elements) => {
                if elements.is_empty() {
                    BraiseType::Array(Box::new(BraiseType::Any))
                } else {
                    let first_type =
                        self.infer_expression_with_checker(&mut elements[0].value, type_checker)?;
                    let common_type = elements.iter_mut().skip(1).try_fold(
                        first_type,
                        |acc, elem| -> Result<BraiseType> {
                            let elem_type =
                                self.infer_expression_with_checker(&mut elem.value, type_checker)?;
                            Ok(acc.common_type(&elem_type))
                        },
                    )?;
                    BraiseType::Array(Box::new(common_type))
                }
            }

            Expression::FunctionCall {
                module,
                function,
                args,
                return_type,
            } => {
                for arg in args {
                    self.infer_expression_with_checker(&mut arg.value, type_checker)?;
                }

                let func_type = self
                    .builtin_types
                    .get(module)
                    .and_then(|module_types| module_types.get(function))
                    .cloned()
                    .unwrap_or(BraiseType::Any);

                *return_type = Some(func_type.clone());
                func_type
            }

            Expression::ModuleAccess {
                module,
                field,
                field_type,
            } => {
                let field_type_resolved = self
                    .builtin_types
                    .get(module)
                    .and_then(|module_types| module_types.get(field))
                    .cloned()
                    .unwrap_or(BraiseType::Any);

                *field_type = Some(field_type_resolved.clone());
                field_type_resolved
            }

            Expression::BinaryOp {
                left,
                op,
                right,
                result_type,
            } => {
                let left_type =
                    self.infer_expression_with_checker(&mut left.value, type_checker)?;
                let right_type =
                    self.infer_expression_with_checker(&mut right.value, type_checker)?;

                let result_type_resolved = op.result_type(&left_type, &right_type);
                *result_type = Some(result_type_resolved.clone());
                result_type_resolved
            }

            Expression::UnaryOp {
                op,
                expr,
                result_type,
            } => {
                let operand_type =
                    self.infer_expression_with_checker(&mut expr.value, type_checker)?;
                let result_type_resolved = op.result_type(&operand_type);
                *result_type = Some(result_type_resolved.clone());
                result_type_resolved
            }

            Expression::Conditional {
                condition,
                then_expr,
                else_expr,
                result_type,
            } => {
                let condition_type =
                    self.infer_expression_with_checker(&mut condition.value, type_checker)?;
                let then_type =
                    self.infer_expression_with_checker(&mut then_expr.value, type_checker)?;
                let else_type =
                    self.infer_expression_with_checker(&mut else_expr.value, type_checker)?;

                if !condition_type.can_convert_from(&BraiseType::Bool) {
                    return Err(RuntimeError::TypeError {
                        source: TypeError::Mismatch {
                            expected: "boolean-convertible".to_string(),
                            got: condition_type.to_string(),
                            context: "conditional expression condition".to_string(),
                        },
                        code: None, // TODO: source code ?
                        span: Some((&condition.span).into()),
                    }
                    .into());
                }

                let result_type_resolved = then_type.common_type(&else_type);
                *result_type = Some(result_type_resolved.clone());
                result_type_resolved
            }

            Expression::Interpolation(parts) => {
                for part in parts {
                    if let InterpolationPart::Expression(expr) = part {
                        self.infer_expression_with_checker(&mut expr.value, type_checker)?;
                    }
                }
                BraiseType::String
            }

            Expression::RecipeRef { .. } => BraiseType::Recipe,

            Expression::Match {
                expr,
                arms,
                result_type,
            } => {
                let _expr_type =
                    self.infer_expression_with_checker(&mut expr.value, type_checker)?;

                if arms.is_empty() {
                    return Err(RuntimeError::Other(
                        "Match expression must have at least one arm".to_string(),
                    )
                    .into());
                }

                let first_arm_type = self
                    .infer_expression_with_checker(&mut arms[0].value.expr.value, type_checker)?;
                let result_type_resolved = arms.iter_mut().skip(1).try_fold(
                    first_arm_type,
                    |acc, arm| -> Result<BraiseType> {
                        let arm_type = self.infer_expression_with_checker(
                            &mut arm.value.expr.value,
                            type_checker,
                        )?;
                        Ok(acc.common_type(&arm_type))
                    },
                )?;

                *result_type = Some(result_type_resolved.clone());
                result_type_resolved
            }
        };

        Ok(expr_type)
    }

    /// Public interface for expression type inference
    pub fn infer_expression(&mut self, expression: &mut Expression) -> Result<BraiseType> {
        let mut type_checker = self.type_checker.clone(); // TODO: can we avoid cloning?
        self.infer_expression_with_checker(expression, &mut type_checker)
    }

    /// Add a variable to the current context
    pub fn define_variable(&mut self, name: String, var_type: BraiseType) {
        self.type_checker.define_variable(name, var_type);
    }

    /// Get the type of a variable
    pub fn get_variable_type(&self, name: &str) -> Option<&BraiseType> {
        self.type_checker.get_variable_type(name)
    }

    pub fn enter_scope(&mut self) -> TypeInferenceEngine {
        TypeInferenceEngine {
            type_checker: self.type_checker.enter_scope(),
            builtin_types: self.builtin_types.clone(),
        }
    }
}

/// Enhanced type checking for better error messages
pub struct TypeValidator;

impl TypeValidator {
    /// Validate that a value can be used in a specific context
    pub fn validate_usage(
        value_type: &BraiseType,
        expected_type: &BraiseType,
        context: &str,
    ) -> Result<(), TypeError> {
        if !value_type.is_compatible_with(expected_type) {
            let suggestion = Self::suggest_conversion(value_type, expected_type);
            let mut message = format!(
                "Type mismatch in {}: expected {}, got {}",
                context, expected_type, value_type
            );

            if let Some(suggestion) = suggestion {
                message.push_str(&format!(". {}", suggestion));
            }

            return Err(TypeError::Mismatch {
                expected: expected_type.to_string(),
                got: value_type.to_string(),
                context: message,
            }
            .into());
        }
        Ok(())
    }

    /// Suggest possible type conversions
    fn suggest_conversion(from_type: &BraiseType, to_type: &BraiseType) -> Option<String> {
        match (from_type, to_type) {
            (BraiseType::String, BraiseType::Number) => {
                Some("Try using a numeric string like \"42\"".to_string())
            }
            (BraiseType::Number, BraiseType::String) => {
                Some("Numbers are automatically converted to strings when needed".to_string())
            }
            (BraiseType::String, BraiseType::Bool) => {
                Some("Use \"true\"/\"false\", \"1\"/\"0\", or \"yes\"/\"no\"".to_string())
            }
            (BraiseType::Array(_), BraiseType::String) => {
                Some("Arrays can be converted to strings by joining elements".to_string())
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FileId, Position, Span};

    fn dummy_span() -> Span {
        Span::new(Position::new(1, 1, 0), Position::new(1, 1, 0), FileId(0))
    }

    fn dummy_spanned<T>(value: T) -> SpannedNode<T> {
        SpannedNode::new(value, dummy_span())
    }

    #[test]
    fn test_type_inference_basic() {
        let mut engine = TypeInferenceEngine::new();

        let mut string_expr = Expression::String("hello".to_string());
        let string_type = engine.infer_expression(&mut string_expr).unwrap();
        assert_eq!(string_type, BraiseType::String);

        let mut number_expr = Expression::Number(42.0);
        let number_type = engine.infer_expression(&mut number_expr).unwrap();
        assert_eq!(number_type, BraiseType::Number);

        let mut bool_expr = Expression::Bool(true);
        let bool_type = engine.infer_expression(&mut bool_expr).unwrap();
        assert_eq!(bool_type, BraiseType::Bool);
    }

    #[test]
    fn test_type_inference_arrays() {
        let mut engine = TypeInferenceEngine::new();

        let mut array_expr = Expression::Array(vec![
            dummy_spanned(Expression::String("a".to_string())),
            dummy_spanned(Expression::String("b".to_string())),
        ]);
        let array_type = engine.infer_expression(&mut array_expr).unwrap();
        assert_eq!(array_type, BraiseType::Array(Box::new(BraiseType::String)));

        let mut mixed_array = Expression::Array(vec![
            dummy_spanned(Expression::String("a".to_string())),
            dummy_spanned(Expression::Number(42.0)),
        ]);
        let mixed_type = engine.infer_expression(&mut mixed_array).unwrap();
        if let BraiseType::Array(element_type) = mixed_type {
            assert_eq!(*element_type, BraiseType::String);
        } else {
            panic!("Expected array type");
        }
    }

    #[test]
    fn test_type_inference_variables() {
        let mut engine = TypeInferenceEngine::new();

        engine.define_variable("test_var".to_string(), BraiseType::Number);

        let mut var_expr = Expression::Variable("test_var".to_string());
        let var_type = engine.infer_expression(&mut var_expr).unwrap();
        assert_eq!(var_type, BraiseType::Number);

        let mut undefined_expr = Expression::Variable("undefined".to_string());
        let result = engine.infer_expression(&mut undefined_expr);
        assert!(result.is_err());
    }

    #[test]
    fn test_builtin_function_types() {
        let mut engine = TypeInferenceEngine::new();

        let mut func_expr = Expression::FunctionCall {
            module: "cpu".to_string(),
            function: "count".to_string(),
            args: vec![],
            return_type: None,
        };
        let func_type = engine.infer_expression(&mut func_expr).unwrap();
        assert_eq!(func_type, BraiseType::Number);

        if let Expression::FunctionCall { return_type, .. } = func_expr {
            assert_eq!(return_type, Some(BraiseType::Number));
        }
    }

    #[test]
    fn test_binary_operations() {
        let mut engine = TypeInferenceEngine::new();

        let mut comparison = Expression::BinaryOp {
            left: Box::new(dummy_spanned(Expression::Number(1.0))),
            op: BinaryOperator::Less,
            right: Box::new(dummy_spanned(Expression::Number(2.0))),
            result_type: None,
        };
        let comp_type = engine.infer_expression(&mut comparison).unwrap();
        assert_eq!(comp_type, BraiseType::Bool);
    }

    #[test]
    fn test_conditional_expressions() {
        let mut engine = TypeInferenceEngine::new();

        let mut conditional = Expression::Conditional {
            condition: Box::new(dummy_spanned(Expression::Bool(true))),
            then_expr: Box::new(dummy_spanned(Expression::String("yes".to_string()))),
            else_expr: Box::new(dummy_spanned(Expression::String("no".to_string()))),
            result_type: None,
        };
        let cond_type = engine.infer_expression(&mut conditional).unwrap();
        assert_eq!(cond_type, BraiseType::String);
    }

    #[test]
    fn test_type_validation() {
        let result =
            TypeValidator::validate_usage(&BraiseType::String, &BraiseType::String, "test context");
        assert!(result.is_ok());

        let result =
            TypeValidator::validate_usage(&BraiseType::Number, &BraiseType::Bool, "test context");
        assert!(result.is_ok());
    }
}

use crate::{BraiseType, TypeChecker, BuiltinTypeRegistry, TypeError};

/// Type inference engine for Braise expressions and statements
#[derive(Debug, Clone)]
pub struct TypeInferenceEngine {
    type_checker: TypeChecker,
    builtin_registry: BuiltinTypeRegistry,
}

impl TypeInferenceEngine {
    /// Create a new type inference engine
    pub fn new() -> Self {
        Self {
            type_checker: TypeChecker::new(),
            builtin_registry: BuiltinTypeRegistry::new(),
        }
    }

    /// Create with a custom builtin registry
    pub fn with_builtins(builtin_registry: BuiltinTypeRegistry) -> Self {
        Self {
            type_checker: TypeChecker::new(),
            builtin_registry,
        }
    }

    /// Get a reference to the type checker
    pub fn type_checker(&self) -> &TypeChecker {
        &self.type_checker
    }

    /// Get a mutable reference to the type checker
    pub fn type_checker_mut(&mut self) -> &mut TypeChecker {
        &mut self.type_checker
    }

    /// Get a reference to the builtin registry
    pub fn builtin_registry(&self) -> &BuiltinTypeRegistry {
        &self.builtin_registry
    }

    /// Define a variable in the current scope
    pub fn define_variable(&mut self, name: String, var_type: BraiseType) {
        self.type_checker.define_variable(name, var_type);
    }

    /// Get the type of a variable
    pub fn get_variable_type(&self, name: &str) -> Option<&BraiseType> {
        self.type_checker.get_variable_type(name)
    }

    /// Check if a variable is defined
    pub fn is_variable_defined(&self, name: &str) -> bool {
        self.type_checker.is_variable_defined(name)
    }

    /// Enter a new scope
    pub fn enter_scope(&self) -> TypeInferenceEngine {
        TypeInferenceEngine {
            type_checker: self.type_checker.enter_scope(),
            builtin_registry: self.builtin_registry.clone(),
        }
    }

    /// Get the return type of a builtin function
    pub fn get_builtin_function_type(&self, module: &str, function: &str) -> BraiseType {
        self.builtin_registry
            .get_function_type(module, function)
            .cloned()
            .unwrap_or(BraiseType::Any)
    }

    /// Get the type of a builtin field
    pub fn get_builtin_field_type(&self, module: &str, field: &str) -> BraiseType {
        self.builtin_registry
            .get_field_type(module, field)
            .cloned()
            .unwrap_or(BraiseType::Any)
    }

    /// Validate that a function call is valid
    pub fn validate_function_call(
        &self,
        module: &str,
        function: &str,
        _args: &[BraiseType],
    ) -> Result<BraiseType, TypeError> {
        if !self.builtin_registry.has_function(module, function) {
            return Err(TypeError::mismatch(
                "valid function",
                &format!("{}.{}", module, function),
                "function call"
            ));
        }

        // For now, we don't validate argument types for builtin functions
        // This could be extended in the future with function signatures
        let return_type = self.get_builtin_function_type(module, function);
        Ok(return_type)
    }

    /// Validate that a field access is valid
    pub fn validate_field_access(&self, module: &str, field: &str) -> Result<BraiseType, TypeError> {
        if !self.builtin_registry.has_field(module, field) {
            return Err(TypeError::mismatch(
                "valid field",
                &format!("{}.{}", module, field),
                "field access"
            ));
        }

        let field_type = self.get_builtin_field_type(module, field);
        Ok(field_type)
    }

    /// Infer the type of a binary operation
    pub fn infer_binary_operation_type(
        &self,
        left_type: &BraiseType,
        operator: &str,
        right_type: &BraiseType,
    ) -> Result<BraiseType, TypeError> {
        match operator {
            // Arithmetic operations
            "+" | "-" | "*" | "/" | "%" => {
                if left_type.can_convert_from(&BraiseType::Number) 
                    && right_type.can_convert_from(&BraiseType::Number) {
                    Ok(BraiseType::Number)
                } else {
                    Err(TypeError::unsupported_operation(operator, left_type, right_type))
                }
            }

            // Comparison operations
            "<" | ">" | "<=" | ">=" => {
                if left_type.can_convert_from(&BraiseType::Number) 
                    && right_type.can_convert_from(&BraiseType::Number) {
                    Ok(BraiseType::Bool)
                } else {
                    Err(TypeError::unsupported_operation(operator, left_type, right_type))
                }
            }

            // Equality operations
            "==" | "!=" => Ok(BraiseType::Bool),

            // Logical operations
            "&&" | "||" => {
                if left_type.can_convert_from(&BraiseType::Bool) 
                    && right_type.can_convert_from(&BraiseType::Bool) {
                    Ok(BraiseType::Bool)
                } else {
                    Err(TypeError::unsupported_operation(operator, left_type, right_type))
                }
            }

            _ => Err(TypeError::unsupported_operation(operator, left_type, right_type)),
        }
    }

    /// Infer the type of a unary operation
    pub fn infer_unary_operation_type(
        &self,
        operator: &str,
        operand_type: &BraiseType,
    ) -> Result<BraiseType, TypeError> {
        match operator {
            "!" | "not" => {
                if operand_type.can_convert_from(&BraiseType::Bool) {
                    Ok(BraiseType::Bool)
                } else {
                    Err(TypeError::mismatch(
                        "boolean or boolean-convertible",
                        &operand_type.to_string(),
                        "logical negation"
                    ))
                }
            }

            "-" => {
                if operand_type.can_convert_from(&BraiseType::Number) {
                    Ok(BraiseType::Number)
                } else {
                    Err(TypeError::mismatch(
                        "number or number-convertible",
                        &operand_type.to_string(),
                        "numeric negation"
                    ))
                }
            }

            _ => Err(TypeError::mismatch(
                "valid unary operator",
                operator,
                "unary operation"
            )),
        }
    }

    /// Check array element type consistency
    pub fn infer_array_type(&self, element_types: &[BraiseType]) -> BraiseType {
        if element_types.is_empty() {
            return BraiseType::Array(Box::new(BraiseType::Any));
        }

        let mut common_type = element_types[0].clone();
        for element_type in element_types.iter().skip(1) {
            common_type = common_type.common_type(element_type);
        }

        BraiseType::Array(Box::new(common_type))
    }

    /// Validate that a type can be used in a conditional context
    pub fn validate_condition_type(&self, condition_type: &BraiseType) -> Result<(), TypeError> {
        if !condition_type.can_convert_from(&BraiseType::Bool) 
            && !matches!(condition_type, BraiseType::Bool | BraiseType::Any) {
            return Err(TypeError::mismatch(
                "boolean or boolean-convertible",
                &condition_type.to_string(),
                "conditional expression"
            ));
        }
        Ok(())
    }

    /// Validate that a type can be iterated over
    pub fn validate_iterable_type(&self, iterable_type: &BraiseType) -> Result<BraiseType, TypeError> {
        match iterable_type {
            BraiseType::Array(element_type) => Ok((**element_type).clone()),
            BraiseType::String => Ok(BraiseType::String),
            BraiseType::Optional(inner) => match inner.as_ref() {
                BraiseType::Array(element_type) => Ok((**element_type).clone()),
                BraiseType::String => Ok(BraiseType::String),
                _ => Err(TypeError::mismatch(
                    "iterable type (array or string)",
                    &iterable_type.to_string(),
                    "for loop"
                )),
            },
            BraiseType::Any => Ok(BraiseType::Any),
            _ => Err(TypeError::mismatch(
                "iterable type (array or string)",
                &iterable_type.to_string(),
                "for loop"
            )),
        }
    }

    /// Create a type error with context
    pub fn create_type_error(
        &self,
        expected: impl Into<String>,
        got: impl Into<String>,
        context: impl Into<String>,
    ) -> TypeError {
        TypeError::mismatch(expected, got, context)
    }

    /// Create an undefined variable error
    pub fn create_undefined_variable_error(&self, name: &str) -> TypeError {
        TypeError::mismatch("defined variable", name, "variable access")
    }
}

impl Default for TypeInferenceEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inference_engine_creation() {
        let engine = TypeInferenceEngine::new();
        
        // Test builtin function types
        assert_eq!(
            engine.get_builtin_function_type("env", "get"),
            BraiseType::String
        );
        assert_eq!(
            engine.get_builtin_function_type("cpu", "count"),
            BraiseType::Number
        );
        assert_eq!(
            engine.get_builtin_function_type("git", "is_clean"),
            BraiseType::Bool
        );
    }

    #[test]
    fn test_variable_management() {
        let mut engine = TypeInferenceEngine::new();
        
        engine.define_variable("x".to_string(), BraiseType::String);
        assert!(engine.is_variable_defined("x"));
        assert_eq!(engine.get_variable_type("x"), Some(&BraiseType::String));
        
        assert!(!engine.is_variable_defined("y"));
    }

    #[test]
    fn test_scope_management() {
        let mut root = TypeInferenceEngine::new();
        root.define_variable("x".to_string(), BraiseType::String);
        
        let child = root.enter_scope();
        assert!(child.is_variable_defined("x"));
        
        // Root shouldn't see child variables
        assert!(!root.is_variable_defined("y"));
    }

    #[test]
    fn test_binary_operation_inference() {
        let engine = TypeInferenceEngine::new();
        
        // Arithmetic operations
        let result = engine.infer_binary_operation_type(
            &BraiseType::Number,
            "+",
            &BraiseType::Number
        ).unwrap();
        assert_eq!(result, BraiseType::Number);
        
        // Comparison operations
        let result = engine.infer_binary_operation_type(
            &BraiseType::Number,
            "<",
            &BraiseType::Number
        ).unwrap();
        assert_eq!(result, BraiseType::Bool);
        
        // Equality operations
        let result = engine.infer_binary_operation_type(
            &BraiseType::String,
            "==",
            &BraiseType::Number
        ).unwrap();
        assert_eq!(result, BraiseType::Bool);
    }

    #[test]
    fn test_array_type_inference() {
        let engine = TypeInferenceEngine::new();
        
        // Homogeneous array
        let types = vec![BraiseType::String, BraiseType::String];
        let array_type = engine.infer_array_type(&types);
        assert_eq!(array_type, BraiseType::Array(Box::new(BraiseType::String)));
        
        // Mixed array -> Common type (String can accept Number through conversion)
        let types = vec![BraiseType::String, BraiseType::Number];
        let array_type = engine.infer_array_type(&types);
        match array_type {
            BraiseType::Array(inner) => {
                // The common_type of String and Number becomes String (since String is compatible with Number)
                assert_eq!(*inner.as_ref(), BraiseType::String);
            }
            _ => panic!("Expected array type"),
        }
    }

    #[test]
    fn test_validation_functions() {
        let engine = TypeInferenceEngine::new();
        
        // Valid function call
        assert!(engine.validate_function_call("env", "get", &[]).is_ok());
        
        // Invalid function call
        assert!(engine.validate_function_call("env", "nonexistent", &[]).is_err());
        
        // Valid field access
        assert!(engine.validate_field_access("env", "HOME").is_ok());
        
        // Invalid field access
        assert!(engine.validate_field_access("env", "nonexistent").is_err());
    }
}
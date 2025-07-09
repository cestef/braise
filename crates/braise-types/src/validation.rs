use crate::{BraiseType, TypeError, TypeInferenceEngine, TypedValue};

/// Type validation utilities for the Braise language
pub struct TypeValidator {
    engine: TypeInferenceEngine,
}

impl TypeValidator {
    /// Create a new type validator
    pub fn new() -> Self {
        Self {
            engine: TypeInferenceEngine::new(),
        }
    }

    /// Create with a custom inference engine
    pub fn with_engine(engine: TypeInferenceEngine) -> Self {
        Self { engine }
    }

    /// Get a reference to the inference engine
    pub fn engine(&self) -> &TypeInferenceEngine {
        &self.engine
    }

    /// Get a mutable reference to the inference engine
    pub fn engine_mut(&mut self) -> &mut TypeInferenceEngine {
        &mut self.engine
    }

    /// Validate that a value matches the expected type
    pub fn validate_value_type(
        &self,
        value: &TypedValue,
        expected_type: &BraiseType,
        context: &str,
    ) -> Result<(), TypeError> {
        if !value.value_type.is_compatible_with(expected_type) {
            return Err(TypeError::mismatch(
                expected_type.to_string(),
                value.value_type.to_string(),
                context,
            ));
        }
        Ok(())
    }

    /// Validate that two types are compatible
    pub fn validate_type_compatibility(
        &self,
        expected: &BraiseType,
        actual: &BraiseType,
        context: &str,
    ) -> Result<(), TypeError> {
        if !actual.is_compatible_with(expected) {
            return Err(TypeError::mismatch(
                expected.to_string(),
                actual.to_string(),
                context,
            ));
        }
        Ok(())
    }

    /// Validate that a type can be converted to another type
    pub fn validate_type_conversion(
        &self,
        from: &BraiseType,
        to: &BraiseType,
        context: &str,
    ) -> Result<(), TypeError> {
        if !to.can_convert_from(from) {
            return Err(TypeError::mismatch(
                format!("type convertible to {to}"),
                from.to_string(),
                context,
            ));
        }
        Ok(())
    }

    /// Validate parameter default value against parameter type
    pub fn validate_parameter_default(
        &self,
        default_type: &BraiseType,
        param_type: &BraiseType,
        param_name: &str,
    ) -> Result<(), TypeError> {
        if !default_type.is_compatible_with(param_type) {
            return Err(TypeError::mismatch(
                param_type.to_string(),
                default_type.to_string(),
                format!("default value for parameter '{param_name}'"),
            ));
        }
        Ok(())
    }

    /// Validate array element types
    pub fn validate_array_elements(
        &self,
        element_types: &[BraiseType],
        expected_element_type: &BraiseType,
        context: &str,
    ) -> Result<(), TypeError> {
        for (i, element_type) in element_types.iter().enumerate() {
            if !element_type.is_compatible_with(expected_element_type) {
                return Err(TypeError::mismatch(
                    expected_element_type.to_string(),
                    element_type.to_string(),
                    format!("{context} element {i}"),
                ));
            }
        }
        Ok(())
    }

    /// Validate that a function call has correct argument types
    pub fn validate_function_arguments(
        &self,
        module: &str,
        function: &str,
        arg_types: &[BraiseType],
    ) -> Result<BraiseType, TypeError> {
        // For now, we use the inference engine's validation
        // This could be extended with more sophisticated argument checking
        self.engine
            .validate_function_call(module, function, arg_types)
    }

    /// Validate that a field access is valid
    pub fn validate_field_access(
        &self,
        module: &str,
        field: &str,
    ) -> Result<BraiseType, TypeError> {
        self.engine.validate_field_access(module, field)
    }

    /// Validate conditional expression type
    pub fn validate_conditional_type(
        &self,
        condition_type: &BraiseType,
        context: &str,
    ) -> Result<(), TypeError> {
        if !condition_type.can_convert_from(&BraiseType::Bool)
            && !matches!(condition_type, BraiseType::Bool | BraiseType::Any)
        {
            return Err(TypeError::mismatch(
                "boolean or boolean-convertible",
                condition_type.to_string(),
                context,
            ));
        }
        Ok(())
    }

    /// Validate that a type is iterable and return element type
    pub fn validate_iterable_type(
        &self,
        iterable_type: &BraiseType,
        context: &str,
    ) -> Result<BraiseType, TypeError> {
        match iterable_type {
            BraiseType::Array(element_type) => Ok((**element_type).clone()),
            BraiseType::String => Ok(BraiseType::String),
            BraiseType::Optional(inner) => match inner.as_ref() {
                BraiseType::Array(element_type) => Ok((**element_type).clone()),
                BraiseType::String => Ok(BraiseType::String),
                _ => Err(TypeError::mismatch(
                    "iterable type (array or string)",
                    iterable_type.to_string(),
                    context,
                )),
            },
            BraiseType::Any => Ok(BraiseType::Any),
            _ => Err(TypeError::mismatch(
                "iterable type (array or string)",
                iterable_type.to_string(),
                context,
            )),
        }
    }

    /// Validate union type consistency
    pub fn validate_union_type(
        &self,
        types: &[BraiseType],
        context: &str,
    ) -> Result<(), TypeError> {
        if types.is_empty() {
            return Err(TypeError::mismatch(
                "non-empty type list",
                "empty list",
                format!("{context} union type"),
            ));
        }

        // Check for redundant types in union
        for (i, type1) in types.iter().enumerate() {
            for (j, type2) in types.iter().enumerate() {
                if i != j && type1.is_compatible_with(type2) {
                    // This is a warning rather than an error, but we can note it
                    // For now, we allow redundant types
                }
            }
        }

        Ok(())
    }

    /// Validate enum variants
    pub fn validate_enum_variants(
        &self,
        variants: &[String],
        context: &str,
    ) -> Result<(), TypeError> {
        if variants.is_empty() {
            return Err(TypeError::mismatch(
                "non-empty variant list",
                "empty list",
                format!("{context} enum type"),
            ));
        }

        // Check for duplicate variants
        for (i, variant1) in variants.iter().enumerate() {
            for (j, variant2) in variants.iter().enumerate() {
                if i != j && variant1 == variant2 {
                    return Err(TypeError::mismatch(
                        "unique enum variants",
                        format!("duplicate variant '{variant1}'"),
                        format!("{context} enum type"),
                    ));
                }
            }
        }

        Ok(())
    }

    /// Comprehensive type validation for complex expressions
    pub fn validate_expression_type(
        &self,
        actual_type: &BraiseType,
        expected_type: Option<&BraiseType>,
        context: &str,
    ) -> Result<BraiseType, TypeError> {
        if let Some(expected) = expected_type {
            self.validate_type_compatibility(expected, actual_type, context)?;
            Ok(expected.clone())
        } else {
            Ok(actual_type.clone())
        }
    }

    /// Generate helpful error messages with suggestions
    pub fn create_helpful_error(
        &self,
        expected: &BraiseType,
        actual: &BraiseType,
        context: &str,
    ) -> TypeError {
        let error = TypeError::mismatch(expected.to_string(), actual.to_string(), context);

        // Add suggestions based on common type mismatches
        if let Some(_suggestion) = self.suggest_fix(expected, actual) {
            // Note: The suggestion would be added to the error if TypeError supported it
            // For now, we just return the basic error
        }

        error
    }

    /// Suggest fixes for common type errors
    fn suggest_fix(&self, expected: &BraiseType, actual: &BraiseType) -> Option<String> {
        match (expected, actual) {
            (BraiseType::Number, BraiseType::String) => {
                Some("Try parsing the string as a number or use a numeric literal".to_string())
            }
            (BraiseType::Bool, BraiseType::String) => Some(
                "Use \"true\", \"false\", \"1\", \"0\", \"yes\", \"no\", \"on\", or \"off\""
                    .to_string(),
            ),
            (BraiseType::Array(_), _) => {
                Some("Use array syntax like [\"item1\", \"item2\"]".to_string())
            }
            (BraiseType::String, BraiseType::Number) => {
                Some("Numbers are automatically converted to strings".to_string())
            }
            _ => None,
        }
    }
}

impl Default for TypeValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TypedValue;

    #[test]
    fn test_value_type_validation() {
        let validator = TypeValidator::new();

        let string_value = TypedValue::new("hello", BraiseType::String);

        // Valid validation
        assert!(
            validator
                .validate_value_type(&string_value, &BraiseType::String, "test")
                .is_ok()
        );

        // Invalid validation
        assert!(
            validator
                .validate_value_type(&string_value, &BraiseType::Number, "test")
                .is_err()
        );
    }

    #[test]
    fn test_type_compatibility_validation() {
        let validator = TypeValidator::new();

        // Compatible types
        assert!(
            validator
                .validate_type_compatibility(&BraiseType::String, &BraiseType::String, "test")
                .is_ok()
        );

        // Incompatible types
        assert!(
            validator
                .validate_type_compatibility(
                    &BraiseType::Number,
                    &BraiseType::Array(Box::new(BraiseType::String)),
                    "test"
                )
                .is_err()
        );
    }

    #[test]
    fn test_parameter_default_validation() {
        let validator = TypeValidator::new();

        // Valid default
        assert!(
            validator
                .validate_parameter_default(&BraiseType::String, &BraiseType::String, "param")
                .is_ok()
        );

        // Invalid default (Array is not compatible with String)
        assert!(
            validator
                .validate_parameter_default(
                    &BraiseType::Array(Box::new(BraiseType::String)),
                    &BraiseType::String,
                    "param"
                )
                .is_err()
        );
    }

    #[test]
    fn test_array_validation() {
        let validator = TypeValidator::new();

        let element_types = vec![BraiseType::String, BraiseType::String];

        // Valid array elements
        assert!(
            validator
                .validate_array_elements(&element_types, &BraiseType::String, "test array")
                .is_ok()
        );

        // Invalid array elements
        assert!(
            validator
                .validate_array_elements(&element_types, &BraiseType::Number, "test array")
                .is_err()
        );
    }

    #[test]
    fn test_conditional_validation() {
        let validator = TypeValidator::new();

        // Valid conditional types
        assert!(
            validator
                .validate_conditional_type(&BraiseType::Bool, "if condition")
                .is_ok()
        );
        assert!(
            validator
                .validate_conditional_type(&BraiseType::String, "if condition")
                .is_ok()
        ); // Can convert to bool

        // Invalid conditional type
        assert!(
            validator
                .validate_conditional_type(
                    &BraiseType::Array(Box::new(BraiseType::String)),
                    "if condition"
                )
                .is_err()
        );
    }

    #[test]
    fn test_iterable_validation() {
        let validator = TypeValidator::new();

        // Valid iterables
        assert!(
            validator
                .validate_iterable_type(
                    &BraiseType::Array(Box::new(BraiseType::String)),
                    "for loop"
                )
                .is_ok()
        );
        assert!(
            validator
                .validate_iterable_type(&BraiseType::String, "for loop")
                .is_ok()
        );

        // Invalid iterable
        assert!(
            validator
                .validate_iterable_type(&BraiseType::Number, "for loop")
                .is_err()
        );
    }

    #[test]
    fn test_enum_validation() {
        let validator = TypeValidator::new();

        // Valid enum
        let variants = vec!["dev".to_string(), "prod".to_string(), "staging".to_string()];
        assert!(
            validator
                .validate_enum_variants(&variants, "environment")
                .is_ok()
        );

        // Empty enum
        assert!(
            validator
                .validate_enum_variants(&[], "environment")
                .is_err()
        );

        // Duplicate variants
        let duplicates = vec!["dev".to_string(), "prod".to_string(), "dev".to_string()];
        assert!(
            validator
                .validate_enum_variants(&duplicates, "environment")
                .is_err()
        );
    }
}

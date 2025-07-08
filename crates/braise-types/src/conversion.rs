use crate::{BraiseType, TypedValue, ValueData, TypeError};
use std::collections::HashMap;

impl TypedValue {
    /// Try to convert this value to the target type
    pub fn convert_to(&self, target_type: &BraiseType) -> Result<TypedValue, TypeError> {
        if self.value_type.is_compatible_with(target_type) {
            return Ok(self.clone());
        }
        
        match (&self.value, target_type) {
            (_, BraiseType::String) => Ok(TypedValue::new(self.to_string(), BraiseType::String)),
            (_, BraiseType::Optional(inner)) => self.convert_to(inner),
            (_, BraiseType::Any) => Ok(self.clone()),
            (_, BraiseType::Union(types)) => {
                for union_type in types {
                    if let Ok(value) = self.convert_to(union_type) {
                        return Ok(value);
                    }
                }
                Err(TypeError::cannot_convert(&self.value_type, target_type))
            }
            (ValueData::String(s), BraiseType::Number) => {
                let num = s.parse::<f64>().map_err(|_| TypeError::mismatch(
                    "number",
                    &format!("string '{}'", s),
                    "string to number conversion"
                ))?;
                Ok(TypedValue::new(num, BraiseType::Number))
            }
            (ValueData::Bool(b), BraiseType::Number) => Ok(TypedValue::new(
                if *b { 1.0 } else { 0.0 },
                BraiseType::Number,
            )),

            (ValueData::String(s), BraiseType::Bool) => {
                let bool_val = match s.to_lowercase().as_str() {
                    "true" | "1" | "yes" | "on" => true,
                    "false" | "0" | "no" | "off" => false,
                    _ => !s.is_empty(),
                };
                Ok(TypedValue::new(bool_val, BraiseType::Bool))
            }
            (ValueData::Number(n), BraiseType::Bool) => {
                Ok(TypedValue::new(*n != 0.0, BraiseType::Bool))
            }

            (ValueData::Array(arr), BraiseType::Array(target_inner)) => {
                let converted: Result<Vec<TypedValue>, TypeError> = arr
                    .iter()
                    .map(|item| item.convert_to(target_inner))
                    .collect();
                Ok(TypedValue::new(
                    converted?,
                    BraiseType::Array(target_inner.clone()),
                ))
            }

            (ValueData::String(s), BraiseType::Enum(variants)) => {
                if variants.contains(s) {
                    Ok(TypedValue::new(s.clone(), target_type.clone()))
                } else {
                    Err(TypeError::mismatch(
                        &format!("one of {{{}}}", variants.join(", ")),
                        s,
                        "enum conversion"
                    ))
                }
            }

            _ => Err(TypeError::cannot_convert(&self.value_type, target_type)),
        }
    }

    /// Type-safe value coercion
    pub fn coerce_to_type(&self, target_type: &BraiseType) -> Result<TypedValue, TypeError> {
        if self.value_type.is_compatible_with(target_type) {
            return Ok(self.clone());
        }

        self.convert_to(target_type)
    }

    /// Convert to number (for arithmetic operations)
    pub fn to_number(&self) -> Result<f64, TypeError> {
        match &self.value {
            ValueData::Number(n) => Ok(*n),
            ValueData::String(s) => s.parse().map_err(|_| TypeError::cannot_convert(
                &self.value_type,
                &BraiseType::Number
            )),
            ValueData::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            _ => Err(TypeError::cannot_convert(
                &self.value_type,
                &BraiseType::Number
            )),
        }
    }

    /// Validate that this value matches the expected type
    pub fn validate_type(
        &self,
        expected_type: &BraiseType,
        context: &str,
    ) -> Result<(), TypeError> {
        if !self.value_type.is_compatible_with(expected_type) {
            return Err(TypeError::mismatch(
                &expected_type.to_string(),
                &format!("{:?}", self.value),
                context
            ));
        }
        Ok(())
    }
}

/// Utility functions for type conversion and parameter handling
pub struct TypeConverter;

impl TypeConverter {
    /// Convert a string parameter to the expected type
    pub fn convert_parameter(
        user_value: &str,
        param_name: &str,
        expected_type: &BraiseType,
    ) -> Result<TypedValue, String> {
        let string_value = TypedValue::new(user_value.to_string(), BraiseType::String);

        match expected_type {
            BraiseType::String => Ok(string_value),

            BraiseType::Number => {
                let num = user_value.parse::<f64>().map_err(|_| {
                    format!(
                        "Invalid parameter '{}': expected number, got '{}'",
                        param_name, user_value
                    )
                })?;
                Ok(TypedValue::new(num, BraiseType::Number))
            }

            BraiseType::Bool => {
                let bool_val = match user_value.to_lowercase().as_str() {
                    "true" | "1" | "yes" | "on" => true,
                    "false" | "0" | "no" | "off" => false,
                    _ => {
                        return Err(format!(
                            "Invalid parameter '{}': expected boolean (true/false, 1/0, yes/no, on/off), got '{}'",
                            param_name, user_value
                        ));
                    }
                };
                Ok(TypedValue::new(bool_val, BraiseType::Bool))
            }

            BraiseType::Enum(variants) => {
                if variants.contains(&user_value.to_string()) {
                    Ok(TypedValue::new(
                        user_value.to_string(),
                        expected_type.clone(),
                    ))
                } else {
                    Err(format!(
                        "Invalid parameter '{}': expected one of [{}], got '{}'",
                        param_name,
                        variants.join(", "),
                        user_value
                    ))
                }
            }

            BraiseType::Array(element_type) => {
                let items: Result<Vec<TypedValue>, String> = user_value
                    .split(',')
                    .map(|s| {
                        let trimmed = s.trim();
                        Self::convert_parameter(trimmed, param_name, element_type)
                    })
                    .collect();

                match items {
                    Ok(values) => Ok(TypedValue::new(values, expected_type.clone())),
                    Err(e) => Err(format!(
                        "Invalid array parameter '{}': {}",
                        param_name, e
                    )),
                }
            }

            BraiseType::Optional(inner_type) => {
                if user_value.is_empty() {
                    Ok(TypedValue::new(None::<String>, expected_type.clone()))
                } else {
                    Self::convert_parameter(user_value, param_name, inner_type)
                }
            }

            BraiseType::Union(types) => {
                for union_type in types {
                    if let Ok(value) = Self::convert_parameter(user_value, param_name, union_type) {
                        return Ok(value);
                    }
                }
                Err(format!(
                    "Invalid parameter '{}': expected one of [{}], got '{}'",
                    param_name,
                    expected_type,
                    user_value
                ))
            }

            BraiseType::Recipe => Ok(TypedValue::new(
                (user_value.to_string(), HashMap::new()),
                BraiseType::Recipe,
            )),

            BraiseType::Any => {
                // Try to infer the best type
                if let Ok(num) = user_value.parse::<f64>() {
                    Ok(TypedValue::new(num, BraiseType::Number))
                } else if let Ok(bool_val) = user_value.parse::<bool>() {
                    Ok(TypedValue::new(bool_val, BraiseType::Bool))
                } else {
                    Ok(TypedValue::new(user_value.to_string(), BraiseType::String))
                }
            }
        }
    }

    /// Smart type coercion for binary operations
    pub fn coerce_for_operation(
        left: &TypedValue,
        right: &TypedValue,
        operation: &str,
    ) -> Result<(TypedValue, TypedValue), TypeError> {
        // If types are already compatible, no coercion needed
        if left.value_type.is_compatible_with(&right.value_type) {
            return Ok((left.clone(), right.clone()));
        }

        // For arithmetic operations, try to convert to numbers
        if matches!(operation, "+" | "-" | "*" | "/" | "%" | "<" | ">" | "<=" | ">=") {
            if let (Ok(left_num), Ok(right_num)) = (left.to_number(), right.to_number()) {
                return Ok((
                    TypedValue::new(left_num, BraiseType::Number),
                    TypedValue::new(right_num, BraiseType::Number),
                ));
            }
        }

        // For equality operations, convert to common type
        if matches!(operation, "==" | "!=") {
            // Try string conversion as fallback
            return Ok((
                TypedValue::new(left.to_string(), BraiseType::String),
                TypedValue::new(right.to_string(), BraiseType::String),
            ));
        }

        // For logical operations, convert to bool
        if matches!(operation, "&&" | "||") {
            return Ok((
                TypedValue::new(left.to_bool(), BraiseType::Bool),
                TypedValue::new(right.to_bool(), BraiseType::Bool),
            ));
        }

        Err(TypeError::unsupported_operation(
            operation,
            &left.value_type,
            &right.value_type,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_conversion() {
        let string_val = TypedValue::new("42".to_string(), BraiseType::String);
        let converted = string_val.convert_to(&BraiseType::Number).unwrap();
        if let ValueData::Number(n) = converted.value {
            assert_eq!(n, 42.0);
        } else {
            panic!("Expected number");
        }

        let bool_val = TypedValue::new(true, BraiseType::Bool);
        let converted = bool_val.convert_to(&BraiseType::String).unwrap();
        assert_eq!(converted.to_string(), "true");
    }

    #[test]
    fn test_parameter_conversion() {
        // Test number conversion
        let result = TypeConverter::convert_parameter("42", "test", &BraiseType::Number).unwrap();
        assert_eq!(result.value_type, BraiseType::Number);
        if let ValueData::Number(n) = result.value {
            assert_eq!(n, 42.0);
        }

        // Test boolean conversion
        let result = TypeConverter::convert_parameter("true", "test", &BraiseType::Bool).unwrap();
        assert_eq!(result.value_type, BraiseType::Bool);
        if let ValueData::Bool(b) = result.value {
            assert!(b);
        }

        // Test enum conversion
        let enum_type = BraiseType::Enum(vec!["dev".to_string(), "prod".to_string()]);
        let result = TypeConverter::convert_parameter("dev", "test", &enum_type).unwrap();
        assert_eq!(result.value_type, enum_type);
    }

    #[test]
    fn test_type_coercion() {
        let left = TypedValue::new("42", BraiseType::String);
        let right = TypedValue::new(10.0, BraiseType::Number);
        
        let (coerced_left, coerced_right) = TypeConverter::coerce_for_operation(&left, &right, "+").unwrap();
        
        assert_eq!(coerced_left.value_type, BraiseType::Number);
        assert_eq!(coerced_right.value_type, BraiseType::Number);
    }
}
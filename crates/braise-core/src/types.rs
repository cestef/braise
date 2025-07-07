use crate::{Result, error::types::TypeError, runtime::RuntimeError};
use std::collections::HashMap;

/// Core type system for Braise
#[derive(Debug, Clone, PartialEq)]
pub enum BraiseType {
    String,
    Number,
    Bool,
    Array(Box<BraiseType>),
    Enum(Vec<String>),
    Recipe,
    Union(Vec<BraiseType>),
    Optional(Box<BraiseType>),
    Any,
}

impl BraiseType {
    /// Check if this type is compatible with another type
    pub fn is_compatible_with(&self, other: &BraiseType) -> bool {
        match (self, other) {
            (BraiseType::String, BraiseType::String) => true,
            (BraiseType::Number, BraiseType::Number) => true,
            (BraiseType::Bool, BraiseType::Bool) => true,
            (BraiseType::Recipe, BraiseType::Recipe) => true,
            (BraiseType::Any, _) | (_, BraiseType::Any) => true,

            (BraiseType::Array(a), BraiseType::Array(b)) => a.is_compatible_with(b),

            (BraiseType::Enum(a), BraiseType::Enum(b)) => a == b,

            (BraiseType::Union(types), other) => types.iter().any(|t| t.is_compatible_with(other)),
            (other, BraiseType::Union(types)) => types.iter().any(|t| other.is_compatible_with(t)),

            (BraiseType::Optional(inner), other) => inner.is_compatible_with(other),
            (other, BraiseType::Optional(inner)) => other.is_compatible_with(inner),

            (BraiseType::Number, BraiseType::String) => true,
            (BraiseType::Bool, BraiseType::String) => true,

            (BraiseType::Bool, BraiseType::Number) => true,
            (BraiseType::Number, BraiseType::Bool) => true,

            _ => false,
        }
    }

    /// Get the most specific common type between two types
    pub fn common_type(&self, other: &BraiseType) -> BraiseType {
        if self == other {
            return self.clone();
        }

        if self.is_compatible_with(other) {
            return other.clone();
        }

        if other.is_compatible_with(self) {
            return self.clone();
        }

        BraiseType::Union(vec![self.clone(), other.clone()])
    }

    pub fn is_iterable(&self) -> bool {
        match self {
            BraiseType::Array(_) => true,
            BraiseType::String => true,
            BraiseType::Optional(element) => element.is_iterable(),
            _ => false,
        }
    }

    /// Check if a value can be converted to this (self) type
    pub fn can_convert_from(&self, value_type: &BraiseType) -> bool {
        match (self, value_type) {
            (target, source) if target.is_compatible_with(source) => true,

            (BraiseType::String, _) => true,

            (BraiseType::Number, BraiseType::String) => true,
            (BraiseType::Number, BraiseType::Bool) => true,

            (BraiseType::Bool, BraiseType::String) => true,
            (BraiseType::Bool, BraiseType::Number) => true,

            (BraiseType::Optional(inner), other) => {
                inner.can_convert_from(other) || other == &BraiseType::Any
            }
            (other, BraiseType::Optional(inner)) => {
                other.can_convert_from(inner) || other == &BraiseType::Any
            }
            _ => false,
        }
    }

    /// Get a default value for this type
    pub fn default_value(&self) -> TypedValue {
        match self {
            BraiseType::String => TypedValue::new("".to_string(), BraiseType::String),
            BraiseType::Number => TypedValue::new(0.0, BraiseType::Number),
            BraiseType::Bool => TypedValue::new(false, BraiseType::Bool),
            BraiseType::Array(inner) => {
                TypedValue::new(Vec::<TypedValue>::new(), BraiseType::Array(inner.clone()))
            }
            BraiseType::Enum(variants) => {
                let first = variants.first().cloned().unwrap_or_default();
                TypedValue::new(first, self.clone())
            }
            BraiseType::Recipe => {
                TypedValue::new(("".to_string(), HashMap::new()), BraiseType::Recipe)
            }
            BraiseType::Union(types) => {
                types.first().unwrap_or(&BraiseType::String).default_value()
            }
            BraiseType::Optional(_) => TypedValue::new(None::<ValueData>, self.clone()),
            BraiseType::Any => TypedValue::new("".to_string(), BraiseType::Any),
        }
    }
}

impl std::fmt::Display for BraiseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BraiseType::String => write!(f, "string"),
            BraiseType::Number => write!(f, "number"),
            BraiseType::Bool => write!(f, "bool"),
            BraiseType::Array(inner) => write!(f, "[{}]", inner),
            BraiseType::Enum(variants) => write!(f, "{{{}}}", variants.join(" | ")),
            BraiseType::Recipe => write!(f, "recipe"),
            BraiseType::Union(types) => {
                let type_strs: Vec<String> = types.iter().map(|t| t.to_string()).collect();
                write!(f, "{}", type_strs.join(" | "))
            }
            BraiseType::Optional(inner) => write!(f, "{}?", inner),
            BraiseType::Any => write!(f, "any"),
        }
    }
}

/// A value with its associated type information
#[derive(Debug, Clone, PartialEq)]
pub struct TypedValue {
    pub value: ValueData,
    pub value_type: BraiseType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValueData {
    String(String),
    Number(f64),
    Bool(bool),
    Array(Vec<TypedValue>),
    Recipe(String, HashMap<String, TypedValue>),
    None,
}

impl TypedValue {
    pub fn new<T: Into<ValueData>>(value: T, value_type: BraiseType) -> Self {
        Self {
            value: value.into(),
            value_type,
        }
    }

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
                Err(TypeError::CannotConvert {
                    from: self.value_type.to_string(),
                    to: target_type.to_string(),
                }
                .into())
            }
            (ValueData::String(s), BraiseType::Number) => {
                let num = s.parse::<f64>().map_err(|_| TypeError::Mismatch {
                    expected: "number".to_string(),
                    got: format!("string '{}'", s),
                    context: "string to number conversion".to_string(),
                })?;
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
                    Err(TypeError::Mismatch {
                        expected: format!("one of {{{}}}", variants.join(", ")),
                        got: s.clone(),
                        context: "enum conversion".to_string(),
                    }
                    .into())
                }
            }

            (me, other) => Err(TypeError::Mismatch {
                expected: other.to_string(),
                got: format!("{:?}", me),
                context: "type conversion".to_string(),
            }
            .into()),
        }
    }

    /// Get the actual runtime type of this value
    pub fn infer_type(&self) -> BraiseType {
        match &self.value {
            ValueData::String(_) => BraiseType::String,
            ValueData::Number(_) => BraiseType::Number,
            ValueData::Bool(_) => BraiseType::Bool,
            ValueData::Array(arr) => {
                if arr.is_empty() {
                    BraiseType::Array(Box::new(BraiseType::Any))
                } else {
                    let first_type = arr[0].infer_type();
                    let common = arr
                        .iter()
                        .skip(1)
                        .fold(first_type, |acc, item| acc.common_type(&item.infer_type()));
                    BraiseType::Array(Box::new(common))
                }
            }
            ValueData::Recipe(_, _) => BraiseType::Recipe,
            ValueData::None => BraiseType::Optional(Box::new(BraiseType::Any)),
        }
    }

    /// Type-safe value coercion
    pub fn coerce_to_type(&self, target_type: &BraiseType) -> Result<TypedValue, TypeError> {
        if self.value_type.is_compatible_with(target_type) {
            return Ok(self.clone());
        }

        self.convert_to(target_type)
    }

    /// Validate that this value matches the expected type
    pub fn validate_type(
        &self,
        expected_type: &BraiseType,
        context: &str,
    ) -> Result<(), TypeError> {
        if !self.value_type.is_compatible_with(expected_type) {
            return Err(TypeError::Mismatch {
                expected: expected_type.to_string(),
                got: format!("{:?}", self.value),
                context: context.to_string(),
            }
            .into());
        }
        Ok(())
    }

    /// Convert to boolean (for conditions)
    pub fn to_bool(&self) -> bool {
        match &self.value {
            ValueData::Bool(b) => *b,
            ValueData::String(s) => !s.is_empty(),
            ValueData::Number(n) => *n != 0.0,
            ValueData::Array(arr) => !arr.is_empty(),
            ValueData::Recipe(_, args) => !args.is_empty(),
            ValueData::None => false,
        }
    }

    /// Convert to number (for arithmetic operations)
    pub fn to_number(&self) -> Result<f64, TypeError> {
        match &self.value {
            ValueData::Number(n) => Ok(*n),
            ValueData::String(s) => s.parse().map_err(|_| TypeError::CannotConvert {
                from: self.value_type.to_string(),
                to: BraiseType::Number.to_string(),
            }),
            ValueData::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
            _ => Err(TypeError::CannotConvert {
                from: self.value_type.to_string(),
                to: BraiseType::Number.to_string(),
            }),
        }
    }

    /// Type-safe parameter conversion
    pub fn convert_parameter(
        user_value: &str,
        param_name: &str,
        expected_type: &BraiseType,
    ) -> Result<TypedValue> {
        let string_value = TypedValue::new(user_value.to_string(), BraiseType::String);

        match expected_type {
            BraiseType::String => Ok(string_value),

            BraiseType::Number => {
                let num =
                    user_value
                        .parse::<f64>()
                        .map_err(|_| RuntimeError::InvalidParameter {
                            name: param_name.to_string(),
                            expected: "number".to_string(),
                            got: user_value.to_string(),
                        })?;
                Ok(TypedValue::new(num, BraiseType::Number))
            }

            BraiseType::Bool => {
                let bool_val = match user_value.to_lowercase().as_str() {
                    "true" | "1" | "yes" | "on" => true,
                    "false" | "0" | "no" | "off" => false,
                    _ => {
                        return Err(RuntimeError::InvalidParameter {
                            name: param_name.to_string(),
                            expected: "boolean (true/false, 1/0, yes/no, on/off)".to_string(),
                            got: user_value.to_string(),
                        }
                        .into());
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
                    Err(RuntimeError::InvalidParameter {
                        name: param_name.to_string(),
                        expected: format!("one of: {}", variants.join(", ")),
                        got: user_value.to_string(),
                    }
                    .into())
                }
            }

            BraiseType::Array(element_type) => {
                let items: Result<Vec<TypedValue>> = user_value
                    .split(',')
                    .map(|s| {
                        let trimmed = s.trim();
                        Self::convert_parameter(trimmed, param_name, element_type)
                    })
                    .collect();

                match items {
                    Ok(values) => Ok(TypedValue::new(values, expected_type.clone())),
                    Err(_) => Err(RuntimeError::InvalidParameter {
                        name: param_name.to_string(),
                        expected: format!("array of {}", element_type),
                        got: user_value.to_string(),
                    }
                    .into()),
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
                Err(RuntimeError::InvalidParameter {
                    name: param_name.to_string(),
                    expected: format!("one of: {}", expected_type),
                    got: user_value.to_string(),
                }
                .into())
            }

            BraiseType::Recipe => Ok(TypedValue::new(
                (user_value.to_string(), HashMap::new()),
                BraiseType::Recipe,
            )),

            BraiseType::Any => {
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
}

impl std::fmt::Display for TypedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            ValueData::String(s) => write!(f, "{}", s),
            ValueData::Number(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            ValueData::Bool(b) => write!(f, "{}", b),
            ValueData::Array(arr) => {
                write!(f, "[")?;
                for (i, item) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", item)?;
                }
                write!(f, "]")
            }
            ValueData::Recipe(name, args) => {
                write!(f, "{}(", name)?;
                for (i, (k, v)) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, ")")
            }
            ValueData::None => write!(f, "null"),
        }
    }
}

impl From<String> for ValueData {
    fn from(s: String) -> Self {
        ValueData::String(s)
    }
}

impl From<&str> for ValueData {
    fn from(s: &str) -> Self {
        ValueData::String(s.to_string())
    }
}

impl From<f64> for ValueData {
    fn from(n: f64) -> Self {
        ValueData::Number(n)
    }
}

impl From<bool> for ValueData {
    fn from(b: bool) -> Self {
        ValueData::Bool(b)
    }
}

impl From<Vec<TypedValue>> for ValueData {
    fn from(arr: Vec<TypedValue>) -> Self {
        ValueData::Array(arr)
    }
}

impl From<(String, HashMap<String, TypedValue>)> for ValueData {
    fn from((name, args): (String, HashMap<String, TypedValue>)) -> Self {
        ValueData::Recipe(name, args)
    }
}

impl<T> From<Option<T>> for ValueData
where
    T: Into<ValueData>,
{
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(val) => val.into(),
            None => ValueData::None,
        }
    }
}

#[derive(Debug, Clone)]
/// Type checker for expressions and statements
pub struct TypeChecker {
    /// Current type context (variable name -> type)
    context: HashMap<String, BraiseType>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            context: HashMap::new(),
        }
    }

    pub fn with_context(context: HashMap<String, BraiseType>) -> Self {
        Self { context }
    }

    /// Add a variable to the type context
    pub fn define_variable(&mut self, name: String, var_type: BraiseType) {
        self.context.insert(name, var_type);
    }

    /// Get the type of a variable
    pub fn get_variable_type(&self, name: &str) -> Option<&BraiseType> {
        self.context.get(name)
    }

    /// Check if a variable is defined
    pub fn is_variable_defined(&self, name: &str) -> bool {
        self.context.contains_key(name)
    }

    /// Create a new scope (returns a new TypeChecker with copied context)
    pub fn enter_scope(&self) -> TypeChecker {
        TypeChecker {
            context: self.context.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_compatibility() {
        let string_type = BraiseType::String;
        let number_type = BraiseType::Number;
        let bool_type = BraiseType::Bool;
        let any_type = BraiseType::Any;

        assert!(string_type.is_compatible_with(&string_type));
        assert!(number_type.is_compatible_with(&number_type));

        assert!(number_type.is_compatible_with(&string_type));
        assert!(bool_type.is_compatible_with(&string_type));

        assert!(any_type.is_compatible_with(&string_type));
        assert!(string_type.is_compatible_with(&any_type));

        assert!(!string_type.is_compatible_with(&number_type));
        assert!(!string_type.is_compatible_with(&bool_type));
    }

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
    fn test_type_inference() {
        let string_val = TypedValue::new("hello".to_string(), BraiseType::String);
        assert_eq!(string_val.infer_type(), BraiseType::String);

        let array_val = TypedValue::new(
            vec![
                TypedValue::new("a".to_string(), BraiseType::String),
                TypedValue::new("b".to_string(), BraiseType::String),
            ],
            BraiseType::Array(Box::new(BraiseType::String)),
        );
        assert_eq!(
            array_val.infer_type(),
            BraiseType::Array(Box::new(BraiseType::String))
        );
    }

    #[test]
    fn test_union_types() {
        let union_type = BraiseType::Union(vec![BraiseType::String, BraiseType::Number]);

        assert!(union_type.is_compatible_with(&BraiseType::String));
        assert!(union_type.is_compatible_with(&BraiseType::Number));
        assert!(union_type.is_compatible_with(&BraiseType::Bool));
        assert!(!union_type.is_compatible_with(&BraiseType::Array(Box::new(BraiseType::String))));
    }

    #[test]
    fn test_optional_types() {
        let optional_string = BraiseType::Optional(Box::new(BraiseType::String));

        assert!(optional_string.is_compatible_with(&BraiseType::String));
        assert!(BraiseType::String.is_compatible_with(&optional_string));
    }
}

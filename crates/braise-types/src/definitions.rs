use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Core type system for Braise
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    Error,
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

            (BraiseType::String, BraiseType::Error) => true,
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
            BraiseType::Error => TypedValue::new(
                ValueData::Error {
                    message: "An error occurred".to_string(),
                    code: None,
                },
                BraiseType::Error,
            ),
        }
    }
}

impl std::fmt::Display for BraiseType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BraiseType::String => write!(f, "string"),
            BraiseType::Number => write!(f, "number"),
            BraiseType::Bool => write!(f, "bool"),
            BraiseType::Array(inner) => write!(f, "[{inner}]"),
            BraiseType::Enum(variants) => write!(f, "{{{}}}", variants.join(" | ")),
            BraiseType::Recipe => write!(f, "recipe"),
            BraiseType::Union(types) => {
                let type_strs: Vec<String> = types.iter().map(|t| t.to_string()).collect();
                write!(f, "{}", type_strs.join(" | "))
            }
            BraiseType::Optional(inner) => write!(f, "{inner}?"),
            BraiseType::Any => write!(f, "any"),
            BraiseType::Error => write!(f, "error"),
        }
    }
}

/// A value with its associated type information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TypedValue {
    pub value: ValueData,
    pub value_type: BraiseType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ValueData {
    String(String),
    Number(f64),
    Bool(bool),
    Array(Vec<TypedValue>),
    Recipe(String, HashMap<String, TypedValue>),
    None,
    Error { message: String, code: Option<i32> },
}

impl TypedValue {
    pub fn new<T: Into<ValueData>>(value: T, value_type: BraiseType) -> Self {
        Self {
            value: value.into(),
            value_type,
        }
    }

    pub fn string(value: impl Into<String>) -> Self {
        Self::new(ValueData::String(value.into()), BraiseType::String)
    }

    pub fn number(value: f64) -> Self {
        Self::new(ValueData::Number(value), BraiseType::Number)
    }

    pub fn bool(value: bool) -> Self {
        Self::new(ValueData::Bool(value), BraiseType::Bool)
    }

    pub fn array(values: Vec<TypedValue>) -> Self {
        let value_type = if values.is_empty() {
            BraiseType::Array(Box::new(BraiseType::Any))
        } else {
            let first_type = values[0].infer_type();
            let common = values
                .iter()
                .skip(1)
                .fold(first_type, |acc, item| acc.common_type(&item.infer_type()));
            BraiseType::Array(Box::new(common))
        };
        Self::new(ValueData::Array(values), value_type)
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
            ValueData::Error { .. } => BraiseType::Error,
        }
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
            ValueData::Error { .. } => false,
        }
    }
}

impl std::fmt::Display for TypedValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            ValueData::String(s) => write!(f, "{s}"),
            ValueData::Number(n) => {
                if n.fract() == 0.0 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{n}")
                }
            }
            ValueData::Bool(b) => write!(f, "{b}"),
            ValueData::Array(arr) => {
                write!(f, "[")?;
                for (i, item) in arr.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{item}")?;
                }
                write!(f, "]")
            }
            ValueData::Recipe(name, args) => {
                write!(f, "{name}(")?;
                for (i, (k, v)) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{k}: {v}")?;
                }
                write!(f, ")")
            }
            ValueData::None => write!(f, "null"),
            ValueData::Error { message, code } => {
                if let Some(code) = code {
                    write!(f, "Error({code}): {message}")
                } else {
                    write!(f, "Error: {message}")
                }
            }
        }
    }
}

// From implementations for ValueData
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

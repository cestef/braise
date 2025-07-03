use core::{ParamType, Parameter};
use std::collections::HashMap;

use super::{Result, RuntimeError};

#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Array(Vec<Value>),
    Recipe(String, HashMap<String, Value>),
}

impl ToString for Value {
    fn to_string(&self) -> String {
        match self {
            Value::String(s) => s.clone(),
            Value::Number(n) => {
                if n.fract() == 0.0 {
                    format!("{}", *n as i64)
                } else {
                    format!("{}", n)
                }
            }
            Value::Bool(b) => b.to_string(),
            Value::Array(arr) => {
                let strings: Vec<String> = arr.iter().map(|v| v.to_string()).collect();
                format!("[{}]", strings.join(", "))
            }
            Value::Recipe(name, args) => {
                let args_str: Vec<String> = args
                    .iter()
                    .map(|(k, v)| format!("{k}: {}", v.to_string()))
                    .collect();
                format!("{}({})", name, args_str.join(", "))
            }
        }
    }
}

impl Value {
    pub fn to_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::String(s) => !s.is_empty(),
            Value::Number(n) => *n != 0.0,
            Value::Array(arr) => !arr.is_empty(),
            Value::Recipe(_, args) => !args.is_empty(),
        }
    }

    pub fn to_number(&self) -> Result<f64> {
        match self {
            Value::Number(n) => Ok(*n),
            Value::String(s) => s
                .parse()
                .map_err(|_| RuntimeError::TypeError(format!("Cannot convert '{}' to number", s))),
            Value::Bool(true) => Ok(1.0),
            Value::Bool(false) => Ok(0.0),
            Value::Array(_) => Err(RuntimeError::TypeError(
                "Cannot convert array to number".to_string(),
            )),
            Value::Recipe(_, _) => Err(RuntimeError::TypeError(
                "Cannot convert recipe to number".to_string(),
            )),
        }
    }
    pub fn is<T: 'static>(&self) -> bool {
        match self {
            Value::String(_) => std::any::TypeId::of::<T>() == std::any::TypeId::of::<String>(),
            Value::Number(_) => std::any::TypeId::of::<T>() == std::any::TypeId::of::<f64>(),
            Value::Bool(_) => std::any::TypeId::of::<T>() == std::any::TypeId::of::<bool>(),
            Value::Array(_) => std::any::TypeId::of::<T>() == std::any::TypeId::of::<Vec<Value>>(),
            Value::Recipe(_, _) => {
                std::any::TypeId::of::<T>() == std::any::TypeId::of::<(String, Vec<Value>)>()
            }
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::String(_) => "string",
            Value::Number(_) => "number",
            Value::Bool(_) => "boolean",
            Value::Array(_) => "array",
            Value::Recipe(_, _) => "recipe",
        }
    }

    pub fn default_for_type(param_type: &ParamType) -> Self {
        match param_type {
            ParamType::String => Value::String(String::new()),
            ParamType::Number => Value::Number(0.0),
            ParamType::Bool => Value::Bool(false),
            ParamType::Array(_) => Value::Array(Vec::new()),
            ParamType::Enum(_) => Value::String(String::new()), // Default for enum is empty string
            ParamType::Recipe => Value::Recipe(String::new(), HashMap::new()),
        }
    }

    pub fn from_str(user_value: &str, name: &String, expected: &ParamType) -> Result<Self> {
        match expected {
            ParamType::String => Ok(Value::String(user_value.to_string())),

            ParamType::Number => user_value.parse::<f64>().map(Value::Number).map_err(|_| {
                RuntimeError::InvalidParameter {
                    name: name.clone(),
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
                            name: name.clone(),
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
                        name: name.clone(),
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
                            name: format!("{}_element", name),
                            param_type: (**element_type).clone(),
                            default: None,
                        };
                        Value::from_str(trimmed, &temp_param.name, &temp_param.param_type).map_err(
                            |_| RuntimeError::InvalidParameter {
                                name: temp_param.name.clone(),
                                expected: format!("array of {}", element_type),
                                got: trimmed.to_string(),
                            },
                        )
                    })
                    .collect();

                match items {
                    Ok(values) => Ok(Value::Array(values)),
                    Err(e) => Err(RuntimeError::InvalidParameter {
                        name: name.clone(),
                        expected: format!("array of {}", element_type),
                        got: format!("{} (error in array element: {})", user_value, e),
                    }),
                }
            }
            _ => Err(RuntimeError::TypeError(format!(
                "Unsupported parameter type: {:?}",
                expected
            ))),
        }
    }

    pub fn convert_to_type(&self, target_type: &ParamType) -> Result<Value> {
        match target_type {
            ParamType::String => Ok(Value::String(self.to_string())),
            ParamType::Number => Ok(Value::Number(self.to_number()?)),
            ParamType::Bool => Ok(Value::Bool(self.to_bool())),
            ParamType::Array(inside_type) => {
                if let Value::Array(arr) = self {
                    let converted: Result<Vec<Value>> =
                        arr.iter().map(|v| v.convert_to_type(inside_type)).collect();
                    converted.map(Value::Array)
                } else {
                    Err(RuntimeError::TypeError(
                        "Cannot convert non-array value to array".to_string(),
                    ))
                }
            }
            ParamType::Enum(_) => Err(RuntimeError::TypeError(
                "Cannot convert value to enum".to_string(),
            )),
            ParamType::Recipe => match self {
                Value::Recipe(name, args) => Ok(Value::Recipe(name.clone(), args.clone())),
                Value::String(name) => Ok(Value::Recipe(name.clone(), HashMap::new())),
                _ => Err(RuntimeError::TypeError(
                    "Cannot convert value to recipe".to_string(),
                )),
            },
        }
    }
}

impl Into<String> for Value {
    fn into(self) -> String {
        self.to_string()
    }
}
impl Into<f64> for Value {
    fn into(self) -> f64 {
        self.to_number().unwrap_or(0.0)
    }
}
impl Into<bool> for Value {
    fn into(self) -> bool {
        self.to_bool()
    }
}

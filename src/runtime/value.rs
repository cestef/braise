use super::error::*;

#[derive(Debug, Clone)]
pub enum Value {
    String(String),
    Number(f64),
    Bool(bool),
    Array(Vec<Value>),
}

impl Value {
    pub fn to_string(&self) -> String {
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
        }
    }

    pub fn to_bool(&self) -> bool {
        match self {
            Value::Bool(b) => *b,
            Value::String(s) => !s.is_empty(),
            Value::Number(n) => *n != 0.0,
            Value::Array(arr) => !arr.is_empty(),
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
        }
    }
    pub fn is<T: 'static>(&self) -> bool {
        match self {
            Value::String(_) => std::any::TypeId::of::<T>() == std::any::TypeId::of::<String>(),
            Value::Number(_) => std::any::TypeId::of::<T>() == std::any::TypeId::of::<f64>(),
            Value::Bool(_) => std::any::TypeId::of::<T>() == std::any::TypeId::of::<bool>(),
            Value::Array(_) => std::any::TypeId::of::<T>() == std::any::TypeId::of::<Vec<Value>>(),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::String(_) => "string",
            Value::Number(_) => "number",
            Value::Bool(_) => "boolean",
            Value::Array(_) => "array",
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

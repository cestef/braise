use crate::{Result, RuntimeError, Value};
use std::env;

pub struct EnvModule;

impl EnvModule {
    pub fn new() -> Self {
        Self
    }

    fn get(&self, var_name: String) -> Result<Value> {
        let value = env::var(&var_name).unwrap_or_default();
        Ok(Value::String(value))
    }

    fn has(&self, var_name: String) -> Result<Value> {
        Ok(Value::Bool(env::var(&var_name).is_ok()))
    }

    fn ci(&self) -> Result<Value> {
        Ok(Value::Bool(env::var("CI").is_ok()))
    }

    fn home(&self) -> Result<Value> {
        Ok(Value::String(env::var("HOME").unwrap_or_default()))
    }

    fn pwd(&self) -> Result<Value> {
        Ok(Value::String(
            env::current_dir()
                .map_err(|e| {
                    RuntimeError::BuiltinError(format!("Failed to get current directory: {e}"))
                })?
                .to_string_lossy()
                .to_string(),
        ))
    }
}

builtin_module! {
    EnvModule {
        functions: {
            "get" => get(String),
            "has" => has(String),
        },
        fields: {
            "HOME" => home,
            "PWD" => pwd,
            "CI" => ci,
        }
    }
}

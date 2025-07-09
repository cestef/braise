use crate::{BraiseType, Result, RuntimeError};
use core::TypedValue;
use std::env;

pub struct EnvModule;

impl EnvModule {
    pub fn new() -> Self {
        Self
    }

    fn get(&self, var_name: TypedValue) -> Result<TypedValue> {
        let value = env::var(var_name.to_string()).unwrap_or_default();
        Ok(TypedValue::new(value, BraiseType::String))
    }

    fn has(&self, var_name: TypedValue) -> Result<TypedValue> {
        Ok(TypedValue::new(
            env::var(var_name.to_string()).is_ok(),
            BraiseType::Bool,
        ))
    }

    fn ci(&self) -> Result<TypedValue> {
        Ok(TypedValue::new(env::var("CI").is_ok(), BraiseType::Bool))
    }

    fn home(&self) -> Result<TypedValue> {
        Ok(TypedValue::new(
            env::var("HOME").unwrap_or_default(),
            BraiseType::String,
        ))
    }

    fn pwd(&self) -> Result<TypedValue> {
        Ok(TypedValue::new(
            env::current_dir()
                .map_err(|e| {
                    RuntimeError::builtin_error(
                        format!("Failed to get current directory: {e}"),
                        "env",
                    )
                })?
                .to_string_lossy()
                .to_string(),
            BraiseType::String,
        ))
    }
}

builtin_module! {
    EnvModule {
        functions: {
            "get" => get(BraiseType::String),
            "has" => has(BraiseType::String),
        },
        fields: {
            "HOME" => home,
            "PWD" => pwd,
            "CI" => ci,
        }
    }
}

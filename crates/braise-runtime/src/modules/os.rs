use crate::{BraiseType, Result};
use core::TypedValue;
use std::env;

pub struct OsModule;

impl OsModule {
    pub fn new() -> Self {
        Self
    }

    fn platform(&self) -> Result<TypedValue> {
        let platform = env::consts::OS.to_string();
        Ok(TypedValue::new(platform, BraiseType::String))
    }
}

builtin_module! {
    OsModule {
        functions: {
            "platform" => platform,
        },
        fields: {
            "OS" => platform,
        }
    }
}

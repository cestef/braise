use crate::{Result, TypedValue};
use std::env;

pub struct CpuModule;

impl CpuModule {
    pub fn new() -> Self {
        Self
    }

    fn count(&self) -> Result<TypedValue> {
        Ok(TypedValue::new(
            num_cpus::get() as f64,
            core::BraiseType::Number,
        ))
    }

    fn physical_count(&self) -> Result<TypedValue> {
        Ok(TypedValue::new(
            num_cpus::get_physical() as f64,
            core::BraiseType::Number,
        ))
    }

    fn arch(&self) -> Result<TypedValue> {
        Ok(TypedValue::new(
            env::consts::ARCH.to_string(),
            core::BraiseType::String,
        ))
    }
}

builtin_module! {
    CpuModule {
        functions: {
            "count" => count,
            "physical_count" => physical_count,
        },
        fields: {
            "arch" => arch,
        }
    }
}

use crate::{Result, Value};
use std::env;

pub struct CpuModule;

impl CpuModule {
    pub fn new() -> Self {
        Self
    }

    fn count(&self) -> Result<Value> {
        Ok(Value::Number(num_cpus::get() as f64))
    }

    fn physical_count(&self) -> Result<Value> {
        Ok(Value::Number(num_cpus::get_physical() as f64))
    }

    fn arch(&self) -> Result<Value> {
        Ok(Value::String(env::consts::ARCH.to_string()))
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

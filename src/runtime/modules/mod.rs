use std::collections::HashMap;

use crate::runtime::value::Value;

use super::error::*;

pub trait BuiltinModule: Send + Sync {
    fn call_function(&self, function: &str, args: Vec<Value>) -> Result<Value>;
    fn get_field(&self, field: &str) -> Result<Value> {
        Err(RuntimeError::BuiltinError(format!(
            "Field '{}' not supported by this module",
            field
        )))
    }
}

macro_rules! builtin_module {
    (
        $module_name:ident {
            functions: {
                $(
                    $func_name:literal => $func_method:ident $( ( $($arg_type:ty),+ ) )?
                ),* $(,)?
            }
            $(,fields: {
                $(
                    $field_name:literal => $field_method:ident
                ),* $(,)?
            })?
        }
    ) => {
        impl crate::runtime::modules::BuiltinModule for $module_name {
            fn call_function(&self, function: &str, args: Vec<crate::runtime::Value>) -> crate::runtime::Result<crate::runtime::Value> {
                match function {
                    $(
                        $func_name => {
                            builtin_module!(@call_function self, $func_method, args $(, $($arg_type),+)?)
                        }
                    )*
                    _ => Err(crate::runtime::RuntimeError::BuiltinError(format!(
                        "Unknown function '{}' in module '{}'",
                        function,
                        stringify!($module_name)
                    )))
                }
            }

            $(
                fn get_field(&self, field: &str) -> crate::runtime::Result<crate::runtime::Value> {
                    match field {
                        $(
                            $field_name => self.$field_method(),
                        )*
                        _ => Err(crate::runtime::RuntimeError::BuiltinError(format!(
                            "Unknown field '{}' in module '{}'",
                            field,
                            stringify!($module_name)
                        )))
                    }
                }
            )?
        }
    };

    (@call_function $self:expr, $func_method:ident, $args:ident) => {
        {
            if !$args.is_empty() {
                return Err(crate::runtime::RuntimeError::BuiltinError(format!(
                    "Function '{}' requires no arguments, got {}",
                    stringify!($func_method),
                    $args.len()
                )));
            }
            $self.$func_method()
        }
    };

    (@call_function $self:expr, $func_method:ident, $args:ident, $($arg_type:ty),+) => {
        {
            if $args.len() != 1 {
                return Err(crate::runtime::RuntimeError::BuiltinError(format!(
                    "Function '{}' requires exactly 1 argument, got {}",
                    stringify!($func_method),
                    $args.len()
                )));
            }
            let arg = $args.into_iter().next().unwrap();
            if !arg.is::<$($arg_type),+>() {
                return Err(crate::runtime::RuntimeError::BuiltinError(format!(
                    "Function '{}' expected argument of type {}, got {}",
                    stringify!($func_method),
                    stringify!($($arg_type),+),
                    arg.type_name()
                )));
            }
            $self.$func_method(arg.into())
        }
    };
}

macro_rules! register_modules {
    (
        $(
            $module_name:literal => $module_type:ty
        ),* $(,)?
    ) => {
        static MODULES: once_cell::sync::Lazy<HashMap<String, Box<dyn BuiltinModule>>> =
            once_cell::sync::Lazy::new(|| {
                let mut modules: HashMap<String, Box<dyn BuiltinModule>> = HashMap::new();
                $(
                    modules.insert($module_name.to_string(), Box::new(<$module_type>::new()));
                )*
                modules
            });
    };
}

pub struct BuiltinModules;

impl BuiltinModules {
    pub fn new() -> Self {
        Self
    }
}

impl BuiltinModules {
    pub fn call_function(&self, module: &str, function: &str, args: Vec<Value>) -> Result<Value> {
        match MODULES.get(module) {
            Some(m) => m.call_function(function, args),
            None => Err(RuntimeError::BuiltinError(format!(
                "Unknown module: {}",
                module
            ))),
        }
    }

    pub fn get_field(&self, module: &str, field: &str) -> Result<Value> {
        match MODULES.get(module) {
            Some(m) => m.get_field(field),
            None => Err(RuntimeError::BuiltinError(format!(
                "Unknown module: {}",
                module
            ))),
        }
    }
}

mod git;
use git::GitModule;

mod env;
use env::EnvModule;

mod cpu;
use cpu::CpuModule;

mod fs;
use fs::FsModule;

register_modules! {
    "git" => GitModule,
    "env" => EnvModule,
    "cpu" => CpuModule,
    "fs" => FsModule,
}

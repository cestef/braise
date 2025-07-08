use core::TypedValue;
use std::collections::HashMap;

use super::{Result, RuntimeError};

pub trait BuiltinModule: Send + Sync {
    fn call_function(&self, function: &str, args: Vec<TypedValue>) -> Result<TypedValue>;
    fn get_field(&self, field: &str) -> Result<TypedValue> {
        Err(RuntimeError::builtin_error(
            format!("Field '{field}' not supported by this module"),
            "module"
        ))
    }
}

macro_rules! builtin_module {
    (
        $module_name:ident {
            functions: {
                $(
                    $func_name:literal => $func_method:ident $( ( $arg_type:expr ) )?
                ),* $(,)?
            }
            $(,fields: {
                $(
                    $field_name:literal => $field_method:ident
                ),* $(,)?
            })?
        }
    ) => {
        impl crate::modules::BuiltinModule for $module_name {
            fn call_function(&self, function: &str, args: Vec<crate::TypedValue>) -> crate::Result<crate::TypedValue> {
                match function {
                    $(
                        $func_name => {
                            builtin_module!(@call_function self, $func_method, args $(, $arg_type)?)
                        }
                    )*
                    _ => Err(crate::RuntimeError::builtin_error(
                        format!("Unknown function '{}' in module '{}'", function, stringify!($module_name)),
                        stringify!($module_name)
                    ))
                }
            }

            $(
                fn get_field(&self, field: &str) -> crate::Result<crate::TypedValue> {
                    match field {
                        $(
                            $field_name => self.$field_method(),
                        )*
                        _ => Err(crate::RuntimeError::builtin_error(
                            format!("Unknown field '{}' in module '{}'", field, stringify!($module_name)),
                            stringify!($module_name)
                        ))
                    }
                }
            )?
        }
    };

    (@call_function $self:expr, $func_method:ident, $args:ident) => {
        {
            if !$args.is_empty() {
                return Err(crate::RuntimeError::builtin_error(
                    format!("Function '{}' requires no arguments, got {}", stringify!($func_method), $args.len()),
                    "module"
                ));
            }
            $self.$func_method()
        }
    };

    (@call_function $self:expr, $func_method:ident, $args:ident, $arg_type:expr) => {
        {
            if $args.len() != 1 {
                return Err(crate::RuntimeError::builtin_error(
                    format!("Function '{}' requires exactly 1 argument, got {}", stringify!($func_method), $args.len()),
                    "module"
                ));
            }
            let arg = $args.into_iter().next().unwrap();
            if !arg.value_type.can_convert_from(&$arg_type) {
                return Err(crate::RuntimeError::builtin_error(
                    format!("Function '{}' expected argument of type {}, got {}", stringify!($func_method), stringify!($arg_type), arg.value_type),
                    "module"
                ));
            }
            $self.$func_method(arg)
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
    pub fn call_function(
        &self,
        module: &str,
        function: &str,
        args: Vec<TypedValue>,
    ) -> Result<TypedValue> {
        match MODULES.get(module) {
            Some(m) => m.call_function(function, args),
            None => Err(RuntimeError::builtin_error(
                format!("Unknown module: {module}"),
                module
            )),
        }
    }

    pub fn get_field(&self, module: &str, field: &str) -> Result<TypedValue> {
        match MODULES.get(module) {
            Some(m) => m.get_field(field),
            None => Err(RuntimeError::builtin_error(
                format!("Unknown module: {module}"),
                module
            )),
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

mod os;
use os::OsModule;

register_modules! {
    "git" => GitModule,
    "env" => EnvModule,
    "cpu" => CpuModule,
    "fs" => FsModule,
    "os" => OsModule,
}

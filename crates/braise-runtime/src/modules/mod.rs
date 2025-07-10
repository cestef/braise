use core::TypedValue;
use std::collections::HashMap;

use super::{Result, RuntimeError};

pub trait BuiltinModule: Send + Sync {
    fn call_function(&self, function: &str, args: Vec<TypedValue>) -> Result<TypedValue>;
    fn get_field(&self, field: &str) -> Result<TypedValue> {
        Err(RuntimeError::builtin_error(
            format!("Field '{field}' not supported by this module"),
            "module",
        )
        .boxed())
    }
    fn has_function(&self, _function: &str) -> bool {
        false
    }
    fn has_field(&self, _field: &str) -> bool {
        false
    }
}

macro_rules! builtin_module {
    (
        $module_name:ident {
            functions: {
                $(
                    $func_name:literal => $func_method:ident $( ( $( $arg_type:expr ),* ) )?
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
                            builtin_module!(@call_function self, $func_method, args $(, [ $( $arg_type ),* ])?)
                        }
                    )*
                    _ => Err(crate::RuntimeError::builtin_error(
                        format!("Unknown function '{}' in module '{}'", function, stringify!($module_name)),
                        stringify!($module_name)
                    ).boxed())
                }
            }

            fn has_function(&self, function: &str) -> bool {
                match function {
                    $(
                        $func_name => true,
                    )*
                    _ => false,
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
                        ).boxed())
                    }
                }

                fn has_field(&self, field: &str) -> bool {
                    match field {
                        $(
                            $field_name => true,
                        )*
                        _ => false,
                    }
                }
            )?
        }
    };

    // No arguments
    (@call_function $self:expr, $func_method:ident, $args:ident) => {
        {
            if !$args.is_empty() {
                return Err(crate::RuntimeError::builtin_error(
                    format!("Function '{}' requires no arguments, got {}", stringify!($func_method), $args.len()),
                    "module"
                ).boxed());
            }
            $self.$func_method()
        }
    };

    // Generalized argument handling - works for any number of arguments
    (@call_function $self:expr, $func_method:ident, $args:ident, [ $( $arg_type:expr ),* ]) => {
        {
            // Count expected arguments at compile time
            const EXPECTED_ARGS: usize = builtin_module!(@count $( $arg_type ),*);

            if $args.len() != EXPECTED_ARGS {
                return Err(crate::RuntimeError::builtin_error(
                    format!("Function '{}' requires exactly {} arguments, got {}",
                        stringify!($func_method), EXPECTED_ARGS, $args.len()),
                    "module"
                ).boxed());
            }

            // Validate argument types
            builtin_module!(@validate_args $args, stringify!($func_method), 0, $( $arg_type ),*)?;

            // Call the function - we need to unpack arguments dynamically
            builtin_module!(@call_with_args $self, $func_method, $args, $( $arg_type ),*)
        }
    };

    // Helper macro to count arguments at compile time
    (@count) => { 0 };
    (@count $head:expr $(, $tail:expr)*) => { 1 + builtin_module!(@count $( $tail ),*) };

    // Helper macro to validate argument types
    (@validate_args $args:ident, $func_name:expr, $index:expr,) => { Ok::<(), crate::RuntimeError>(()) };
    (@validate_args $args:ident, $func_name:expr, $index:expr, $arg_type:expr $(, $rest:expr)*) => {
        {
            if let Some(arg) = $args.get($index) {
                if !arg.value_type.can_convert_from(&$arg_type) {
                    return Err(crate::RuntimeError::builtin_error(
                        format!("Function '{}' expected argument {} of type {}, got {}",
                            $func_name, $index + 1, $arg_type, arg.value_type),
                        "module"
                    ).boxed());
                }
            }
            builtin_module!(@validate_args $args, $func_name, $index + 1, $( $rest ),*)
        }
    };

    // Helper macro to call function with unpacked arguments
    (@call_with_args $self:expr, $func_method:ident, $args:ident,) => {
        $self.$func_method()
    };
    (@call_with_args $self:expr, $func_method:ident, $args:ident, $arg1_type:expr) => {
        $self.$func_method($args.into_iter().next().unwrap())
    };
    (@call_with_args $self:expr, $func_method:ident, $args:ident, $arg1_type:expr, $arg2_type:expr) => {
        {
            let mut iter = $args.into_iter();
            $self.$func_method(iter.next().unwrap(), iter.next().unwrap())
        }
    };
    (@call_with_args $self:expr, $func_method:ident, $args:ident, $arg1_type:expr, $arg2_type:expr, $arg3_type:expr) => {
        {
            let mut iter = $args.into_iter();
            $self.$func_method(iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap())
        }
    };
    (@call_with_args $self:expr, $func_method:ident, $args:ident, $arg1_type:expr, $arg2_type:expr, $arg3_type:expr, $arg4_type:expr) => {
        {
            let mut iter = $args.into_iter();
            $self.$func_method(iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap())
        }
    };
    (@call_with_args $self:expr, $func_method:ident, $args:ident, $arg1_type:expr, $arg2_type:expr, $arg3_type:expr, $arg4_type:expr, $arg5_type:expr) => {
        {
            let mut iter = $args.into_iter();
            $self.$func_method(iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap())
        }
    };
    (@call_with_args $self:expr, $func_method:ident, $args:ident, $arg1_type:expr, $arg2_type:expr, $arg3_type:expr, $arg4_type:expr, $arg5_type:expr, $arg6_type:expr) => {
        {
            let mut iter = $args.into_iter();
            $self.$func_method(iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap())
        }
    };
    (@call_with_args $self:expr, $func_method:ident, $args:ident, $arg1_type:expr, $arg2_type:expr, $arg3_type:expr, $arg4_type:expr, $arg5_type:expr, $arg6_type:expr, $arg7_type:expr) => {
        {
            let mut iter = $args.into_iter();
            $self.$func_method(iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap())
        }
    };
    (@call_with_args $self:expr, $func_method:ident, $args:ident, $arg1_type:expr, $arg2_type:expr, $arg3_type:expr, $arg4_type:expr, $arg5_type:expr, $arg6_type:expr, $arg7_type:expr, $arg8_type:expr) => {
        {
            let mut iter = $args.into_iter();
            $self.$func_method(iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap(), iter.next().unwrap())
        }
    };

}

macro_rules! register_modules {
    (
        $(
            $module_name:literal => $module_type:ty
        ),* $(,)?
    ) => {
        pub static MODULES: once_cell::sync::Lazy<HashMap<String, Box<dyn BuiltinModule>>> =
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

impl Default for BuiltinModules {
    fn default() -> Self {
        Self::new()
    }
}

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
            None => Err(
                RuntimeError::builtin_error(format!("Unknown module: {module}"), module).boxed(),
            ),
        }
    }

    pub fn get_field(&self, module: &str, field: &str) -> Result<TypedValue> {
        match MODULES.get(module) {
            Some(m) => m.get_field(field),
            None => Err(
                RuntimeError::builtin_error(format!("Unknown module: {module}"), module).boxed(),
            ),
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

mod input;
use input::InputModule;

mod math;
use math::MathModule;

mod http;
use http::HttpModule;

register_modules! {
    "git" => GitModule,
    "env" => EnvModule,
    "cpu" => CpuModule,
    "fs" => FsModule,
    "os" => OsModule,
    "input" => InputModule,
    "math" => MathModule,
    "http" => HttpModule,
}

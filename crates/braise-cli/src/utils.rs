use braise_types::TypeConverter;
use core::{BraiseType, TypedValue, ValueData, ast::Config};
use std::collections::HashMap;
use std::collections::hash_map::Entry;

pub fn find_first_existing_file(files: &[&str]) -> Option<String> {
    for file in files {
        if std::path::Path::new(file).exists() {
            return Some(file.to_string());
        }
    }
    None
}

/// Result of CLI argument parsing
#[derive(Debug)]
pub struct CliArguments {
    pub unnamed: Vec<String>,
    pub named: HashMap<String, TypedValue>,
}

/// Extract arguments from command line args into a HashMap
/// Unnamed arguments (no prefix, no =) are collected separately
/// Supports multiple formats:
/// Passing a comma-separated list as a value will split it into an Array.
/// - `--key value` or `--key=value` (long form)
/// - `-k value` or `-k=value` (short form)  
/// - `key=value` (direct assignment)
/// - `--flag` (boolean flag, becomes true)
pub const ARG_ARRAY_SEP: char = ',';

pub fn extract_args(args: &[String]) -> CliArguments {
    let mut named = HashMap::new();
    let mut unnamed = Vec::new();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        // Helper to parse value string into TypedValue, splitting on ARG_ARRAY_SEP
        fn parse_value(val: &str) -> TypedValue {
            if val.contains(ARG_ARRAY_SEP) {
                let arr: Vec<TypedValue> = val
                    .split(ARG_ARRAY_SEP)
                    .map(|s| TypedValue::new(s.trim().to_string(), BraiseType::String))
                    .collect();
                TypedValue::new(arr, BraiseType::Array(Box::new(BraiseType::String)))
            } else {
                TypedValue::new(val.to_string(), BraiseType::String)
            }
        }

        let mut insert_value = |key: String, value: TypedValue| match named.entry(key) {
            Entry::Vacant(e) => {
                e.insert(value);
            }
            Entry::Occupied(mut e) => {
                let existing = e.get_mut();
                match (&mut existing.value_type, &value.value_type) {
                    (BraiseType::Array(_), BraiseType::Array(_)) => {
                        if let ValueData::Array(arr) = &mut existing.value
                            && let ValueData::Array(new_arr) = value.value
                        {
                            arr.extend(new_arr);
                        }
                    }
                    (BraiseType::Array(_), _) => {
                        if let ValueData::Array(arr) = &mut existing.value {
                            arr.push(value);
                        }
                    }
                    (_, BraiseType::Array(_)) => {
                        if let ValueData::Array(mut arr) = value.value {
                            arr.insert(0, existing.clone());
                            *existing = TypedValue::new(
                                arr,
                                BraiseType::Array(Box::new(BraiseType::String)),
                            );
                        }
                    }
                    (BraiseType::String, BraiseType::String) => {
                        // Convert both to array
                        let arr = vec![existing.clone(), value];
                        *existing =
                            TypedValue::new(arr, BraiseType::Array(Box::new(BraiseType::String)));
                    }
                    _ => {
                        *existing = value;
                    }
                }
            }
        };

        if arg.starts_with("--") {
            let key = arg.trim_start_matches("--");

            if let Some(equals_pos) = key.find('=') {
                let (name, value_str) = key.split_at(equals_pos);
                let value_str = &value_str[1..];
                insert_value(name.to_string(), parse_value(value_str));
            } else if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                insert_value(key.to_string(), parse_value(&args[i + 1]));
                i += 1;
            } else {
                insert_value(key.to_string(), TypedValue::new(true, BraiseType::Bool));
            }
        } else if arg.starts_with('-') && arg.len() > 1 {
            let key = arg.trim_start_matches('-');

            if let Some(equals_pos) = key.find('=') {
                let (name, value_str) = key.split_at(equals_pos);
                let value_str = &value_str[1..];
                insert_value(name.to_string(), parse_value(value_str));
            } else if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                insert_value(key.to_string(), parse_value(&args[i + 1]));
                i += 1;
            } else {
                insert_value(key.to_string(), TypedValue::new(true, BraiseType::Bool));
            }
        } else if let Some(equals_pos) = arg.find('=') {
            let (name, value_str) = arg.split_at(equals_pos);
            let value_str = &value_str[1..];
            insert_value(name.to_string(), parse_value(value_str));
        } else {
            // This is an unnamed argument
            unnamed.push(arg.clone());
        }

        i += 1;
    }

    CliArguments { unnamed, named }
}

/// Resolve CLI arguments by mapping unnamed args to recipe parameters by position
pub fn resolve_args(
    cli_args: CliArguments,
    config: &Config,
    recipe_name: &str,
) -> Result<HashMap<String, TypedValue>, String> {
    // Find the recipe
    let recipe = config
        .recipes
        .iter()
        .find(|r| r.value.name == recipe_name)
        .ok_or_else(|| format!("Recipe '{recipe_name}' not found"))?;

    let mut result = cli_args.named;

    // Map unnamed arguments by position to parameter names
    for (i, unnamed_arg) in cli_args.unnamed.iter().enumerate() {
        if let Some(param_spanned) = recipe.value.parameters.get(i) {
            let param = &param_spanned.value;
            let param_name = &param.name;

            // Check if this parameter was already provided as a named argument
            if result.contains_key(param_name) {
                return Err(format!(
                    "Parameter '{param_name}' specified both as positional argument (position {i}) and named argument"
                ));
            }

            // Convert the string value to appropriate TypedValue
            let typed_value =
                TypeConverter::convert_parameter(unnamed_arg, param_name, &param.param_type)?;
            result.insert(param_name.clone(), typed_value);
        } else {
            return Err(format!(
                "Too many unnamed arguments: recipe '{}' has only {} parameters, but {} unnamed arguments were provided",
                recipe_name,
                recipe.value.parameters.len(),
                cli_args.unnamed.len()
            ));
        }
    }

    Ok(result)
}

use core::{BraiseType, TypedValue};
use std::collections::HashMap;

pub fn find_first_existing_file(files: &[&str]) -> Option<String> {
    for file in files {
        if std::path::Path::new(file).exists() {
            return Some(file.to_string());
        }
    }
    None
}

/// Extract arguments from command line args into a HashMap
/// Supports multiple formats:
/// - `--key value` or `--key=value` (long form)
/// - `-k value` or `-k=value` (short form)  
/// - `key=value` (direct assignment)
/// - `--flag` (boolean flag, becomes true)
pub fn extract_args(args: &[String]) -> HashMap<String, TypedValue> {
    let mut result = HashMap::new();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        if arg.starts_with("--") {
            let key = arg.trim_start_matches("--");

            if let Some(equals_pos) = key.find('=') {
                let (name, value_str) = key.split_at(equals_pos);
                let value_str = &value_str[1..];
                result.insert(
                    name.to_string(),
                    TypedValue::new(value_str.to_string(), BraiseType::String),
                );
            } else if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                result.insert(
                    key.to_string(),
                    TypedValue::new(args[i + 1].to_string(), BraiseType::String),
                );
                i += 1;
            } else {
                result.insert(key.to_string(), TypedValue::new(true, BraiseType::Bool));
            }
        } else if arg.starts_with('-') && arg.len() > 1 {
            let key = arg.trim_start_matches('-');

            if let Some(equals_pos) = key.find('=') {
                let (name, value_str) = key.split_at(equals_pos);
                let value_str = &value_str[1..];
                result.insert(
                    name.to_string(),
                    TypedValue::new(value_str.to_string(), BraiseType::String),
                );
            } else if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                result.insert(
                    key.to_string(),
                    TypedValue::new(args[i + 1].to_string(), BraiseType::String),
                );
                i += 1;
            } else {
                result.insert(key.to_string(), TypedValue::new(true, BraiseType::Bool));
            }
        } else if let Some(equals_pos) = arg.find('=') {
            let (name, value_str) = arg.split_at(equals_pos);
            let value_str = &value_str[1..];
            result.insert(
                name.to_string(),
                TypedValue::new(value_str.to_string(), BraiseType::String),
            );
        }

        i += 1;
    }

    result
}

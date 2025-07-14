use core::{BraiseType, TypedValue, ValueData};
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

/// Extract arguments from command line args into a HashMap
/// Supports multiple formats:
/// Passing a comma-separated list as a value will split it into an Array.
/// - `--key value` or `--key=value` (long form)
/// - `-k value` or `-k=value` (short form)  
/// - `key=value` (direct assignment)
/// - `--flag` (boolean flag, becomes true)
pub const ARG_ARRAY_SEP: char = ',';

pub fn extract_args(args: &[String]) -> HashMap<String, TypedValue> {
    let mut result = HashMap::new();
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

        let mut insert_value = |key: String, value: TypedValue| match result.entry(key) {
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
        }

        i += 1;
    }

    result
}

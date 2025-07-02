use std::collections::HashMap;

pub fn find_first_existing_file(files: &[&str]) -> Option<String> {
    for file in files {
        if std::path::Path::new(file).exists() {
            return Some(file.to_string());
        }
    }
    None
}

// --arg1 value1 --arg2=value2 -x 21 -y=23 --z
// {"arg1": "value1", "arg2": "value2", "x": "21", "y": "23", "z": "true"}
pub fn extract_args(args: &[String]) -> HashMap<String, String> {
    let mut result = HashMap::new();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        if arg.starts_with("--") {
            // Handle long options: --arg or --arg=value
            let arg_name = arg.trim_start_matches("--");

            if let Some(equals_pos) = arg_name.find('=') {
                // Case: --arg=value
                let (name, value) = arg_name.split_at(equals_pos);
                result.insert(name.to_string(), value[1..].to_string());
            } else if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                // Case: --arg value
                result.insert(arg_name.to_string(), args[i + 1].clone());
                i += 1;
            } else {
                // Case: --arg (with no value)
                result.insert(arg_name.to_string(), "true".to_string());
            }
        } else if arg.starts_with('-') {
            // Handle short options: -x or -x=value
            let arg_name = arg.trim_start_matches("-");

            if let Some(equals_pos) = arg_name.find('=') {
                // Case: -x=value
                let (name, value) = arg_name.split_at(equals_pos);
                result.insert(name.to_string(), value[1..].to_string());
            } else if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                // Case: -x value
                result.insert(arg_name.to_string(), args[i + 1].clone());
                i += 1;
            } else {
                // Case: -x (with no value)
                result.insert(arg_name.to_string(), "true".to_string());
            }
        }

        i += 1;
    }

    result
}

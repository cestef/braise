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
            let arg_name = arg.trim_start_matches("--");

            if let Some(equals_pos) = arg_name.find('=') {
                let (name, value) = arg_name.split_at(equals_pos);
                result.insert(name.to_string(), value[1..].to_string());
            } else if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                result.insert(arg_name.to_string(), args[i + 1].clone());
                i += 1;
            } else {
                result.insert(arg_name.to_string(), "true".to_string());
            }
        } else if arg.starts_with('-') {
            let arg_name = arg.trim_start_matches("-");

            if let Some(equals_pos) = arg_name.find('=') {
                let (name, value) = arg_name.split_at(equals_pos);
                result.insert(name.to_string(), value[1..].to_string());
            } else if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                result.insert(arg_name.to_string(), args[i + 1].clone());
                i += 1;
            } else {
                result.insert(arg_name.to_string(), "true".to_string());
            }
        }

        i += 1;
    }

    result
}

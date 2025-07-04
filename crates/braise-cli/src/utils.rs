// crates/braise-cli/src/utils.rs
use runtime::Value;
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
pub fn extract_args(args: &[String]) -> HashMap<String, Value> {
    let mut result = HashMap::new();
    let mut i = 0;

    while i < args.len() {
        let arg = &args[i];

        if arg.starts_with("--") {
            let key = arg.trim_start_matches("--");

            if let Some(equals_pos) = key.find('=') {
                // Format: --key=value
                let (name, value_str) = key.split_at(equals_pos);
                let value_str = &value_str[1..]; // Remove the '='
                result.insert(name.to_string(), Value::String(value_str.to_string()));
            } else if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                // Format: --key value
                result.insert(key.to_string(), Value::String(args[i + 1].to_string()));
                i += 1; // Skip the value
            } else {
                // Format: --flag (boolean)
                result.insert(key.to_string(), Value::Bool(true));
            }
        } else if arg.starts_with('-') && arg.len() > 1 {
            let key = arg.trim_start_matches('-');

            if let Some(equals_pos) = key.find('=') {
                // Format: -k=value
                let (name, value_str) = key.split_at(equals_pos);
                let value_str = &value_str[1..]; // Remove the '='
                result.insert(name.to_string(), Value::String(value_str.to_string()));
            } else if i + 1 < args.len() && !args[i + 1].starts_with('-') {
                // Format: -k value
                result.insert(key.to_string(), Value::String(args[i + 1].to_string()));
                i += 1; // Skip the value
            } else {
                // Format: -flag (boolean)
                result.insert(key.to_string(), Value::Bool(true));
            }
        } else if let Some(equals_pos) = arg.find('=') {
            // Format: key=value (direct assignment)
            let (name, value_str) = arg.split_at(equals_pos);
            let value_str = &value_str[1..]; // Remove the '='
            result.insert(name.to_string(), Value::String(value_str.to_string()));
        }
        // Ignore other formats for now

        i += 1;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_args_long_form() {
        let args = vec![
            "--name".to_string(),
            "Alice".to_string(),
            "--count=42".to_string(),
            "--enabled".to_string(),
        ];

        let result = extract_args(&args);

        assert_eq!(
            result.get("name"),
            Some(&Value::String("Alice".to_string()))
        );
        assert_eq!(result.get("count"), Some(&Value::Number(42.0)));
        assert_eq!(result.get("enabled"), Some(&Value::Bool(true)));
    }

    #[test]
    fn test_extract_args_short_form() {
        let args = vec![
            "-n".to_string(),
            "Bob".to_string(),
            "-c=10".to_string(),
            "-v".to_string(),
        ];

        let result = extract_args(&args);

        assert_eq!(result.get("n"), Some(&Value::String("Bob".to_string())));
        assert_eq!(result.get("c"), Some(&Value::Number(10.0)));
        assert_eq!(result.get("v"), Some(&Value::Bool(true)));
    }

    #[test]
    fn test_extract_args_direct_assignment() {
        let args = vec![
            "name=Charlie".to_string(),
            "age=30".to_string(),
            "active=true".to_string(),
        ];

        let result = extract_args(&args);

        assert_eq!(
            result.get("name"),
            Some(&Value::String("Charlie".to_string()))
        );
        assert_eq!(result.get("age"), Some(&Value::Number(30.0)));
        assert_eq!(result.get("active"), Some(&Value::Bool(true)));
    }

    #[test]
    fn test_extract_args_mixed() {
        let args = vec![
            "--env".to_string(),
            "prod".to_string(),
            "workers=4".to_string(),
            "-v".to_string(),
            "--timeout=30".to_string(),
        ];

        let result = extract_args(&args);

        assert_eq!(result.get("env"), Some(&Value::String("prod".to_string())));
        assert_eq!(result.get("workers"), Some(&Value::Number(4.0)));
        assert_eq!(result.get("v"), Some(&Value::Bool(true)));
        assert_eq!(result.get("timeout"), Some(&Value::Number(30.0)));
    }

    #[test]
    fn test_find_first_existing_file() {
        // This test would need actual files to work properly
        // For now, just test that it returns None when no files exist
        let result = find_first_existing_file(&["nonexistent1.txt", "nonexistent2.txt"]);
        assert_eq!(result, None);
    }
}

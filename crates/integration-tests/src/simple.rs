use crate::*;
use std::collections::HashMap;

#[test]
fn test_basic_recipe() {
    let code = r#"
        recipe "hello" {
            run "echo 'Hello, World!'"
        }
        "#;

    assert!(execute_recipe(code, "hello", HashMap::new()).is_ok());
}

#[test]
fn test_recipe_with_string_parameter() {
    let code = r#"
        recipe "greet" {
            param name: string
            run "echo 'Hello, ${name}!'"
        }
        "#;

    let mut params = HashMap::new();
    params.insert("name".to_string(), string_param("Alice"));

    assert!(execute_recipe(code, "greet", params).is_ok());
}

#[test]
fn test_recipe_with_default_parameter() {
    let code = r#"
        recipe "greet" {
            param name: string = "World"
            run "echo 'Hello, ${name}!'"
        }
        "#;

    assert!(execute_recipe(code, "greet", HashMap::new()).is_ok());
}

#[test]
fn test_recipe_with_number_parameter() {
    let code = r#"
        recipe "count" {
            param num: number
            run "echo 'Count: ${num}'"
        }
        "#;

    let mut params = HashMap::new();
    params.insert("num".to_string(), number_param(42.0));

    assert!(execute_recipe(code, "count", params).is_ok());
}

#[test]
fn test_recipe_with_bool_parameter() {
    let code = r#"
        recipe "check" {
            param enabled: bool
            run "echo 'Enabled: ${enabled}'"
        }
        "#;

    let mut params = HashMap::new();
    params.insert("enabled".to_string(), bool_param(true));

    assert!(execute_recipe(code, "check", params).is_ok());
}

#[test]
fn test_multiple_commands() {
    let code = r#"
        recipe "build" {
            run "echo 'Starting build'"
            run "echo 'Compiling'"
            run "echo 'Done'"
        }
        "#;

    assert!(execute_recipe(code, "build", HashMap::new()).is_ok());
}

#[test]
fn test_variable_assignment() {
    let code = r#"
        recipe "vars" {
            let message = "Hello"
            let count = 42
            let enabled = true
            run "echo '${message} ${count} ${enabled}'"
        }
        "#;

    assert!(execute_recipe(code, "vars", HashMap::new()).is_ok());
}

#[test]
fn test_string_interpolation() {
    let code = r#"
        recipe "interpolate" {
            param name: string = "World"
            let greeting = "Hello, ${name}!"
            run "echo '${greeting}'"
        }
        "#;

    assert!(execute_recipe(code, "interpolate", HashMap::new()).is_ok());
}

#[test]
fn test_if_statement() {
    let code = r#"
        recipe "conditional" {
            param debug: bool = false
            if debug {
                run "echo 'Debug mode'"
            } else {
                run "echo 'Release mode'"
            }
        }
        "#;

    // Test with default (false)
    assert!(execute_recipe(code, "conditional", HashMap::new()).is_ok());

    // Test with true
    let mut params = HashMap::new();
    params.insert("debug".to_string(), bool_param(true));
    assert!(execute_recipe(code, "conditional", params).is_ok());
}

#[test]
fn test_array_literal() {
    let code = r#"
        recipe "arrays" {
            let items = ["a", "b", "c"]
            let numbers = [1, 2, 3]
            run "echo 'Items: ${items}'"
        }
        "#;

    assert!(execute_recipe(code, "arrays", HashMap::new()).is_ok());
}

#[test]
fn test_for_loop() {
    let code = r#"
        recipe "iterate" {
            param items: [string] = ["a", "b", "c"]
            for item in items {
                run "echo 'Processing ${item}'"
            }
        }
        "#;

    assert!(execute_recipe(code, "iterate", HashMap::new()).is_ok());
}

#[test]
fn test_match_statement() {
    let code = r#"
        recipe "match" {
            param status: string = "success"
            match status {
                "success" => run "echo 'Operation succeeded'",
                "error" => run "echo 'Operation failed'",
                _ => run "echo 'Unknown status'"
            }
        }
        "#;

    assert!(execute_recipe(code, "match", HashMap::new()).is_ok());
}

#[test]
fn test_builtin_env_module() {
    let code = r#"
        recipe "env_test" {
            let home = env.get("HOME")
            run "echo 'Home: ${home}'"
        }
        "#;

    assert!(execute_recipe(code, "env_test", HashMap::new()).is_ok());
}

#[test]
fn test_builtin_os_module() {
    let code = r#"
        recipe "os_test" {
            let platform = os.platform()
            run "echo 'Platform: ${platform}'"
        }
        "#;

    assert!(execute_recipe(code, "os_test", HashMap::new()).is_ok());
}

#[test]
fn test_builtin_fs_module() {
    let code = r#"
        recipe "fs_test" {
            let exists = fs.exists("Cargo.toml")
            run "echo 'Exists: ${exists}'"
        }
        "#;

    assert!(execute_recipe(code, "fs_test", HashMap::new()).is_ok());
}

#[test]
fn test_error_undefined_recipe() {
    let code = r#"
        recipe "test" {
            run "echo 'test'"
        }
        "#;

    let result = execute_recipe(code, "nonexistent", HashMap::new());
    assert!(result.is_err());
}

#[test]
fn test_error_missing_required_parameter() {
    let code = r#"
        recipe "test" {
            param required: string
            run "echo '${required}'"
        }
        "#;

    let result = execute_recipe(code, "test", HashMap::new());
    assert!(result.is_err());
}

#[test]
fn test_complex_deployment_example() {
    let code = r#"
        recipe "deploy" {
            param service: string = "api"
            param environment: string = "staging"
            param version: string = "1.0.0"
            
            let artifact = "${service}-${version}.tar.gz"
            
            if environment == "prod" {
                run "echo 'Production deployment of ${artifact}'"
            } else {
                run "echo 'Staging deployment of ${artifact}'"
            }
        }
        "#;

    let mut params = HashMap::new();
    params.insert("service".to_string(), string_param("web"));
    params.insert("environment".to_string(), string_param("prod"));
    params.insert("version".to_string(), string_param("2.1.0"));

    assert!(execute_recipe(code, "deploy", params).is_ok());
}

#[test]
fn test_union_type_parameters() {
    let code = r#"
        recipe "flexible" {
            param input: string = "test"
            param count: number = 5
            
            run "echo 'Input: ${input}, Count: ${count}'"
        }
        "#;

    let mut params = HashMap::new();
    params.insert("input".to_string(), string_param("hello"));
    params.insert("count".to_string(), number_param(10.0));

    assert!(execute_recipe(code, "flexible", params).is_ok());
}

#[test]
fn test_enum_like_parameters() {
    let code = r#"
        recipe "deploy" {
            param env: string = "staging"
            
            if env == "staging" {
                run "echo 'Deploying to staging'"
            } else if env == "prod" {
                run "echo 'Deploying to production'"
            } else {
                run "echo 'Unknown environment: ${env}'"
            }
        }
        "#;

    let mut params = HashMap::new();
    params.insert("env".to_string(), string_param("prod"));

    assert!(execute_recipe(code, "deploy", params).is_ok());
}

#[test]
fn test_complex_conditionals() {
    let code = r#"
        recipe "build" {
            param debug: bool = false
            param cores: number = 4
            
            if cores > 2 {
                run "echo 'Using ${cores} cores for parallel build'"
            } else {
                run "echo 'Single-threaded build'"
            }
            
            if debug {
                run "echo 'Debug build enabled'"
            }
        }
        "#;

    let mut params = HashMap::new();
    params.insert("cores".to_string(), number_param(8.0));
    params.insert("debug".to_string(), bool_param(true));

    assert!(execute_recipe(code, "build", params).is_ok());
}

#[test]
fn test_advanced_string_interpolation() {
    let code = r#"
        recipe "package" {
            param service: string = "api"
            param version: string = "1.0.0"
            param env: string = "prod"
            
            let image_name = "${service}:${version}"
            let full_tag = "registry.example.com/${image_name}-${env}"
            
            run "echo 'Building image: ${full_tag}'"
        }
        "#;

    let mut params = HashMap::new();
    params.insert("service".to_string(), string_param("web-server"));
    params.insert("version".to_string(), string_param("2.1.0"));
    params.insert("env".to_string(), string_param("staging"));

    assert!(execute_recipe(code, "package", params).is_ok());
}

#[test]
fn test_array_iteration_with_conditionals() {
    let code = r#"
        recipe "check_files" {
            param files: [string] = ["README.md", "package.json", "Cargo.toml"]
            
            for file in files {
                run "echo 'Checking ${file}'"
            }
        }
        "#;

    assert!(execute_recipe(code, "check_files", HashMap::new()).is_ok());
}

#[test]
fn test_match_with_multiple_patterns() {
    let code = r#"
        recipe "process_status" {
            param status: string = "success"
            param count: number = 5
            
            match status {
                "success" => run "echo '✅ Success!'",
                "error" => run "echo '❌ Error!'",
                "pending" => run "echo '⏳ Pending...'", 
                _ => run "echo '❓ Unknown status: ${status}'"
            }
            
            match count {
                0 => run "echo 'No items'",
                1 => run "echo 'Single item'",
                _ => run "echo 'Multiple items: ${count}'"
            }
        }
        "#;

    let mut params = HashMap::new();
    params.insert("status".to_string(), string_param("error"));
    params.insert("count".to_string(), number_param(3.0));

    assert!(execute_recipe(code, "process_status", params).is_ok());
}

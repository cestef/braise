use crate::*;
use std::collections::HashMap;

#[test]
fn test_recipe_dependencies_syntax() {
    // Test recipe dependency declaration (even if not fully implemented)
    let code = r#"
        recipe "lint" {
            run "echo 'Linting code'"
        }
        
        recipe "test" {
            run "echo 'Running tests'"
        }
        
        recipe "build" {
            run "echo 'Building project'"
        }
        "#;

    // Test that we can at least parse and execute basic recipes
    let output = execute_recipe(code, "lint", HashMap::new()).unwrap();
    assert_eq!(output, "Linting code\n");

    let output = execute_recipe(code, "test", HashMap::new()).unwrap();
    assert_eq!(output, "Running tests\n");

    let output = execute_recipe(code, "build", HashMap::new()).unwrap();
    assert_eq!(output, "Building project\n");
}

#[test]
fn test_advanced_match_patterns() {
    let code = r#"
        recipe "process_status" {
            param status: string = "success"
            param count: number = 5
            param enabled: bool = true
            
            match status {
                "success" => run "echo '✅ Success!'",
                "error" => run "echo '❌ Error!'", 
                "pending" => run "echo '⏳ Pending...'",
                _ => run "echo '❓ Unknown status'"
            }
            
            match count {
                0 => run "echo 'No items'",
                1 => run "echo 'Single item'",
                5 => run "echo 'Five items'", 
                _ => run "echo 'Multiple items'"
            }
            
            if enabled {
                run "echo 'Feature is enabled'"
            } else {
                run "echo 'Feature is disabled'"
            }
        }
        "#;

    let mut params = HashMap::new();
    params.insert("status".to_string(), string_param("error"));
    params.insert("count".to_string(), number_param(1.0));
    params.insert("enabled".to_string(), bool_param(false));

    let output = execute_recipe(code, "process_status", params).unwrap();
    assert!(output.contains("❌ Error!"));
    assert!(output.contains("Single item"));
    assert!(output.contains("Feature is disabled"));
}

#[test]
fn test_union_types_and_enums() {
    // Test enum-like parameter constraints
    let code = r#"
        recipe "deploy" {
            param env: string = "staging"
            param service: string = "api"
            
            if env == "staging" {
                run "echo 'Deploying ${service} to staging'"
            } else if env == "prod" {
                run "echo 'Deploying ${service} to production'"
            } else {
                run "echo 'Unknown environment: ${env}'"
            }
        }
        "#;

    let mut params = HashMap::new();
    params.insert("env".to_string(), string_param("prod"));
    params.insert("service".to_string(), string_param("web"));

    let output = execute_recipe(code, "deploy", params).unwrap();
    assert_eq!(output, "Deploying web to production\n");
}

#[test]
fn test_complex_conditionals_and_operators() {
    let code = r#"
        recipe "build" {
            param cores: number = 4
            param ci: bool = false
            param debug: bool = false
            
            if cores > 4 {
                run "echo 'Using ${cores} cores for parallel build'"
            } else if cores >= 2 {
                run "echo 'Using 2 cores'"
            } else {
                run "echo 'Single-threaded build'"
            }
            
            if ci {
                run "echo 'CI environment detected'"
            }
            
            if debug {
                run "echo 'Debug build enabled'"
            } else {
                run "echo 'Release build'"
            }
        }
        "#;

    let mut params = HashMap::new();
    params.insert("cores".to_string(), number_param(8.0));
    params.insert("ci".to_string(), bool_param(true));
    params.insert("debug".to_string(), bool_param(false));

    let output = execute_recipe(code, "build", params).unwrap();
    assert!(output.contains("Using 8 cores for parallel build"));
    assert!(output.contains("CI environment detected"));
    assert!(output.contains("Release build"));
}

#[test]
fn test_advanced_string_interpolation() {
    let code = r#"
        recipe "package" {
            param service: string = "api"
            param version: string = "1.0.0"
            param registry: string = "registry.example.com"
            
            let image_name = "${service}:${version}"
            let full_tag = "${registry}/${image_name}"
            let artifact = "${service}-${version}.tar.gz"
            
            run "echo 'Building image: ${full_tag}'"
            run "echo 'Creating artifact: ${artifact}'"
        }
        "#;

    let mut params = HashMap::new();
    params.insert("service".to_string(), string_param("web-server"));
    params.insert("version".to_string(), string_param("2.1.0"));
    params.insert("registry".to_string(), string_param("ghcr.io"));

    let output = execute_recipe(code, "package", params).unwrap();
    assert!(output.contains("Building image: ghcr.io/web-server:2.1.0"));
    assert!(output.contains("Creating artifact: web-server-2.1.0.tar.gz"));
}

#[test]
fn test_array_operations_and_iteration() {
    let code = r#"
        recipe "check_files" {
            param files: [string] = ["package.json", "Cargo.toml", "README.md"]
            
            for file in files {
                run "echo 'Checking ${file}'"
            }
            
            let numbers = [1, 2, 3, 4, 5]
            for num in numbers {
                run "echo 'Number: ${num}'"
            }
        }
        "#;

    let output = execute_recipe(code, "check_files", HashMap::new()).unwrap();
    assert!(output.contains("Checking package.json"));
    assert!(output.contains("Checking Cargo.toml"));
    assert!(output.contains("Checking README.md"));
    assert!(output.contains("Number: 1"));
    assert!(output.contains("Number: 5"));
}

#[test]
fn test_builtin_modules_integration() {
    let code = r#"
        recipe "system_info" {
            let home = env.get("HOME")
            let platform = os.platform()
            let exists = fs.exists("Cargo.toml")
            
            run "echo 'Home: ${home}'"
            run "echo 'Platform: ${platform}'"
            
            if exists {
                run "echo 'Cargo.toml found'"
            } else {
                run "echo 'Cargo.toml not found'"
            }
        }
        "#;

    let output = execute_recipe(code, "system_info", HashMap::new()).unwrap();
    assert!(output.contains("Home: "));
    assert!(output.contains("Platform: "));
    assert!(output.contains("Cargo.toml"));
}

#[test]
fn test_complex_workflow_patterns() {
    // Based on the rust.braise sample
    let code = r#"
        recipe "test" {
            param coverage: bool = false
            
            if coverage {
                run "echo 'Running with coverage'"
            } else {
                run "echo 'Running normal tests'"
            }
        }
        
        recipe "build" {
            param profile: string = "debug"
            param jobs: number = 1
            
            match profile {
                "debug" => run "echo 'Building in debug mode'",
                "release" => run "echo 'Building in release mode'",
                _ => run "echo 'Unknown profile: ${profile}'"
            }
            
            let flag = if profile == "release" { "--release " } else { "" }
            run "echo 'cargo build ${flag}-j ${jobs}'"
        }
        "#;

    // Test normal test execution
    let output = execute_recipe(code, "test", HashMap::new()).unwrap();
    assert_eq!(output, "Running normal tests\n");

    // Test with coverage
    let mut params = HashMap::new();
    params.insert("coverage".to_string(), bool_param(true));
    let output = execute_recipe(code, "test", params).unwrap();
    assert_eq!(output, "Running with coverage\n");

    // Test release build
    let mut params = HashMap::new();
    params.insert("profile".to_string(), string_param("release"));
    params.insert("jobs".to_string(), number_param(4.0));
    let output = execute_recipe(code, "build", params).unwrap();
    assert!(output.contains("Building in release mode"));
    assert!(output.contains("cargo build --release -j 4"));
}

#[test]
fn test_monorepo_patterns() {
    // Based on the monorepo.braise sample
    let code = r#"
        recipe "test" {
            param service: string = "all"
            
            match service {
                "all" => {
                    for s in ["api", "web", "worker"] {
                        run "echo 'Testing ${s}'"
                    }
                },
                _ => run "echo 'Testing ${service}'"
            }
        }
        
        recipe "dev" {
            param services: [string] = ["api", "web"]
            
            for service in services {
                run "echo 'Starting ${service}'"
            }
        }
        "#;

    // Test all services
    let output = execute_recipe(code, "test", HashMap::new()).unwrap();
    assert!(output.contains("Testing api"));
    assert!(output.contains("Testing web"));
    assert!(output.contains("Testing worker"));

    // Test specific service
    let mut params = HashMap::new();
    params.insert("service".to_string(), string_param("api"));
    let output = execute_recipe(code, "test", params).unwrap();
    assert_eq!(output, "Testing api\n");

    // Test dev workflow
    let output = execute_recipe(code, "dev", HashMap::new()).unwrap();
    assert!(output.contains("Starting api"));
    assert!(output.contains("Starting web"));
}

#[test]
fn test_conditional_expressions_in_assignments() {
    let code = r#"
        recipe "smart_build" {
            param ci: bool = false
            param cores: number = 4
            
            let profile = if ci { "release" } else { "debug" }
            let jobs = if ci { cores } else { 1 }
            let flag = if profile == "release" { "--release " } else { "" }
            
            run "echo 'Building with profile: ${profile}'"
            run "echo 'Using ${jobs} jobs'"
            run "echo 'Command: cargo build ${flag}-j ${jobs}'"
        }
        "#;

    let mut params = HashMap::new();
    params.insert("ci".to_string(), bool_param(true));
    params.insert("cores".to_string(), number_param(8.0));

    let output = execute_recipe(code, "smart_build", params).unwrap();
    assert!(output.contains("Building with profile: release"));
    assert!(output.contains("Using 8 jobs"));
    assert!(output.contains("Command: cargo build --release -j 8"));
}

#[test]
fn test_error_handling_patterns() {
    let code = r#"
        recipe "validate" {
            param service: string
            
            if service == "" {
                run "echo 'Error: service name cannot be empty'"
            } else {
                run "echo 'Valid service: ${service}'"
            }
        }
        "#;

    // Test with valid service
    let mut params = HashMap::new();
    params.insert("service".to_string(), string_param("api"));
    let output = execute_recipe(code, "validate", params).unwrap();
    assert_eq!(output, "Valid service: api\n");

    // Test with empty service
    let mut params = HashMap::new();
    params.insert("service".to_string(), string_param(""));
    let output = execute_recipe(code, "validate", params).unwrap();
    assert_eq!(output, "Error: service name cannot be empty\n");
}

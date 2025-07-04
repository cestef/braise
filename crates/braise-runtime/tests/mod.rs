#[cfg(test)]
mod tests {
    use braise_runtime::{Runtime, StringExecutor, Value};
    use core::runtime::RuntimeError;
    use core::{BraiseError, ParamType};
    use lexer::tokenize;
    use miette::Result;
    use parser::Parser;
    use std::collections::HashMap;

    fn create_runtime(input: &str) -> Result<Runtime> {
        let tokens = tokenize(&input).map_err(|error_span| BraiseError::LexerError {
            code: input.to_string(),
            span: miette::SourceSpan::new(error_span.start.into(), error_span.len()),
        })?;
        let mut parser = Parser::new(tokens, input.to_string(), "test.braise".to_string());
        let ast = parser.parse()?;
        Ok(Runtime::new(ast).with_executor(StringExecutor::new(false)))
    }

    #[test]
    fn test_circular_dependency_detection() -> Result<()> {
        let input = r#"
        recipe "a" -> ["b"] {
            run "echo a"
        }
        recipe "b" -> ["c"] {
            run "echo b"
        }
        recipe "c" -> ["a"] {
            run "echo c"
        }
        "#;

        let runtime = create_runtime(input)?;

        let result = runtime.execute_recipe("a", HashMap::new());
        assert!(result.is_err());

        match result.unwrap_err() {
            RuntimeError::CircularDependency { recipe, stack } => {
                assert_eq!(recipe, "a");
                assert!(stack.contains("a"));
                assert!(stack.contains("b"));
                assert!(stack.contains("c"));
            }
            _ => panic!("Expected circular dependency error"),
        }
        Ok(())
    }

    #[test]
    fn test_recipe_execution() -> Result<()> {
        let input = r#"
        recipe "greet" {
            run "echo Hello, World!"
        }
        "#;

        let runtime = create_runtime(input)?;
        let result = runtime.execute_recipe("greet", HashMap::new());
        assert!(result.is_ok());

        Ok(())
    }

    #[test]
    fn test_undefined_recipe_error() -> Result<()> {
        let input = r#"
        recipe "greet" {
            run "echo Hello, World!"
        }
        "#;
        let runtime = create_runtime(input)?;
        let result = runtime.execute_recipe("undefined_recipe", HashMap::new());
        assert!(result.is_err());
        match result.unwrap_err() {
            RuntimeError::UndefinedRecipe(name) => {
                assert_eq!(name, "undefined_recipe");
            }
            _ => panic!("Expected undefined recipe error"),
        }

        Ok(())
    }

    #[test]
    fn test_undefined_variable_error() -> Result<()> {
        let input1 = r#"
        recipe "greet" {
            run "echo ${undefined_variable}"
        }
        "#;
        let input2 = r#"
        recipe "greet" {
            let name: string = undefined_variable
            run "echo ${name}"
        }"#;
        for input in [input1, input2] {
            let runtime = create_runtime(input)?;
            let result = runtime.execute_recipe("greet", HashMap::new());
            assert!(result.is_err());
            match result.unwrap_err() {
                RuntimeError::UndefinedVariable(var) => {
                    assert_eq!(var, "undefined_variable");
                }
                _ => panic!("Expected undefined variable error"),
            }
        }

        Ok(())
    }

    #[test]
    fn test_variable_assignment_and_usage() -> Result<()> {
        let input = r#"
        recipe "greet" {
            let name: string = "World"
            run "echo Hello, ${name}!"
        }"#;
        let runtime = create_runtime(input)?;
        let result = runtime.execute_recipe("greet", HashMap::new());
        assert!(result.is_ok());
        assert!(runtime.executor.output().unwrap().contains("Hello, World!"));
        Ok(())
    }

    #[test]
    fn test_shell_setting() -> Result<()> {
        let input = r#"
        recipe "shell" {
            shell "bash -c"
            run "echo using $0"
            shell "zsh -c"
            run "echo using $0"
        }"#;
        let runtime = create_runtime(input)?;
        let result = runtime.execute_recipe("shell", HashMap::new());
        assert!(result.is_ok());
        let output = runtime.executor.output().unwrap();

        // Check that bash is mentioned first, then zsh
        let bash_pos = output.find("bash").expect("Output should contain 'bash'");
        let zsh_pos = output.find("zsh").expect("Output should contain 'zsh'");
        assert!(
            bash_pos < zsh_pos,
            "bash should appear before zsh in the output"
        );

        Ok(())
    }

    #[test]
    fn test_recipe_with_parameters() -> Result<()> {
        let input = r#"
        recipe "greet" {
            param name: string
            run "echo Hello, ${name}!"
        }"#;
        let runtime = create_runtime(input)?;
        let mut params = HashMap::new();
        params.insert("name".to_string(), Value::String("Alice".to_string()));
        let result = runtime.execute_recipe("greet", params);
        assert!(result.is_ok());
        assert!(runtime.executor.output().unwrap().contains("Hello, Alice!"));

        let result = runtime.execute_recipe("greet", HashMap::new());
        assert!(result.is_err());

        match result.unwrap_err() {
            RuntimeError::InvalidParameter { name, .. } => {
                assert_eq!(name, "name");
            }
            _ => panic!("Expected missing parameter error"),
        }
        Ok(())
    }

    #[test]
    fn test_value_display() {
        assert_eq!(Value::String("hello".to_string()).to_string(), "hello");
        assert_eq!(Value::Number(42.0).to_string(), "42");
        assert_eq!(Value::Number(3.14).to_string(), "3.14");
        assert_eq!(Value::Bool(true).to_string(), "true");
        assert_eq!(Value::Bool(false).to_string(), "false");

        let array = Value::Array(vec![
            Value::String("a".to_string()),
            Value::Number(1.0),
            Value::Bool(true),
        ]);
        assert_eq!(array.to_string(), "[a, 1, true]");

        let mut args = HashMap::new();
        args.insert("param1".to_string(), Value::String("value1".to_string()));
        let recipe = Value::Recipe("test".to_string(), args);
        assert_eq!(recipe.to_string(), "test(param1: value1)");
    }

    #[test]
    fn test_value_to_bool() {
        // Bool values
        assert!(Value::Bool(true).to_bool());
        assert!(!Value::Bool(false).to_bool());

        // String values
        assert!(!Value::String("".to_string()).to_bool()); // empty string is false
        assert!(Value::String("hello".to_string()).to_bool()); // non-empty is true

        // Number values
        assert!(!Value::Number(0.0).to_bool()); // zero is false
        assert!(Value::Number(42.0).to_bool()); // non-zero is true
        assert!(Value::Number(-1.0).to_bool()); // negative is true

        // Array values
        assert!(!Value::Array(vec![]).to_bool()); // empty array is false
        assert!(Value::Array(vec![Value::String("test".to_string())]).to_bool()); // non-empty is true

        // Recipe values
        assert!(!Value::Recipe("test".to_string(), HashMap::new()).to_bool()); // no args is false
        let mut args = HashMap::new();
        args.insert("param".to_string(), Value::String("value".to_string()));
        assert!(Value::Recipe("test".to_string(), args).to_bool()); // with args is true
    }

    #[test]
    fn test_value_to_number() {
        // Number values
        assert_eq!(Value::Number(42.0).to_number().unwrap(), 42.0);
        assert_eq!(Value::Number(-3.14).to_number().unwrap(), -3.14);

        // String values
        assert_eq!(Value::String("123".to_string()).to_number().unwrap(), 123.0);
        assert_eq!(Value::String("3.14".to_string()).to_number().unwrap(), 3.14);
        assert!(
            Value::String("not_a_number".to_string())
                .to_number()
                .is_err()
        );

        // Bool values
        assert_eq!(Value::Bool(true).to_number().unwrap(), 1.0);
        assert_eq!(Value::Bool(false).to_number().unwrap(), 0.0);

        // Array and Recipe should fail
        assert!(Value::Array(vec![]).to_number().is_err());
        assert!(
            Value::Recipe("test".to_string(), HashMap::new())
                .to_number()
                .is_err()
        );
    }

    #[test]
    fn test_value_type_checking() {
        assert!(Value::String("hello".to_string()).is::<String>());
        assert!(!Value::String("hello".to_string()).is::<f64>());

        assert!(Value::Number(42.0).is::<f64>());
        assert!(!Value::Number(42.0).is::<String>());

        assert!(Value::Bool(true).is::<bool>());
        assert!(!Value::Bool(true).is::<String>());

        assert!(Value::Array(vec![]).is::<Vec<Value>>());
        assert!(!Value::Array(vec![]).is::<String>());
    }

    #[test]
    fn test_value_type_names() {
        assert_eq!(Value::String("test".to_string()).type_name(), "string");
        assert_eq!(Value::Number(42.0).type_name(), "number");
        assert_eq!(Value::Bool(true).type_name(), "boolean");
        assert_eq!(Value::Array(vec![]).type_name(), "array");
        assert_eq!(
            Value::Recipe("test".to_string(), HashMap::new()).type_name(),
            "recipe"
        );
    }

    #[test]
    fn test_value_default_for_type() {
        let string_default = Value::default_for_type(&ParamType::String);
        assert_eq!(string_default, Value::String("".to_string()));

        let number_default = Value::default_for_type(&ParamType::Number);
        assert_eq!(number_default, Value::Number(0.0));

        let bool_default = Value::default_for_type(&ParamType::Bool);
        assert_eq!(bool_default, Value::Bool(false));

        let array_default = Value::default_for_type(&ParamType::Array(Box::new(ParamType::String)));
        assert_eq!(array_default, Value::Array(vec![]));

        let enum_default = Value::default_for_type(&ParamType::Enum(vec![]));
        assert_eq!(enum_default, Value::String("".to_string()));

        let recipe_default = Value::default_for_type(&ParamType::Recipe);
        assert_eq!(
            recipe_default,
            Value::Recipe("".to_string(), HashMap::new())
        );
    }

    #[test]
    fn test_value_from_str() {
        // String type
        let result = Value::from_str("hello", &"test".to_string(), &ParamType::String).unwrap();
        assert_eq!(result, Value::String("hello".to_string()));

        // Number type
        let result = Value::from_str("42", &"test".to_string(), &ParamType::Number).unwrap();
        assert_eq!(result, Value::Number(42.0));

        let result = Value::from_str("3.14", &"test".to_string(), &ParamType::Number).unwrap();
        assert_eq!(result, Value::Number(3.14));

        // Invalid number
        let result = Value::from_str("not_a_number", &"test".to_string(), &ParamType::Number);
        assert!(result.is_err());

        // Bool type
        let result = Value::from_str("true", &"test".to_string(), &ParamType::Bool).unwrap();
        assert_eq!(result, Value::Bool(true));

        let result = Value::from_str("false", &"test".to_string(), &ParamType::Bool).unwrap();
        assert_eq!(result, Value::Bool(false));

        let result = Value::from_str("1", &"test".to_string(), &ParamType::Bool).unwrap();
        assert_eq!(result, Value::Bool(true));

        let result = Value::from_str("0", &"test".to_string(), &ParamType::Bool).unwrap();
        assert_eq!(result, Value::Bool(false));

        // Invalid bool
        let result = Value::from_str("maybe", &"test".to_string(), &ParamType::Bool);
        assert!(result.is_err());

        // Enum type
        let enum_type = ParamType::Enum(vec!["dev".to_string(), "prod".to_string()]);
        let result = Value::from_str("dev", &"test".to_string(), &enum_type).unwrap();
        assert_eq!(result, Value::String("dev".to_string()));

        // Invalid enum value
        let result = Value::from_str("staging", &"test".to_string(), &enum_type);
        assert!(result.is_err());

        // Array type
        let array_type = ParamType::Array(Box::new(ParamType::String));
        let result = Value::from_str("a,b,c", &"test".to_string(), &array_type).unwrap();
        assert_eq!(
            result,
            Value::Array(vec![
                Value::String("a".to_string()),
                Value::String("b".to_string()),
                Value::String("c".to_string())
            ])
        );

        // Array of numbers
        let number_array_type = ParamType::Array(Box::new(ParamType::Number));
        let result = Value::from_str("1,2,3", &"test".to_string(), &number_array_type).unwrap();
        assert_eq!(
            result,
            Value::Array(vec![
                Value::Number(1.0),
                Value::Number(2.0),
                Value::Number(3.0)
            ])
        );
    }

    #[test]
    fn test_value_convert_to_type() {
        // String to number
        let string_val = Value::String("42".to_string());
        let converted = string_val.convert_to_type(&ParamType::Number).unwrap();
        assert_eq!(converted, Value::Number(42.0));

        // Number to string
        let number_val = Value::Number(42.0);
        let converted = number_val.convert_to_type(&ParamType::String).unwrap();
        assert_eq!(converted, Value::String("42".to_string()));

        // Bool to string
        let bool_val = Value::Bool(true);
        let converted = bool_val.convert_to_type(&ParamType::String).unwrap();
        assert_eq!(converted, Value::String("true".to_string()));

        // Array conversion
        let array_val = Value::Array(vec![
            Value::String("1".to_string()),
            Value::String("2".to_string()),
        ]);
        let array_type = ParamType::Array(Box::new(ParamType::Number));
        let converted = array_val.convert_to_type(&array_type).unwrap();
        assert_eq!(
            converted,
            Value::Array(vec![Value::Number(1.0), Value::Number(2.0)])
        );

        // Enum conversion
        let string_val = Value::String("dev".to_string());
        let enum_type = ParamType::Enum(vec!["dev".to_string(), "prod".to_string()]);
        let converted = string_val.convert_to_type(&enum_type).unwrap();
        assert_eq!(converted, Value::String("dev".to_string()));

        // Invalid enum conversion
        let string_val = Value::String("staging".to_string());
        let result = string_val.convert_to_type(&enum_type);
        assert!(result.is_err());

        // Recipe conversion
        let string_val = Value::String("test_recipe".to_string());
        let converted = string_val.convert_to_type(&ParamType::Recipe).unwrap();
        assert_eq!(
            converted,
            Value::Recipe("test_recipe".to_string(), HashMap::new())
        );
    }

    #[test]
    fn test_value_conversions() {
        // Test From implementations
        let string_val = Value::String("hello".to_string());
        let string_result: String = string_val.into();
        assert_eq!(string_result, "hello");

        let number_val = Value::Number(42.0);
        let number_result: f64 = number_val.into();
        assert_eq!(number_result, 42.0);

        let bool_val = Value::Bool(true);
        let bool_result: bool = bool_val.into();
        assert_eq!(bool_result, true);
    }
}

#[cfg(test)]
mod tests {
    use braise_errors::RuntimeError;
    use braise_runtime::{Runtime, StringExecutor};
    use core::{BraiseType, TypedValue};
    use lexer::tokenize;
    use miette::Result;
    use parser::Parser;
    use std::collections::HashMap;

    fn create_runtime(input: &str) -> Result<Runtime> {
        let tokens = tokenize(&input)?;
        let mut parser = Parser::new(tokens, input.to_string().into(), "test.braise".to_string());
        let ast = parser.parse()?;
        Ok(Runtime::new(ast, input.to_string().into()).with_executor(StringExecutor::new(false)))
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
            RuntimeError::CircularDependency { recipe, stack, .. } => {
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
            RuntimeError::UndefinedRecipe { name, .. } => {
                assert_eq!(name, "undefined_recipe");
            }
            _ => panic!("Expected undefined recipe error"),
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
        params.insert(
            "name".to_string(),
            TypedValue::new("Alice".to_string(), BraiseType::String),
        );
        let result = runtime.execute_recipe("greet", params);
        assert!(result.is_ok());
        assert!(runtime.executor.output().unwrap().contains("Hello, Alice!"));
        let runtime = create_runtime(input)?;
        let result = runtime.execute_recipe("greet", HashMap::new());
        assert!(result.is_err());

        match result.unwrap_err() {
            RuntimeError::MissingRequiredParameter {
                name,
                expected_type,
                ..
            } => {
                assert_eq!(name, "name");
                assert_eq!(expected_type, "string");
            }
            e => panic!(
                "Expected missing parameter or circular dependency error, got {:?}",
                e
            ),
        }
        Ok(())
    }
}

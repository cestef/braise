#[cfg(test)]
mod tests {
    use braise_runtime::{Runtime, StringExecutor, Value};
    use core::BraiseError;
    use core::runtime::RuntimeError;
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
}

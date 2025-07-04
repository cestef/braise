#[cfg(test)]
mod tests {
    use braise_parser::*;
    use core::ast::*;
    use core::parser::*;
    use lexer::tokenize;

    fn parse_input(input: &str) -> Result<Config> {
        let tokens =
            tokenize(input).map_err(|_| ParseError::Other("Tokenization failed".to_string()))?;
        let mut parser = Parser::new(tokens, input.to_string(), "test.braise".to_string());
        parser.parse()
    }

    #[test]
    fn test_simple_recipe() {
        let input = r#"
        recipe "test" {
            run "echo hello"
        }
        "#;

        let config = parse_input(input).unwrap();
        assert_eq!(config.recipes.len(), 1);

        let recipe = &config.recipes[0].value;
        assert_eq!(recipe.name, "test");
        assert_eq!(recipe.dependencies.len(), 0);
        assert_eq!(recipe.parameters.len(), 0);
        assert_eq!(recipe.body.len(), 1);

        match &recipe.body[0].value {
            Statement::Run(expr) => match &expr.value {
                Expression::String(s) => assert_eq!(s, "echo hello"),
                _ => panic!("Expected string expression"),
            },
            _ => panic!("Expected run statement"),
        }
    }

    #[test]
    fn test_recipe_with_dependencies() {
        let input = r#"
        recipe "main" -> ["dep1", "dep2"] {
            run "echo main"
        }
        "#;

        let config = parse_input(input).unwrap();
        let recipe = &config.recipes[0].value;

        assert_eq!(recipe.name, "main");
        assert_eq!(recipe.dependencies, vec!["dep1", "dep2"]);
    }

    #[test]
    fn test_recipe_with_parameters() {
        let input = r#"
        recipe "test" {
            param name: string = "default"
            param count: number
            param enabled: bool = true
            param items: [string] = ["a", "b"]
            param env: ["dev", "prod"] = "dev"
            
            run "echo ${name}"
        }
        "#;

        let config = parse_input(input).unwrap();
        let recipe = &config.recipes[0].value;

        assert_eq!(recipe.parameters.len(), 5);

        // Check string parameter with default
        let name_param = &recipe.parameters[0].value;
        assert_eq!(name_param.name, "name");
        assert_eq!(name_param.param_type, ParamType::String);
        assert!(name_param.default.is_some());

        // Check number parameter without default
        let count_param = &recipe.parameters[1].value;
        assert_eq!(count_param.name, "count");
        assert_eq!(count_param.param_type, ParamType::Number);
        assert!(count_param.default.is_none());

        // Check bool parameter with default
        let enabled_param = &recipe.parameters[2].value;
        assert_eq!(enabled_param.name, "enabled");
        assert_eq!(enabled_param.param_type, ParamType::Bool);

        // Check array parameter
        let items_param = &recipe.parameters[3].value;
        assert_eq!(items_param.name, "items");
        if let ParamType::Array(element_type) = &items_param.param_type {
            assert_eq!(**element_type, ParamType::String);
        } else {
            panic!("Expected array type");
        }

        // Check enum parameter
        let env_param = &recipe.parameters[4].value;
        assert_eq!(env_param.name, "env");
        if let ParamType::Enum(variants) = &env_param.param_type {
            assert_eq!(*variants, vec!["dev", "prod"]);
        } else {
            panic!("Expected enum type");
        }
    }

    #[test]
    fn test_control_flow_statements() {
        let input = r#"
        recipe "test" {
            if condition {
                run "echo true"
            } else {
                run "echo false"
            }
            
            match value {
                "option1" => { run "echo 1" },
                "option2" => { run "echo 2" },
                _ => { run "echo default" }
            }
            
            for item in items {
                run "echo ${item}"
            }
        }
        "#;

        let config = parse_input(input).unwrap();
        let recipe = &config.recipes[0].value;

        assert_eq!(recipe.body.len(), 3);

        // Check if statement
        match &recipe.body[0].value {
            Statement::If {
                condition,
                then_block,
                else_block,
            } => {
                assert!(matches!(condition.value, Expression::Variable(_)));
                assert_eq!(then_block.len(), 1);
                assert!(else_block.is_some());
                assert_eq!(else_block.as_ref().unwrap().len(), 1);
            }
            _ => panic!("Expected if statement"),
        }

        // Check match statement
        match &recipe.body[1].value {
            Statement::Match { expr, arms } => {
                assert!(matches!(expr.value, Expression::Variable(_)));
                assert_eq!(arms.len(), 3);

                // Check wildcard pattern
                match &arms[2].value.pattern {
                    MatchPattern::Wildcard => {}
                    _ => panic!("Expected wildcard pattern"),
                }
            }
            _ => panic!("Expected match statement"),
        }

        // Check for statement
        match &recipe.body[2].value {
            Statement::For {
                var,
                iterable,
                body,
                is_async,
            } => {
                assert_eq!(var, "item");
                assert!(matches!(iterable.value, Expression::Variable(_)));
                assert_eq!(body.len(), 1);
                assert!(!is_async);
            }
            _ => panic!("Expected for statement"),
        }
    }

    #[test]
    fn test_expressions() {
        let input = r#"
        recipe "test" {
            let result: string = if condition { "yes" } else { "no" }
            let computed: number = x > 5 && y < 10
            let module_call: string = env.get("HOME")
            let interpolated: string = "Hello ${name}!"
            let array_val: [string] = ["a", "b", "c"]
        }
        "#;

        let config = parse_input(input).unwrap();
        let recipe = &config.recipes[0].value;

        assert_eq!(recipe.body.len(), 5);

        // Check conditional expression
        if let Statement::Let {
            value: Some(expr), ..
        } = &recipe.body[0].value
        {
            assert!(matches!(expr.value, Expression::Conditional { .. }));
        }

        // Check binary operation
        if let Statement::Let {
            value: Some(expr), ..
        } = &recipe.body[1].value
        {
            assert!(matches!(expr.value, Expression::BinaryOp { .. }));
        }

        // Check function call
        if let Statement::Let {
            value: Some(expr), ..
        } = &recipe.body[2].value
        {
            assert!(matches!(expr.value, Expression::FunctionCall { .. }));
        }

        // Check interpolation
        if let Statement::Let {
            value: Some(expr), ..
        } = &recipe.body[3].value
        {
            assert!(matches!(expr.value, Expression::Interpolation(_)));
        }

        // Check array
        if let Statement::Let {
            value: Some(expr), ..
        } = &recipe.body[4].value
        {
            assert!(matches!(expr.value, Expression::Array(_)));
        }
    }

    #[test]
    fn test_shell_configuration() {
        let input = r#"
        shell "bash -c"
        
        recipe "test" {
            run "echo hello"
        }
        "#;

        let config = parse_input(input).unwrap();
        assert_eq!(config.shell, Some("bash -c".to_string()));
    }

    #[test]
    fn test_recipe_calls() {
        let input = r#"
        recipe "main" {
            call @other(param1: "value", param2: 42)
            call @other()(param1: "value2", param2: 69)
        }
        "#;

        let config = parse_input(input).unwrap();
        let recipe = &config.recipes[0].value;

        match &recipe.body[0].value {
            Statement::Call {
                recipe,
                args: call_args,
            } => {
                assert_eq!(call_args.len(), 0);
                if let Expression::RecipeRef { recipe, args } = &recipe.value {
                    assert_eq!(recipe, "other");
                    assert_eq!(args.len(), 2);
                    assert_eq!(
                        args.get("param1").unwrap().value,
                        Expression::String("value".to_string())
                    );
                    assert_eq!(args.get("param2").unwrap().value, Expression::Number(42.0));
                } else {
                    panic!("Expected recipe reference expression");
                }
            }
            _ => panic!("Expected call statement"),
        }

        match &recipe.body[1].value {
            Statement::Call {
                recipe,
                args: call_args,
            } => {
                assert_eq!(call_args.len(), 2);
                assert_eq!(
                    call_args.get("param1").unwrap().value,
                    Expression::String("value2".to_string())
                );
                assert_eq!(
                    call_args.get("param2").unwrap().value,
                    Expression::Number(69.0)
                );
                if let Expression::RecipeRef { recipe, args } = &recipe.value {
                    assert_eq!(recipe, "other");
                    assert_eq!(args.len(), 0);
                } else {
                    panic!("Expected recipe reference expression");
                }
            }
            _ => panic!("Expected call statement"),
        }
    }

    #[test]
    fn test_parse_error() {
        let input = r#"
        recipe "test" {
            invalid_statement
        }
        "#;

        let result = parse_input(input);
        assert!(result.is_err());
    }

    #[test]
    fn test_match_expression() {
        let input = r#"
        recipe "test" {
            let result: string = match value {
                "a" => "first",
                "b" => "second",
                _ => "default"
            }
        }
        "#;

        let config = parse_input(input).unwrap();
        let recipe = &config.recipes[0].value;

        if let Statement::Let {
            value: Some(expr), ..
        } = &recipe.body[0].value
        {
            if let Expression::Match {
                expr: match_expr,
                arms,
            } = &expr.value
            {
                assert!(matches!(match_expr.value, Expression::Variable(_)));
                assert_eq!(arms.len(), 3);
            } else {
                panic!("Expected match expression");
            }
        } else {
            panic!("Expected let statement with match expression");
        }
    }
}

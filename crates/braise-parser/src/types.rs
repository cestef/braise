use ::lexer::Token;
use core::{BraiseType, ast::*, error::parser::Result};

use crate::Parser;

impl<'input> Parser<'input> {
    /// Parse a parameter type with support for new type system features
    pub fn parse_param_type(&mut self) -> Result<BraiseType> {
        let base_type = self.parse_base_type()?;

        if self.match_token(&Token::QuestionMark) {
            return Ok(BraiseType::Optional(Box::new(base_type)));
        }

        if self.peek() == &Token::Pipe {
            let mut union_types = vec![base_type];

            while self.match_token(&Token::Pipe) {
                union_types.push(self.parse_base_type()?);

                if self.match_token(&Token::QuestionMark) {
                    return Ok(BraiseType::Optional(Box::new(BraiseType::Union(
                        union_types,
                    ))));
                }
            }

            return Ok(BraiseType::Union(union_types));
        }

        Ok(base_type)
    }

    /// Parse a base type (without optional or union modifiers)
    fn parse_base_type(&mut self) -> Result<BraiseType> {
        match self.peek() {
            Token::StringType => {
                self.advance();
                Ok(BraiseType::String)
            }
            Token::NumberType => {
                self.advance();
                Ok(BraiseType::Number)
            }
            Token::BoolType => {
                self.advance();
                Ok(BraiseType::Bool)
            }
            Token::ArrayType => {
                self.advance();
                self.consume_token(Token::LeftBracket)?;
                let element_type = self.parse_param_type()?;
                self.consume_token(Token::RightBracket)?;
                Ok(BraiseType::Array(Box::new(element_type)))
            }

            Token::LeftBracket => {
                self.advance();

                if self.check_string() {
                    let mut values = Vec::new();

                    loop {
                        values.push(self.parse_string()?);
                        if !self.match_token(&Token::Comma) {
                            break;
                        }
                    }

                    self.consume_token(Token::RightBracket)?;
                    Ok(BraiseType::Enum(values))
                } else {
                    let element_type = self.parse_param_type()?;
                    self.consume_token(Token::RightBracket)?;
                    Ok(BraiseType::Array(Box::new(element_type)))
                }
            }

            Token::Recipe => {
                self.advance();
                Ok(BraiseType::Recipe)
            }

            Token::Identifier(name) if name == "any" => {
                self.advance();
                Ok(BraiseType::Any)
            }

            Token::LeftParen => {
                self.advance();
                let inner_type = self.parse_param_type()?;
                self.consume_token(Token::RightParen)?;
                Ok(inner_type)
            }

            e => Err(self
                .create_error("parameter type".to_string(), e.clone())
                .boxed()),
        }
    }

    /// Parse a parameter with enhanced type support
    pub fn parse_parameter(&mut self) -> Result<SpannedNode<Parameter>> {
        let start_token = self.current;

        self.consume_token(Token::Param)?;

        let name = self.parse_identifier()?;

        let param_type = if self.match_token(&Token::Colon) {
            self.parse_param_type()?
        } else {
            BraiseType::Any
        };

        let mut is_optional = matches!(param_type, BraiseType::Optional(_));

        let default = if self.match_token(&Token::Equals) {
            is_optional = true;
            Some(self.parse_expression()?)
        } else {
            None
        };

        let end_token = self.current;
        let span = self.span_from_token_range(start_token, end_token);

        let effective_type = if is_optional && !matches!(param_type, BraiseType::Optional(_)) {
            BraiseType::Optional(Box::new(param_type))
        } else {
            param_type
        };

        let parameter = Parameter {
            name,
            param_type: effective_type,
            default,
            optional: is_optional,
        };

        Ok(SpannedNode::new(parameter, span))
    }

    /// Parse type annotation for let statements
    pub fn parse_type_annotation(&mut self) -> Result<Option<BraiseType>> {
        if self.match_token(&Token::Colon) {
            Ok(Some(self.parse_param_type()?))
        } else {
            Ok(None)
        }
    }

    /// Try to infer type from expression (for type inference)
    pub fn infer_expression_type(&mut self, expr: &Expression) -> BraiseType {
        if !self.enable_type_checking {
            return BraiseType::Any;
        }

        match expr {
            Expression::String(_) => BraiseType::String,
            Expression::Number(_) => BraiseType::Number,
            Expression::Bool(_) => BraiseType::Bool,
            Expression::Array(elements) => {
                if elements.is_empty() {
                    BraiseType::Array(Box::new(BraiseType::Any))
                } else {
                    let first_type = self.infer_expression_type(&elements[0].value);
                    BraiseType::Array(Box::new(first_type))
                }
            }
            Expression::Variable(name) => self
                .type_engine
                .get_variable_type(name)
                .cloned()
                .unwrap_or(BraiseType::Any),
            Expression::FunctionCall {
                return_type,
                module,
                function,
                ..
            } => return_type
                .clone()
                .unwrap_or_else(|| self.get_builtin_function_type(module, function)),
            Expression::ModuleAccess {
                field_type,
                module,
                field,
            } => field_type
                .clone()
                .unwrap_or_else(|| self.get_builtin_field_type(module, field)),
            Expression::BinaryOp { result_type, .. } => {
                result_type.clone().unwrap_or(BraiseType::Any)
            }
            Expression::UnaryOp { result_type, .. } => {
                result_type.clone().unwrap_or(BraiseType::Any)
            }
            Expression::Conditional { result_type, .. } => {
                result_type.clone().unwrap_or(BraiseType::Any)
            }
            Expression::Match { result_type, .. } => result_type.clone().unwrap_or(BraiseType::Any),
            Expression::Interpolation(_) => BraiseType::String,
            Expression::RecipeRef { .. } => BraiseType::Recipe,
        }
    }

    /// Validate parameter default value against parameter type
    pub fn validate_parameter_default(&mut self, param: &Parameter) -> Result<()> {
        if !self.enable_type_checking {
            return Ok(());
        }

        if let Some(ref default_expr) = param.default {
            let default_type = self.infer_expression_type(&default_expr.value);

            let expected_type = match &param.param_type {
                BraiseType::Optional(inner) => inner.as_ref(),
                other => other,
            };

            if !default_type.is_compatible_with(expected_type) {
                return Err(self
                    .create_type_error(
                        expected_type.to_string(),
                        default_type.to_string(),
                        format!("default value for parameter '{}'", param.name),
                        &default_expr.span,
                    )
                    .boxed());
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::assert_matches::assert_matches;

    use super::*;
    use lexer::tokenize;

    fn parse_type_string(input: &str) -> Result<BraiseType> {
        let tokens = tokenize(input).unwrap();
        let mut parser = Parser::new(&tokens, input, "test.braise".to_string());
        parser.parse_param_type()
    }

    #[test]
    fn test_basic_types() {
        assert_eq!(parse_type_string("string").unwrap(), BraiseType::String);
        assert_eq!(parse_type_string("number").unwrap(), BraiseType::Number);
        assert_eq!(parse_type_string("bool").unwrap(), BraiseType::Bool);
        assert_eq!(parse_type_string("any").unwrap(), BraiseType::Any);
    }

    #[test]
    fn test_array_types() {
        let array_type = parse_type_string("[string]").unwrap();
        assert_eq!(array_type, BraiseType::Array(Box::new(BraiseType::String)));

        let nested_array = parse_type_string("[[number]]").unwrap();
        assert_eq!(
            nested_array,
            BraiseType::Array(Box::new(BraiseType::Array(Box::new(BraiseType::Number))))
        );
    }

    #[test]
    fn test_enum_types() {
        let enum_type = parse_type_string(r#"["dev", "prod", "staging"]"#).unwrap();
        if let BraiseType::Enum(variants) = enum_type {
            assert_eq!(variants, vec!["dev", "prod", "staging"]);
        } else {
            panic!("Expected enum type");
        }
    }

    #[test]
    fn test_union_types() {
        let union_type = parse_type_string("string | number").unwrap();
        if let BraiseType::Union(types) = union_type {
            assert_eq!(types.len(), 2);
            assert!(types.contains(&BraiseType::String));
            assert!(types.contains(&BraiseType::Number));
        } else {
            panic!("Expected union type");
        }

        let complex_union = parse_type_string("string | number | bool").unwrap();
        if let BraiseType::Union(types) = complex_union {
            assert_eq!(types.len(), 3);
            assert!(types.contains(&BraiseType::String));
            assert!(types.contains(&BraiseType::Number));
            assert!(types.contains(&BraiseType::Bool));
        } else {
            panic!("Expected union type");
        }
    }

    #[test]
    fn test_optional_types() {
        let optional_string = parse_type_string("string?").unwrap();
        if let BraiseType::Optional(inner) = optional_string {
            assert_eq!(*inner, BraiseType::String);
        } else {
            panic!("Expected optional type");
        }

        let optional_union = parse_type_string("(string | number)?").unwrap();
        if let BraiseType::Optional(inner) = optional_union {
            if let BraiseType::Union(types) = inner.as_ref() {
                assert_eq!(types.len(), 2);
                assert!(types.contains(&BraiseType::String));
                assert!(types.contains(&BraiseType::Number));
            } else {
                panic!("Expected union type inside optional");
            }
        } else {
            panic!("Expected optional type");
        }
    }

    #[test]
    fn test_complex_types() {
        let complex_type = parse_type_string("[string | number]").unwrap();
        if let BraiseType::Array(element_type) = complex_type {
            if let BraiseType::Union(types) = element_type.as_ref() {
                assert_eq!(types.len(), 2);
                assert!(types.contains(&BraiseType::String));
                assert!(types.contains(&BraiseType::Number));
            } else {
                panic!("Expected union type as array element");
            }
        } else {
            panic!("Expected array type");
        }

        let optional_array = parse_type_string("[string]?").unwrap();
        if let BraiseType::Optional(inner) = optional_array {
            if let BraiseType::Array(element_type) = inner.as_ref() {
                assert_eq!(**element_type, BraiseType::String);
            } else {
                panic!("Expected array type inside optional");
            }
        } else {
            panic!("Expected optional type");
        }
    }

    #[test]
    fn test_recipe_with_enhanced_types() -> miette::Result<()> {
        let input = r#"
        recipe "test" {
            param name: string | number = "default"
            param count: number?
            param items: [string] = ["a", "b"]
            param env: ["dev", "prod"] = "dev"
            
            let value: string = match name {
                n if n > 10 => "large",
                _ => "small"
            }
        }
        "#;

        let tokens = tokenize(input)?;
        let mut parser = Parser::new(&tokens, input, "test.braise".to_string());
        let config = parser.parse();

        assert!(config.is_ok(), "Parser should handle enhanced types");
        let config = config.unwrap();
        assert_eq!(config.recipes.len(), 1);

        let recipe = &config.recipes[0].value;
        assert_eq!(recipe.parameters.len(), 4);

        let name_param = &recipe.parameters[0].value;
        if let BraiseType::Optional(inner) = &name_param.param_type {
            if let BraiseType::Union(types) = inner.as_ref() {
                assert_eq!(types.len(), 2);
                assert!(types.contains(&BraiseType::String));
                assert!(types.contains(&BraiseType::Number));
            } else {
                panic!("Expected union type inside optional");
            }
        } else {
            panic!("Expected union type for name parameter");
        }

        let count_param = &recipe.parameters[1].value;
        assert!(matches!(count_param.param_type, BraiseType::Optional(_)));
        assert!(count_param.optional);

        let items_param = &recipe.parameters[2].value;
        assert_matches!(
            &items_param.param_type,
            BraiseType::Optional(inner) if matches!(**inner, BraiseType::Array(_))
        );

        let env_param = &recipe.parameters[3].value;
        if let BraiseType::Optional(inner) = &env_param.param_type {
            if let BraiseType::Enum(variants) = inner.as_ref() {
                assert_eq!(variants, &vec!["dev".to_string(), "prod".to_string()]);
            } else {
                panic!("Expected enum type inside optional");
            }
        } else {
            panic!("Expected enum type for env parameter");
        }
        Ok(())
    }
}

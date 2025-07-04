use ::lexer::Token;
use core::{error::parser::Result, *};

use crate::Parser;

impl Parser {
    pub fn parse_param_type(&mut self) -> Result<ParamType> {
        match self.peek() {
            // Handle type keywords
            Token::StringType => {
                self.advance();
                Ok(ParamType::String)
            }
            Token::NumberType | Token::IntType => {
                self.advance();
                Ok(ParamType::Number)
            }
            Token::BoolType => {
                self.advance();
                Ok(ParamType::Bool)
            }
            Token::ArrayType => {
                self.advance();
                self.consume_token(Token::LeftBracket)?;
                let element_type = self.parse_param_type()?;
                self.consume_token(Token::RightBracket)?;
                Ok(ParamType::Array(Box::new(element_type)))
            }

            Token::LeftBracket => {
                self.advance();

                // Check if this is an enum [value1, value2, ...] or array [element_type]
                if self.check_string() {
                    // Enum type: [value1, value2, ...]
                    let mut values = Vec::new();

                    loop {
                        values.push(self.parse_string()?);
                        if !self.match_token(&Token::Comma) {
                            break;
                        }
                    }

                    self.consume_token(Token::RightBracket)?;
                    Ok(ParamType::Enum(values))
                } else {
                    // Array type: [element_type]
                    let element_type = self.parse_param_type()?;
                    self.consume_token(Token::RightBracket)?;
                    Ok(ParamType::Array(Box::new(element_type)))
                }
            }

            Token::Recipe => {
                self.advance();
                Ok(ParamType::Recipe)
            }

            e => Err(self.create_error("parameter type".to_string(), e.clone())),
        }
    }

    pub fn parse_parameter(&mut self) -> Result<SpannedNode<Parameter>> {
        let start_token = self.current;

        self.consume_token(Token::Param)?;

        let name = self.parse_identifier()?;
        self.consume_token(Token::Colon)?;

        let param_type = self.parse_param_type()?;

        let default = if self.match_token(&Token::Equals) {
            Some(self.parse_expression()?)
        } else {
            None
        };

        let end_token = self.current;
        let span = self.span_from_token_range(start_token, end_token);

        let parameter = Parameter {
            name,
            param_type,
            default,
        };

        Ok(SpannedNode::new(parameter, span))
    }
}

use ::lexer::{SpannedToken, Token};
use core::{error::parser::Result, *};
use std::rc::Rc;

mod expressions;
mod helpers;
mod match_patterns;
mod statements;
mod types;

pub struct Parser {
    pub tokens: Vec<SpannedToken>,
    pub source: Rc<String>,
    pub current: usize,
    pub file_id: FileId,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>, source: String, filename: String) -> Self {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        filename.hash(&mut hasher);
        let file_id = FileId(hasher.finish() as usize);

        Parser {
            tokens,
            current: 0,
            source: Rc::new(source),
            file_id,
        }
    }

    pub fn parse(&mut self) -> Result<Config> {
        let mut recipes = Vec::new();
        let mut shell = None;
        let start_token = self.current;

        while !self.is_at_end() {
            if self.match_token(&Token::Shell) {
                if shell.is_some() {
                    return Err(self.create_error(
                        "only one shell command is allowed".to_string(),
                        Token::Shell,
                    ));
                }
                let s = self.parse_string()?;
                shell = Some(s);
                continue;
            }
            recipes.push(self.parse_recipe()?);
        }

        let end_token = self.current;
        let span = self.span_from_token_range(start_token, end_token.max(1));

        Ok(Config {
            recipes,
            span,
            shell,
        })
    }

    fn parse_recipe(&mut self) -> Result<SpannedNode<Recipe>> {
        let start_token = self.current;
        self.consume_token(Token::Recipe)?;

        let name = self.parse_string()?;

        let dependencies = if self.match_token(&Token::Arrow) {
            self.consume_token(Token::LeftBracket)?;
            let mut deps = Vec::new();

            if !self.check(&Token::RightBracket) {
                loop {
                    deps.push(self.parse_string()?);
                    if !self.match_token(&Token::Comma) {
                        break;
                    }
                }
            }

            self.consume_token(Token::RightBracket)?;
            deps
        } else {
            Vec::new()
        };

        self.consume_token(Token::LeftBrace)?;

        let mut parameters = Vec::new();
        let mut body = Vec::new();

        while !self.check(&Token::RightBrace) && !self.is_at_end() {
            if self.check(&Token::Param) {
                parameters.push(self.parse_parameter()?);
            } else {
                body.push(self.parse_statement()?);
            }
        }

        self.consume_token(Token::RightBrace)?;
        let end_token = self.current;

        let recipe = Recipe {
            name,
            dependencies,
            parameters,
            body,
        };

        let span = self.span_from_token_range(start_token, end_token);
        Ok(SpannedNode::new(recipe, span))
    }
}

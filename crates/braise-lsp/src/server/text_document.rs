use crate::{
    server::diagnostics::DiagnosticsProvider,
    utils::{get_expression_preview, span_to_range},
};

use super::Document;
use braise_core::ast::*;
use lexer::Token;
use tower_lsp::lsp_types::*;

pub struct TextDocumentProvider;

impl TextDocumentProvider {
    pub fn new() -> Self {
        Self
    }

    pub async fn provide_hover(&self, doc: &Document, position: Position) -> Option<Hover> {
        let text = doc.get_text();
        let lines: Vec<&str> = text.lines().collect();

        if let Some(line) = lines.get(position.line as usize) {
            let char_pos = position.character as usize;
            if char_pos >= line.len() {
                return None;
            }

            let word = self.get_word_at_position(line, char_pos)?;

            if let Some(hover_content) = self.get_builtin_hover(&word) {
                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: hover_content,
                    }),
                    range: Some(self.get_word_range(line, char_pos, position.line)),
                });
            }

            if let Some(ref ast) = doc.ast
                && let Some(current_recipe) = Self::find_recipe_at_position(ast, position)
            {
                for param in &current_recipe.value.parameters {
                    if param.value.name == word {
                        let content = format!(
                            "**Parameter**: `{}`\n\n**Type**: `{}`{}",
                            param.value.name,
                            param.value.param_type,
                            if let Some(ref default) = param.value.default {
                                format!(
                                    "\n\n**Default**: `{}`",
                                    get_expression_preview(&default.value)
                                )
                            } else {
                                String::new()
                            }
                        );

                        return Some(Hover {
                            contents: HoverContents::Markup(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: content,
                            }),
                            range: Some(self.get_word_range(line, char_pos, position.line)),
                        });
                    }
                }

                if let Some(param_type) =
                    DiagnosticsProvider::get_variable_type(&word, &current_recipe.value)
                {
                    // TODO: at the moment we are getting variables from the whole recipe,
                    // whereas we should be getting it from the current scope
                    let content = format!("**Variable**: `{word}`\n\n**Type**: `{param_type}`");

                    return Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: content,
                        }),
                        range: Some(self.get_word_range(line, char_pos, position.line)),
                    });
                }

                for recipe in &ast.recipes {
                    if recipe.value.name == word {
                        let content = format!(
                            "**Recipe**: `{}`\n\n**Parameters**:\n{}",
                            recipe.value.name,
                            recipe
                                .value
                                .parameters
                                .iter()
                                .map(|param| {
                                    format!(
                                        "- `{}`: `{}`{}",
                                        param.value.name,
                                        param.value.param_type,
                                        if let Some(ref default) = param.value.default {
                                            format!(
                                                " (default: `{}`)",
                                                get_expression_preview(&default.value)
                                            )
                                        } else {
                                            String::new()
                                        }
                                    )
                                })
                                .collect::<Vec<_>>()
                                .join("\n")
                        );
                        return Some(Hover {
                            contents: HoverContents::Markup(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: content,
                            }),
                            range: Some(self.get_word_range(line, char_pos, position.line)),
                        });
                    }
                }
            }
        }

        None
    }

    pub async fn provide_definition(&self, doc: &Document, position: Position) -> Option<Location> {
        let text = doc.get_text();
        let lines: Vec<&str> = text.lines().collect();

        if let Some(line) = lines.get(position.line as usize) {
            let char_pos = position.character as usize;
            let word = self.get_word_at_position(line, char_pos)?;

            if let Some(ref ast) = doc.ast
                && let Some(current_recipe) = Self::find_recipe_at_position(ast, position)
            {
                for param in &current_recipe.value.parameters {
                    if param.value.name == word {
                        return Some(Location {
                            uri: doc.uri.clone(),
                            range: span_to_range(&param.span),
                        });
                    }
                }
            }
        }

        None
    }

    pub async fn provide_code_actions(&self, _doc: &Document, _range: Range) -> CodeActionResponse {
        CodeActionResponse::from(Vec::new())
    }

    pub async fn provide_formatting(&self, doc: &Document) -> Vec<TextEdit> {
        let text = doc.get_text();
        let formatted = fmt::Formatter::format(&text);

        if formatted != text {
            vec![TextEdit {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: text.lines().count() as u32,
                        character: 0,
                    },
                },
                new_text: formatted,
            }]
        } else {
            Vec::new()
        }
    }

    pub async fn provide_folding_ranges(&self, doc: &Document) -> Vec<FoldingRange> {
        let text = doc.get_text();
        let lines: Vec<&str> = text.lines().collect();
        let mut ranges = Vec::new();
        let mut brace_stack = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            let mut in_string = false;
            let mut escape_next = false;

            for ch in line.chars() {
                if escape_next {
                    escape_next = false;
                    continue;
                }

                match ch {
                    '\\' if in_string => escape_next = true,
                    '"' => in_string = !in_string,
                    '{' if !in_string => brace_stack.push(line_num as u32),
                    '}' if !in_string => {
                        if let Some(start_line) = brace_stack.pop()
                            && line_num as u32 > start_line
                        {
                            ranges.push(FoldingRange {
                                start_line,
                                start_character: None,
                                end_line: line_num as u32,
                                end_character: None,
                                kind: Some(FoldingRangeKind::Region),
                                collapsed_text: None,
                            });
                        }
                    }
                    _ => {}
                }
            }
        }

        ranges
    }

    pub async fn provide_semantic_tokens(&self, doc: &Document) -> Option<SemanticTokens> {
        if let Some(ref tokens) = doc.tokens {
            let mut semantic_tokens = Vec::new();
            let mut prev_line = 0;
            let mut prev_character = 0;

            for token in tokens {
                let line = token.span.start.line.saturating_sub(1);
                let character = token.span.start.column.saturating_sub(1);
                let length = token
                    .span
                    .end
                    .offset
                    .saturating_sub(token.span.start.offset) as u32;

                let (token_type, token_modifiers) = self.get_semantic_token_info(&token.token);

                let delta_line = line - prev_line;
                let delta_character = if delta_line == 0 {
                    character - prev_character
                } else {
                    character
                };

                semantic_tokens.extend_from_slice(&[
                    delta_line,
                    delta_character,
                    length,
                    token_type,
                    token_modifiers,
                ]);

                prev_line = line;
                prev_character = character;
            }

            Some(SemanticTokens {
                result_id: None,
                data: semantic_tokens
                    .iter()
                    .map(|&x| SemanticToken {
                        delta_line: x,
                        delta_start: 0,
                        length: 0,
                        token_type: x,
                        token_modifiers_bitset: 0,
                    })
                    .collect(),
            })
        } else {
            None
        }
    }

    fn get_word_at_position(&self, line: &str, char_pos: usize) -> Option<String> {
        if char_pos >= line.len() {
            return None;
        }

        let chars: Vec<char> = line.chars().collect();

        let mut in_string = false;
        let mut in_interpolation = false;
        let mut escape_next = false;

        for i in 0..char_pos {
            if escape_next {
                escape_next = false;
                continue;
            }

            if chars[i] == '\\' && in_string {
                escape_next = true;
            } else if chars[i] == '"' && !in_interpolation {
                in_string = !in_string;
            } else if in_string && chars[i] == '$' && i + 1 < chars.len() && chars[i + 1] == '{' {
                in_interpolation = true;
            } else if in_interpolation && chars[i] == '}' {
                in_interpolation = false;
            }
        }

        if in_string && !in_interpolation {
            return None;
        }

        let mut start = char_pos;
        let mut end = char_pos;

        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }

        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }

        if start < end {
            Some(chars[start..end].iter().collect())
        } else {
            None
        }
    }

    fn get_word_range(&self, line: &str, char_pos: usize, line_num: u32) -> Range {
        let chars: Vec<char> = line.chars().collect();
        let mut start = char_pos;
        let mut end = char_pos;

        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }

        while end < chars.len() && (chars[end].is_alphanumeric() || chars[end] == '_') {
            end += 1;
        }

        Range {
            start: Position {
                line: line_num,
                character: start as u32,
            },
            end: Position {
                line: line_num,
                character: end as u32,
            },
        }
    }

    fn get_builtin_hover(&self, word: &str) -> Option<String> {
        match word {
            "env" => Some(include_str!("../../docs/builtins/env.md").to_string()),
            "cpu" => Some(include_str!("../../docs/builtins/cpu.md").to_string()),
            "git" => Some(include_str!("../../docs/builtins/git.md").to_string()),
            "fs" => Some(include_str!("../../docs/builtins/fs.md").to_string()),
            "input" => Some(include_str!("../../docs/builtins/input.md").to_string()),
            "recipe" => Some(include_str!("../../docs/keywords/recipe.md").to_string()),
            "param" => Some(include_str!("../../docs/keywords/param.md").to_string()),
            "run" => Some(include_str!("../../docs/keywords/run.md").to_string()),
            "print" => Some(include_str!("../../docs/keywords/print.md").to_string()),
            "if" => Some(include_str!("../../docs/keywords/if.md").to_string()),
            "match" => Some(include_str!("../../docs/keywords/match.md").to_string()),
            "for" => Some(include_str!("../../docs/keywords/for.md").to_string()),
            "let" => Some(include_str!("../../docs/keywords/let.md").to_string()),
            "shell" => Some(include_str!("../../docs/keywords/shell.md").to_string()),
            "try" => Some(include_str!("../../docs/keywords/try.md").to_string()),
            _ => None,
        }
    }

    fn get_semantic_token_info(&self, token: &Token) -> (u32, u32) {
        let token_type = match token {
            Token::Recipe
            | Token::Param
            | Token::Run
            | Token::If
            | Token::Else
            | Token::Match
            | Token::For
            | Token::In
            | Token::Async
            | Token::Exit
            | Token::Print
            | Token::Let => 0, // KEYWORD
            Token::String(_) => 1, // STRING
            Token::Number(_) => 2, // NUMBER
            Token::Identifier(_) => {
                3 // FUNCTION (default)
            }
            Token::Bool(_) => 2, // NUMBER (treating bool as number type)
            Token::StringType
            | Token::NumberType
            | Token::IntType
            | Token::BoolType
            | Token::ArrayType => 6, // TYPE
            _ => 4,              // VARIABLE (default)
        };

        (token_type, 0) // No modifiers for now
    }

    /// Find which recipe contains the given position
    pub fn find_recipe_at_position(
        ast: &Config,
        position: Position,
    ) -> Option<&SpannedNode<Recipe>> {
        for recipe in &ast.recipes {
            let recipe_start = Position {
                line: recipe.span.start.line.saturating_sub(1),
                character: recipe.span.start.column.saturating_sub(1),
            };
            let recipe_end = Position {
                line: recipe.span.end.line.saturating_sub(1),
                character: recipe.span.end.column.saturating_sub(1),
            };

            if Self::is_position_in_range(position, recipe_start, recipe_end) {
                return Some(recipe);
            }
        }
        None
    }

    /// Check if a position is within a range (inclusive)
    pub fn is_position_in_range(pos: Position, start: Position, end: Position) -> bool {
        if pos.line < start.line || pos.line > end.line {
            return false;
        }

        if pos.line == start.line && pos.character < start.character {
            return false;
        }

        if pos.line == end.line && pos.character > end.character {
            return false;
        }

        true
    }
}

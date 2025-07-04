use crate::server::diagnostics::DiagnosticsProvider;

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

            // Find word at position
            let word = self.get_word_at_position(line, char_pos)?;

            // Check if it's a built-in module or function
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
                // Check if it's a parameter in the current recipe
                for param in &current_recipe.value.parameters {
                    if param.value.name == word {
                        let content = format!(
                            "**Parameter**: `{}`\n\n**Type**: `{}`{}",
                            param.value.name,
                            param.value.param_type,
                            if let Some(ref default) = param.value.default {
                                format!(
                                    "\n\n**Default**: `{}`",
                                    Self::expression_to_string(&default.value)
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

                // Check if it's a variable in the current recipe
                if let Some(param_type) =
                    DiagnosticsProvider::get_variable_type(&word, &current_recipe.value)
                {
                    let content = format!("**Variable**: `{word}`\n\n**Type**: `{param_type}`");

                    return Some(Hover {
                        contents: HoverContents::Markup(MarkupContent {
                            kind: MarkupKind::Markdown,
                            value: content,
                        }),
                        range: Some(self.get_word_range(line, char_pos, position.line)),
                    });
                }

                // Check if it's a recipe call
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
                                                Self::expression_to_string(&default.value)
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

            // Look for parameter definitions in the current recipe only
            if let Some(ref ast) = doc.ast
                && let Some(current_recipe) = Self::find_recipe_at_position(ast, position)
            {
                for param in &current_recipe.value.parameters {
                    if param.value.name == word {
                        return Some(Location {
                            uri: doc.uri.clone(),
                            range: self.span_to_range(&param.span),
                        });
                    }
                }
            }
        }

        None
    }

    pub async fn provide_code_actions(&self, doc: &Document, range: Range) -> CodeActionResponse {
        let mut actions = Vec::new();

        if let Some(action) = self.create_add_parameter_action(doc, range) {
            actions.push(action);
        }

        CodeActionResponse::from(actions)
    }

    pub async fn provide_formatting(&self, doc: &Document) -> Vec<TextEdit> {
        let text = doc.get_text();
        let formatted = self.format_braise_code(&text);

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
                        delta_start: 0, // No delta start for now
                        length: 0,      // Length is handled in the next token
                        token_type: x,
                        token_modifiers_bitset: 0, // No modifiers for now
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
        // Check if we're inside a string
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

        // If inside a string but not in interpolation, don't provide hover
        if in_string && !in_interpolation {
            return None;
        }

        let mut start = char_pos;
        let mut end = char_pos;

        // Find start of word
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }

        // Find end of word
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

        // Find start of word
        while start > 0 && (chars[start - 1].is_alphanumeric() || chars[start - 1] == '_') {
            start -= 1;
        }

        // Find end of word
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
            "env" => Some("**Environment Module**\n\nProvides access to environment variables and system information.\n\n**Functions:**\n- `get(name)` - Get environment variable\n- `has(name)` - Check if environment variable exists\n\n**Fields:**\n- `HOME` - Home directory\n- `PWD` - Current working directory\n- `CI` - Whether running in CI".to_string()),
            "cpu" => Some("**CPU Module**\n\nProvides CPU and system information.\n\n**Functions:**\n- `count()` - Get number of CPU cores\n- `physical_count()` - Get number of physical CPU cores\n\n**Fields:**\n- `arch` - CPU architecture".to_string()),
            "git" => Some("**Git Module**\n\nProvides Git repository information.\n\n**Functions:**\n- `branch()` - Get current branch\n- `commit_hash()` - Get current commit hash\n- `commit_hash_short()` - Get short commit hash\n- `is_clean()` - Check if working directory is clean\n- `is_dirty()` - Check if working directory is dirty\n- `tag()` - Get current tag".to_string()),
            "fs" => Some("**File System Module**\n\nProvides file system utilities.\n\n**Functions:**\n- `exists(path)` - Check if path exists\n- `is_file(path)` - Check if path is a file\n- `is_dir(path)` - Check if path is a directory".to_string()),
            "recipe" => Some("**recipe** keyword\n\nDefines a new recipe.\n\n**Syntax:**\n```braise\nrecipe \"name\" {\n    // recipe body\n}\n```\n\n**With dependencies:**\n```braise\nrecipe \"name\" -> [\"dep1\", \"dep2\"] {\n    // recipe body\n}\n```".to_string()),
            "param" => Some("**param** keyword\n\nDefines a parameter for a recipe.\n\n**Syntax:**\n```braise\nparam name: type = default\n```\n\n**Types:** `string`, `number`, `bool`, `[type]` (array), `[\"opt1\", \"opt2\"]` (enum)".to_string()),
            "run" => Some("**run** statement\n\nExecutes a shell command.\n\n**Syntax:**\n```braise\nrun \"command\"\nrun \"echo ${variable}\"\n```".to_string()),
            "print" => Some("**print** statement\n\nPrints a message.\n\n**Syntax:**\n```braise\nprint \"message\"\nprint \"Hello ${name}\"\n```".to_string()),
            "if" => Some("**if** statement\n\nConditional execution.\n\n**Syntax:**\n```braise\nif condition {\n    // statements\n} else {\n    // statements\n}\n```".to_string()),
            "match" => Some("**match** statement\n\nPattern matching.\n\n**Syntax:**\n```braise\nmatch expr {\n    \"pattern1\" => { /* statements */ },\n    \"pattern2\" => { /* statements */ },\n    _ => { /* default */ }\n}\n```".to_string()),
            "for" => Some("**for** statement\n\nLoop over an array.\n\n**Syntax:**\n```braise\nfor item in items {\n    // statements\n}\n\n// Parallel execution\nfor item in items {\n    // statements\n} async\n```".to_string()),
            "let" => Some("**let** keyword\n\nDefines a variable.\n\n**Syntax:**\n```braise\nlet name: type = value\n```\n\n**Types:** `string`, `number`, `bool`, `[type]` (array), `[\"opt1\", \"opt2\"]` (enum)".to_string()),
            "shell" => Some("**shell** keyword\n\nSets the shell to use for commands.\n\n**Syntax:**\n```braise\nshell \"bash\"\n```".to_string()),
            _ => None,
        }
    }

    fn expression_to_string(expr: &Expression) -> String {
        match expr {
            Expression::String(s) => format!("\"{s}\""),
            Expression::Number(n) => n.to_string(),
            Expression::Bool(b) => b.to_string(),
            Expression::Variable(name) => name.clone(),
            Expression::FunctionCall {
                module, function, ..
            } => {
                format!("{module}.{function}(...)")
            }
            Expression::ModuleAccess { module, field } => {
                format!("{module}.{field}")
            }
            Expression::Array(elements) => {
                let elements_str: Vec<String> = elements
                    .iter()
                    .take(3)
                    .map(|elem| Self::expression_to_string(&elem.value))
                    .collect();

                if elements.len() > 3 {
                    format!("[{}, ...]", elements_str.join(", "))
                } else {
                    format!("[{}]", elements_str.join(", "))
                }
            }
            _ => "...".to_string(),
        }
    }

    fn span_to_range(&self, span: &braise_core::Span) -> Range {
        Range {
            start: Position {
                line: span.start.line.saturating_sub(1),
                character: span.start.column.saturating_sub(1),
            },
            end: Position {
                line: span.end.line.saturating_sub(1),
                character: span.end.column.saturating_sub(1),
            },
        }
    }

    fn create_add_parameter_action(
        &self,
        doc: &Document,
        range: Range,
    ) -> Option<CodeActionOrCommand> {
        Some(CodeActionOrCommand::CodeAction(CodeAction {
            title: "Add parameter".to_string(),
            kind: Some(CodeActionKind::REFACTOR),
            diagnostics: None,
            edit: Some(WorkspaceEdit {
                changes: None,
                document_changes: Some(DocumentChanges::Edits(vec![TextDocumentEdit {
                    text_document: OptionalVersionedTextDocumentIdentifier {
                        uri: doc.uri.clone(),
                        version: Some(doc.version),
                    },
                    edits: vec![OneOf::Left(TextEdit {
                        range: Range {
                            start: Position {
                                line: range.start.line + 1,
                                character: 4,
                            },
                            end: Position {
                                line: range.start.line + 1,
                                character: 4,
                            },
                        },
                        new_text: "param name: string = \"default\"\n    ".to_string(),
                    })],
                }])),
                change_annotations: None,
            }),
            command: None,
            is_preferred: Some(false),
            disabled: None,
            data: None,
        }))
    }

    fn format_braise_code(&self, text: &str) -> String {
        let lines: Vec<&str> = text.lines().collect();
        let mut formatted_lines = Vec::new();
        let mut indent_level: usize = 0;
        let indent_size = 4;

        // First pass: format indentation and clean spaces
        let mut pre_aligned_lines = Vec::new();
        for line in lines {
            let trimmed = line.trim();

            if trimmed.is_empty() {
                pre_aligned_lines.push((String::new(), None));
                continue;
            }

            // closing
            if trimmed.starts_with('}') {
                indent_level = indent_level.saturating_sub(1);
            }

            // del. dup spaces
            let cleaned_line = self.clean_duplicate_spaces(trimmed);

            // Split line into code and comment
            let (code_part, comment_part) = self.split_code_and_comment(&cleaned_line);

            let indented_line = format!("{}{}", " ".repeat(indent_level * indent_size), code_part);

            pre_aligned_lines.push((indented_line, comment_part));

            // opening
            if trimmed.ends_with('{') {
                indent_level += 1;
            }
        }

        // Second pass: align comments
        let mut i = 0;
        while i < pre_aligned_lines.len() {
            let mut comment_group = Vec::new();
            let mut j = i;

            // Find consecutive lines with comments
            while j < pre_aligned_lines.len() && pre_aligned_lines[j].1.is_some() {
                comment_group.push(j);
                j += 1;
            }

            // Align comments if we have at least 2 consecutive commented lines
            if comment_group.len() >= 2 {
                let max_code_len = comment_group
                    .iter()
                    .map(|&idx| pre_aligned_lines[idx].0.len())
                    .max()
                    .unwrap_or(0);

                // Add 2 spaces between code and comment
                let comment_start = max_code_len + 2;

                for idx in comment_group {
                    let (code, comment) = &pre_aligned_lines[idx];
                    let padding = " ".repeat(comment_start.saturating_sub(code.len()));
                    formatted_lines.push(format!(
                        "{}{}{}",
                        code,
                        padding,
                        comment.as_ref().unwrap()
                    ));
                }

                i = j;
            } else {
                // Handle the current line without special alignment
                let (code, comment) = &pre_aligned_lines[i];
                if let Some(cmt) = comment {
                    formatted_lines.push(format!("{}  {}", code, cmt));
                } else {
                    formatted_lines.push(code.clone());
                }
                i += 1;
            }
        }

        formatted_lines.join("\n")
    }

    fn split_code_and_comment(&self, line: &str) -> (String, Option<String>) {
        let mut in_string = false;
        let mut escape_next = false;

        for (i, c) in line.char_indices() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match c {
                '"' => in_string = !in_string,
                '\\' if in_string => escape_next = true,
                '/' if !in_string && i + 1 < line.len() && line.chars().nth(i + 1) == Some('/') => {
                    let code = line[..i].trim_end().to_string();
                    let comment = line[i..].to_string();
                    return (code, Some(comment));
                }
                _ => {}
            }
        }

        (line.to_string(), None)
    }

    fn clean_duplicate_spaces(&self, text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();
        let mut in_string = false;
        let mut last_was_space = false;
        let mut escape_next = false;

        while let Some(c) = chars.next() {
            if escape_next {
                result.push(c);
                escape_next = false;
                last_was_space = false;
                continue;
            }

            match c {
                '"' => {
                    in_string = !in_string;
                    result.push(c);
                    last_was_space = false;
                }
                '\\' if in_string => {
                    result.push(c);
                    escape_next = true;
                    last_was_space = false;
                }
                ' ' => {
                    if in_string || !last_was_space {
                        result.push(c);
                    }
                    last_was_space = !in_string && c == ' ';
                }
                _ => {
                    result.push(c);
                    last_was_space = false;
                }
            }
        }

        result
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
                // Could be function, variable, or parameter depending on context
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

            // Check if position is within this recipe's span
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

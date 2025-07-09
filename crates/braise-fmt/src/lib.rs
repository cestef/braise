use std::borrow::Cow;

/// Configuration for the Braise code formatter
#[derive(Debug, Clone, Copy)]
pub struct FormatterConfig {
    pub indent_size: u8,
    pub max_consecutive_blank_lines: u8,
    pub align_inline_comments: bool,
    pub min_comment_spacing: u8,
}

impl Default for FormatterConfig {
    fn default() -> Self {
        Self {
            indent_size: 4,
            max_consecutive_blank_lines: 1,
            align_inline_comments: true,
            min_comment_spacing: 2,
        }
    }
}

/// Represents a parsed line with minimal allocations
#[derive(Debug)]
struct ParsedLine<'a> {
    code: Cow<'a, str>,
    comment_start: Option<usize>,
    is_full_line_comment: bool,
    indent_delta: i8, // -1 for closing brace, +1 for opening brace, 0 for no change
}

/// Main formatter for Braise code
pub struct Formatter {
    config: FormatterConfig,
}

impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter {
    #[inline]
    pub fn new() -> Self {
        Self {
            config: FormatterConfig::default(),
        }
    }

    #[inline]
    pub fn with_config(config: FormatterConfig) -> Self {
        Self { config }
    }

    /// Format with default configuration (static method)
    pub fn format(text: &str) -> String {
        Self::new().format_text(text)
    }

    /// Format with custom configuration (static method)
    pub fn format_with_config(text: &str, config: FormatterConfig) -> String {
        Self::with_config(config).format_text(text)
    }

    /// Main entry point for formatting Braise code
    pub fn format_text(&self, text: &str) -> String {
        let mut output = String::with_capacity(text.len() + (text.len() >> 3)); // +12.5% capacity

        let lines: Vec<&str> = text.lines().collect();
        if lines.is_empty() {
            return String::new();
        }

        let parsed_lines = self.parse_lines(&lines);

        let indent_levels = self.calculate_indent_levels(&parsed_lines);

        self.write_formatted_lines(&mut output, &lines, &parsed_lines, &indent_levels);

        output
    }

    /// Parse all lines efficiently
    fn parse_lines<'a>(&self, lines: &[&'a str]) -> Vec<ParsedLine<'a>> {
        lines
            .iter()
            .map(|&line| self.parse_line_fast(line))
            .collect()
    }

    /// Fast line parsing with minimal allocations
    fn parse_line_fast<'a>(&self, line: &'a str) -> ParsedLine<'a> {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            return ParsedLine {
                code: Cow::Borrowed(""),
                comment_start: None,
                is_full_line_comment: false,
                indent_delta: 0,
            };
        }

        if trimmed.starts_with("//") {
            return ParsedLine {
                code: self.clean_spaces_cow(trimmed),
                comment_start: None,
                is_full_line_comment: true,
                indent_delta: 0,
            };
        }

        let comment_pos = self.find_comment_position(line);

        let (code_part, indent_delta) = if let Some(pos) = comment_pos {
            let code = line[..pos].trim();
            (code, self.calculate_indent_delta(code))
        } else {
            (trimmed, self.calculate_indent_delta(trimmed))
        };

        ParsedLine {
            code: self.clean_spaces_cow(code_part),
            comment_start: comment_pos,
            is_full_line_comment: false,
            indent_delta,
        }
    }

    /// Calculate indent delta for a code line
    #[inline]
    fn calculate_indent_delta(&self, code: &str) -> i8 {
        let starts_with_brace = code.starts_with('}');
        let ends_with_brace = code.trim_end().ends_with('{');

        match (starts_with_brace, ends_with_brace) {
            (true, true) => 0,   // }{ - closes one, opens one
            (true, false) => -1, // } - closes one
            (false, true) => 1,  // { - opens one
            (false, false) => 0,
        }
    }

    #[inline]
    fn is_continuation_construct(&self, trimmed_line: &str) -> bool {
        const CONTINUATION_KEYWORDS: &[&str] = &["catch", "finally", "else"];

        if !trimmed_line.starts_with('}') {
            return false;
        }

        let after_brace = &trimmed_line[1..].trim_start();
        CONTINUATION_KEYWORDS.iter().any(|&keyword| {
            after_brace.starts_with(keyword)
                && (after_brace.len() == keyword.len()
                    || after_brace
                        .chars()
                        .nth(keyword.len())
                        .map_or(false, |c| !c.is_alphanumeric()))
        })
    }

    /// Clean spaces with copy-on-write optimization
    fn clean_spaces_cow<'a>(&self, text: &'a str) -> Cow<'a, str> {
        if !self.needs_space_cleaning(text) {
            return Cow::Borrowed(text);
        }

        let mut result = String::with_capacity(text.len());
        let chars = text.chars();
        let mut in_string = false;
        let mut last_was_space = false;
        let mut escape_next = false;

        for c in chars {
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
                    last_was_space = !in_string;
                }
                _ => {
                    result.push(c);
                    last_was_space = false;
                }
            }
        }

        Cow::Owned(result)
    }

    /// Quick check if space cleaning is needed
    #[inline]
    fn needs_space_cleaning(&self, text: &str) -> bool {
        let mut last_was_space = false;
        let mut in_string = false;

        for c in text.chars() {
            match c {
                '"' => in_string = !in_string,
                ' ' if !in_string => {
                    if last_was_space {
                        return true;
                    }
                    last_was_space = true;
                }
                _ => last_was_space = false,
            }
        }
        false
    }

    /// Find comment position without allocation
    fn find_comment_position(&self, line: &str) -> Option<usize> {
        let mut in_string = false;
        let mut escape_next = false;
        let chars: Vec<char> = line.chars().collect();

        for (i, &c) in chars.iter().enumerate() {
            if escape_next {
                escape_next = false;
                continue;
            }

            match c {
                '"' => in_string = !in_string,
                '\\' if in_string => escape_next = true,
                '/' if !in_string && i + 1 < chars.len() && chars[i + 1] == '/' => {
                    return Some(i);
                }
                _ => {}
            }
        }

        None
    }

    /// Calculate indentation levels for all lines
    fn calculate_indent_levels(&self, parsed_lines: &[ParsedLine<'_>]) -> Vec<u16> {
        let mut levels = Vec::with_capacity(parsed_lines.len());
        let mut current_level = 0u16;

        for line in parsed_lines {
            let trimmed = line.code.trim();

            //  } catch, } finally, } else
            if self.is_continuation_construct(trimmed) && !line.is_full_line_comment {
                current_level = current_level.saturating_sub(1);
                levels.push(current_level);
                current_level += 1;
                continue;
            }

            if line.indent_delta < 0 && !line.is_full_line_comment {
                current_level = current_level.saturating_sub(1);
            }

            levels.push(current_level);

            if line.indent_delta > 0 && !line.is_full_line_comment {
                current_level += 1;
            }
        }

        levels
    }

    /// Write formatted output directly to buffer
    fn write_formatted_lines(
        &self,
        output: &mut String,
        original_lines: &[&str],
        parsed_lines: &[ParsedLine<'_>],
        indent_levels: &[u16],
    ) {
        if self.config.align_inline_comments {
            self.write_with_comment_alignment(output, original_lines, parsed_lines, indent_levels);
        } else {
            self.write_without_comment_alignment(
                output,
                original_lines,
                parsed_lines,
                indent_levels,
            );
        }
    }

    /// Write output with comment alignment
    fn write_with_comment_alignment(
        &self,
        output: &mut String,
        original_lines: &[&str],
        parsed_lines: &[ParsedLine<'_>],
        indent_levels: &[u16],
    ) {
        let mut i = 0;
        let mut consecutive_empty = 0u8;

        while i < parsed_lines.len() {
            let line = &parsed_lines[i];

            if line.code.is_empty() {
                consecutive_empty += 1;
                if consecutive_empty <= self.config.max_consecutive_blank_lines {
                    output.push('\n');
                }
                i += 1;
                continue;
            }

            consecutive_empty = 0;

            if line.is_full_line_comment || line.comment_start.is_none() {
                self.write_single_line(output, original_lines[i], line, indent_levels[i]);
                i += 1;
                continue;
            }

            let group_end = self.find_comment_group_end(parsed_lines, i);

            if group_end - i >= 2 {
                self.write_aligned_comment_group(
                    output,
                    original_lines,
                    parsed_lines,
                    indent_levels,
                    i,
                    group_end,
                );
                i = group_end;
            } else {
                self.write_single_line(output, original_lines[i], line, indent_levels[i]);
                i += 1;
            }
        }

        while output.ends_with('\n') {
            output.pop();
        }
    }

    /// Write output without comment alignment
    fn write_without_comment_alignment(
        &self,
        output: &mut String,
        original_lines: &[&str],
        parsed_lines: &[ParsedLine<'_>],
        indent_levels: &[u16],
    ) {
        let mut consecutive_empty = 0u8;

        for (i, line) in parsed_lines.iter().enumerate() {
            if line.code.is_empty() {
                consecutive_empty += 1;
                if consecutive_empty <= self.config.max_consecutive_blank_lines {
                    output.push('\n');
                }
                continue;
            }

            consecutive_empty = 0;
            self.write_single_line(output, original_lines[i], line, indent_levels[i]);
        }

        while output.ends_with('\n') {
            output.pop();
        }
    }

    /// Find the end of a comment group
    fn find_comment_group_end(&self, parsed_lines: &[ParsedLine<'_>], start: usize) -> usize {
        let mut end = start;

        while end < parsed_lines.len() {
            let line = &parsed_lines[end];
            if line.comment_start.is_some() && !line.is_full_line_comment && !line.code.is_empty() {
                end += 1;
            } else {
                break;
            }
        }

        end
    }

    /// Write aligned comment group
    fn write_aligned_comment_group(
        &self,
        output: &mut String,
        original_lines: &[&str],
        parsed_lines: &[ParsedLine<'_>],
        indent_levels: &[u16],
        start: usize,
        end: usize,
    ) {
        let max_code_len = (start..end)
            .map(|i| {
                let indent = (indent_levels[i] as usize) * (self.config.indent_size as usize);
                indent + parsed_lines[i].code.len()
            })
            .max()
            .unwrap_or(0);

        let comment_start = max_code_len + (self.config.min_comment_spacing as usize);

        for i in start..end {
            let line = &parsed_lines[i];
            let original = original_lines[i];
            let indent = (indent_levels[i] as usize) * (self.config.indent_size as usize);

            for _ in 0..indent {
                output.push(' ');
            }

            output.push_str(&line.code);

            if let Some(comment_pos) = line.comment_start {
                let current_pos = indent + line.code.len();
                let spaces_needed = comment_start.saturating_sub(current_pos);

                for _ in 0..spaces_needed {
                    output.push(' ');
                }

                output.push_str(&original[comment_pos..]);
            }

            output.push('\n');
        }
    }

    /// Write a single line
    #[inline]
    fn write_single_line(
        &self,
        output: &mut String,
        original_line: &str,
        parsed_line: &ParsedLine<'_>,
        indent_level: u16,
    ) {
        if parsed_line.code.is_empty() {
            return;
        }

        let indent = (indent_level as usize) * (self.config.indent_size as usize);
        for _ in 0..indent {
            output.push(' ');
        }

        output.push_str(&parsed_line.code);

        if let Some(comment_pos) = parsed_line.comment_start {
            output.push_str("  ");

            let comment_part = &original_line[comment_pos..];
            output.push_str(comment_part);
        }

        output.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_static_format() {
        let input = r#"recipe "test" {
param name: string = "default"


run "echo hello"
}"#;

        let expected = r#"recipe "test" {
    param name: string = "default"

    run "echo hello"
}"#;

        assert_eq!(Formatter::format(input), expected);
    }

    #[test]
    fn test_performance_format() {
        let input = r#"recipe "test" {
    param name: string = "default"
    run "echo hello"
}"#;

        let result = Formatter::format_with_config(
            input,
            FormatterConfig {
                align_inline_comments: false,
                ..Default::default()
            },
        );
        assert!(result.contains("param name"));
    }

    #[test]
    fn test_comment_alignment_optimization() {
        let input = r#"recipe "test" {
    param name: string = "default" // Parameter name
    param age: number = 25 // User age
    run "echo hello"
}"#;

        let result = Formatter::format(input);

        assert!(result.contains("// Parameter name"));
        assert!(result.contains("// User age"));
    }

    #[test]
    fn test_continuation_constructs() {
        let input = r#"recipe "test_continuation" {
try {
run "work"
} catch e {
print "error"
} finally {
run "cleanup"
}
if condition {
run "if block"
} else {
run "else block"
}
}"#;

        let expected = r#"recipe "test_continuation" {
    try {
        run "work"
    } catch e {
        print "error"
    } finally {
        run "cleanup"
    }
    if condition {
        run "if block"
    } else {
        run "else block"
    }
}"#;

        assert_eq!(Formatter::format(input), expected);
    }
}

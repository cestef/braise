use crate::{create_lexer, SpannedToken};
use braise_core::{BraiseError, Span, Position, FileId};

/// Represents a segment of an interpolated string
#[derive(Debug, Clone, PartialEq)]
pub enum InterpolationSegment {
    /// A literal string segment
    String(String),
    /// A tokenized expression segment with its tokens
    Expression(Vec<SpannedToken>),
}

/// Result of lexing an interpolated string
#[derive(Debug, Clone)]
pub struct InterpolatedString {
    pub segments: Vec<InterpolationSegment>,
    pub span: Span,
}

/// Recursively lex an interpolated string, tokenizing expressions within ${} blocks
pub fn lex_interpolated_string(input: &str, source: &str, start_offset: usize) -> Result<InterpolatedString, BraiseError> {
    let mut segments = Vec::new();
    let mut chars = input.chars().peekable();
    let mut current_string = String::new();
    let mut position = 0;
    
    while let Some(ch) = chars.next() {
        if ch == '$' && chars.peek() == Some(&'{') {
            chars.next(); // consume '{'
            position += 2;
            
            // Save any accumulated string content
            if !current_string.is_empty() {
                segments.push(InterpolationSegment::String(current_string.clone()));
                current_string.clear();
            }
            
            // Extract the expression content
            let expr_start = position;
            let mut expr_content = String::new();
            let mut brace_depth = 1;
            
            for ch in chars.by_ref() {
                position += ch.len_utf8();
                
                if ch == '{' {
                    brace_depth += 1;
                } else if ch == '}' {
                    brace_depth -= 1;
                    if brace_depth == 0 {
                        break;
                    }
                }
                expr_content.push(ch);
            }
            
            if brace_depth > 0 {
                return Err(BraiseError::lexer(
                    "Unclosed interpolation expression".to_string(),
                    source.to_string(),
                    miette::SourceSpan::new((start_offset + expr_start - 2).into(), 2),
                ));
            }
            
            // Tokenize the expression content
            let expr_tokens = tokenize_expression(&expr_content, source, start_offset + expr_start)?;
            segments.push(InterpolationSegment::Expression(expr_tokens));
        } else {
            current_string.push(ch);
            position += ch.len_utf8();
        }
    }
    
    // Add any remaining string content
    if !current_string.is_empty() {
        segments.push(InterpolationSegment::String(current_string));
    }
    
    Ok(InterpolatedString {
        segments,
        span: Span::new(
            Position::from_offset(start_offset),
            Position::from_offset(start_offset + input.len()),
            FileId(0), // Default file ID
        ),
    })
}

/// Tokenize an expression string from within an interpolation
fn tokenize_expression(expr: &str, source: &str, start_offset: usize) -> Result<Vec<SpannedToken>, BraiseError> {
    let mut lexer = create_lexer(expr);
    let mut tokens = Vec::new();
    
    while let Some(result) = lexer.next() {
        match result {
            Ok(token) => {
                let span = lexer.extras.span_from_range(lexer.span());
                tokens.push(SpannedToken::new(token, span));
            }
            Err(error_range) => {
                return Err(BraiseError::lexer(
                    "Invalid token in interpolation expression".to_string(),
                    source.to_string(),
                    miette::SourceSpan::new(
                        (start_offset + error_range.start).into(),
                        error_range.len()
                    ),
                ));
            }
        }
    }
    
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_simple_interpolation() {
        let input = "Hello ${name}!";
        let result = lex_interpolated_string(input, input, 0).unwrap();
        
        assert_eq!(result.segments.len(), 3);
        assert!(matches!(result.segments[0], InterpolationSegment::String(_)));
        assert!(matches!(result.segments[1], InterpolationSegment::Expression(_)));
        assert!(matches!(result.segments[2], InterpolationSegment::String(_)));
    }
    
    #[test]
    fn test_nested_braces() {
        let input = "Result: ${match x { 1 => \"one\", _ => \"other\" }}";
        let result = lex_interpolated_string(input, input, 0).unwrap();
        
        assert_eq!(result.segments.len(), 2);
        assert!(matches!(result.segments[0], InterpolationSegment::String(_)));
        assert!(matches!(result.segments[1], InterpolationSegment::Expression(_)));
    }
    
    #[test]
    fn test_multiple_interpolations() {
        let input = "Sum: ${a} + ${b} = ${a + b}";
        let result = lex_interpolated_string(input, input, 0).unwrap();
        
        assert_eq!(result.segments.len(), 6);
        // Should be: "Sum: ", ${a}, " + ", ${b}, " = ", ${a + b}
    }
    
    #[test]
    fn test_unclosed_interpolation() {
        let input = "Hello ${name";
        let result = lex_interpolated_string(input, input, 0);
        
        assert!(result.is_err());
    }
}
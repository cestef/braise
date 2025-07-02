use std::ops::Range;

use braise_core::{TokenPosition, TokenSpan};

/// Lexer extras to track position information
#[derive(Debug, Clone)]
pub struct LexerExtras {
    /// Current line number (1-based)
    pub line: u32,
    /// Character position at the start of the current line (0-based offset)
    pub line_start_offset: usize,
    /// Source text for position calculations
    pub source: String,
}

impl Default for LexerExtras {
    fn default() -> Self {
        Self {
            line: 1,
            line_start_offset: 0,
            source: String::new(),
        }
    }
}

impl LexerExtras {
    pub fn new(source: String) -> Self {
        Self {
            line: 1,
            line_start_offset: 0,
            source,
        }
    }

    pub fn position_at_offset(&self, offset: usize) -> TokenPosition {
        let column = if offset >= self.line_start_offset {
            (offset - self.line_start_offset) as u32 + 1
        } else {
            1
        };

        TokenPosition::new(self.line, column, offset)
    }

    pub fn span_from_range(&self, range: Range<usize>) -> TokenSpan {
        let start = self.position_at_offset(range.start);
        let end = self.position_at_offset(range.end);
        TokenSpan::new(start, end)
    }
}

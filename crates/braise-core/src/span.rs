#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Position {
    pub line: u32,
    pub column: u32,
    pub offset: usize,
}

impl Position {
    pub fn new(line: u32, column: u32, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }

    pub fn from_offset(offset: usize) -> Self {
        Self {
            line: 1,
            column: 1,
            offset,
        }
    }
}

/// Position information for a token (used by lexer)
#[derive(Debug, Clone, PartialEq)]
pub struct TokenPosition {
    pub line: u32,
    pub column: u32,
    pub offset: usize,
}

impl TokenPosition {
    pub fn new(line: u32, column: u32, offset: usize) -> Self {
        Self {
            line,
            column,
            offset,
        }
    }
}

/// Position range for a token (used by lexer)
#[derive(Debug, Clone, PartialEq)]
pub struct TokenSpan {
    pub start: TokenPosition,
    pub end: TokenPosition,
}

impl TokenSpan {
    pub fn new(start: TokenPosition, end: TokenPosition) -> Self {
        Self { start, end }
    }
}

impl From<TokenPosition> for Position {
    fn from(pos: TokenPosition) -> Self {
        Self {
            line: pos.line,
            column: pos.column,
            offset: pos.offset,
        }
    }
}

/// Represents a span in source code
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: Position,
    pub end: Position,
    pub file_id: FileId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub usize);

impl Span {
    pub fn new(start: Position, end: Position, file_id: FileId) -> Self {
        Self {
            start,
            end,
            file_id,
        }
    }

    pub fn merge(&self, other: &Span) -> Span {
        Span {
            start: self.start,
            end: other.end,
            file_id: self.file_id,
        }
    }

    pub fn contains(&self, span: &Span) -> bool {
        // TODO: check for file_id equality?
        self.start.offset <= span.start.offset && self.end.offset >= span.end.offset
    }

    /// Create a span from a lexer token span
    pub fn from_token_span(token_span: &TokenSpan, file_id: FileId) -> Self {
        Self {
            start: token_span.start.clone().into(),
            end: token_span.end.clone().into(),
            file_id,
        }
    }

    /// Get the length of this span in characters
    pub fn len(&self) -> usize {
        self.end.offset.saturating_sub(self.start.offset)
    }

    /// Check if this span is empty
    pub fn is_empty(&self) -> bool {
        self.start.offset == self.end.offset
    }
}

/// A value with associated source location
#[derive(Debug, Clone, PartialEq)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Spanned<U> {
        Spanned::new(f(self.value), self.span)
    }

    /// Create a spanned value from a lexer token span
    pub fn from_token_span(value: T, token_span: &TokenSpan, file_id: FileId) -> Self {
        Self {
            value,
            span: Span::from_token_span(token_span, file_id),
        }
    }
}

/// Utility for converting byte offsets to line/column positions
#[derive(Debug)]
pub struct SourceMap {
    /// Line start positions (byte offsets)
    line_starts: Vec<usize>,
    /// The source text
    source: String,
    /// File ID for this source
    file_id: FileId,
}

impl SourceMap {
    pub fn new(source: String, file_id: FileId) -> Self {
        let mut line_starts = vec![0];

        for (i, ch) in source.char_indices() {
            if ch == '\n' {
                line_starts.push(i + 1);
            }
        }

        Self {
            line_starts,
            source,
            file_id,
        }
    }

    /// Convert a byte offset to a Position
    pub fn position_at_offset(&self, offset: usize) -> Position {
        let line_index = match self.line_starts.binary_search(&offset) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };

        let line = (line_index + 1) as u32;
        let line_start = self.line_starts.get(line_index).copied().unwrap_or(0);
        let column = (offset.saturating_sub(line_start) + 1) as u32;

        Position::new(line, column, offset)
    }

    /// Create a span from a byte range
    pub fn span_from_range(&self, start: usize, end: usize) -> Span {
        let start_pos = self.position_at_offset(start);
        let end_pos = self.position_at_offset(end);
        Span::new(start_pos, end_pos, self.file_id)
    }

    /// Get the source text
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Get the file ID
    pub fn file_id(&self) -> FileId {
        self.file_id
    }
}

impl From<&Span> for miette::SourceSpan {
    fn from(span: &Span) -> Self {
        miette::SourceSpan::new(span.start.offset.into(), span.len())
    }
}

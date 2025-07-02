use std::ops::Range;

use braise_core::TokenSpan;

use crate::{Token, extras::LexerExtras};

#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub span: TokenSpan,
}

impl SpannedToken {
    pub fn new(token: Token, span: TokenSpan) -> Self {
        SpannedToken { token, span }
    }

    pub fn new_from_range(token: Token, range: Range<usize>, extras: &LexerExtras) -> Self {
        let span = extras.span_from_range(range);
        SpannedToken { token, span }
    }
}

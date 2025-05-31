//! Utilities to assist with parsing items from a [`TokenBuffer`].

use super::base::*;

/// Cursor into a [`TokenBuffer`], which is free to be advanced independently of other cursors
/// in the same buffer.
#[derive(Clone, Copy)]
pub struct Cursor<'b> {
    /// Buffer that this cursor refers to.
    buffer: &'b TokenBuffer,
    /// Next offset into the buffer.
    offset: usize,
}

impl<'b> Cursor<'b> {
    pub fn new(buffer: &'b TokenBuffer) -> Self {
        Self { buffer, offset: 0 }
    }

    /// Produce the token that the cursor is currently pointed at.
    fn entry(&self) -> Option<&BaseToken> {
        self.buffer.get(self.offset)
    }

    /// Consume the current cursor, and create a new cursor which points to the next token.
    fn next_cursor(mut self) -> Self {
        self.offset += 1;
        self
    }

    /// Determine if the cursor is at the end of the buffer.
    pub fn eof(&self) -> bool {
        self.offset >= self.buffer.len()
    }

    /// If the next token is [`Ident`], return it and advance the cursor.
    pub fn ident(self) -> Option<(Ident, Self)> {
        match self.entry()? {
            BaseToken::Ident(ident) => Some((ident.clone(), self.next_cursor())),
            _ => None,
        }
    }

    /// If the next token is [`Punct`], return it and advance the cursor.
    pub fn punct(self) -> Option<(Punct, Self)> {
        match self.entry()? {
            BaseToken::Punct(punct) => Some((punct.clone(), self.next_cursor())),
            _ => None,
        }
    }
}

/// Wrapper over [`Cursor`] with convenience methods to assist with parsing.
#[derive(Clone)]
pub struct ParseStream<'b> {
    cursor: Cursor<'b>,
}

impl<'b> ParseStream<'b> {
    pub fn new(cursor: Cursor<'b>) -> Self {
        Self { cursor }
    }

    /// Parse a token from the stream.
    pub fn parse<T: Parse>(&mut self) -> Result<T, String> {
        T::parse(self)
    }

    /// Peek at the next token in the stream.
    pub fn peek<T: Parse>(&self) -> bool {
        T::parse(&mut self.clone()).is_ok()
    }

    /// Attempt to parse a token from the stream, only advancing the stream if the parse is
    /// successful.
    pub fn step<T>(
        &mut self,
        function: impl FnOnce(Cursor) -> Result<(T, Cursor), String>,
    ) -> Result<T, String> {
        let (result, cursor) = function(self.cursor)?;
        self.cursor = cursor;
        Ok(result)
    }

    pub fn lookahead(&self) -> Lookahead {
        Lookahead::new(self.cursor)
    }
}

pub trait Parse: Sized {
    fn parse(input: &mut ParseStream) -> Result<Self, String>;
}

/// A single token.
pub trait Token {
    fn peek(cursor: Cursor) -> bool;
    fn display() -> &'static str;
}

/// A type that can be parsed from a single token.
pub trait Peek {
    type Token: Token;
}

pub struct Lookahead<'b> {
    cursor: Cursor<'b>,
    comparisons: Vec<&'static str>,
}

impl<'b> Lookahead<'b> {
    pub fn new(cursor: Cursor<'b>) -> Self {
        Self {
            cursor,
            comparisons: Vec::new(),
        }
    }

    pub fn peek<T: Peek>(&mut self) -> bool {
        self.comparisons.push(T::Token::display());
        T::Token::peek(self.cursor)
    }

    pub fn error(self) -> String {
        match self.comparisons.len() {
            0 => {
                if self.cursor.eof() {
                    "unexpected end of input".into()
                } else {
                    "unexpected token".into()
                }
            }
            1 => {
                format!("expected {}", self.comparisons[0])
            }
            2 => {
                format!(
                    "expected {} or {}",
                    self.comparisons[0], self.comparisons[1]
                )
            }
            _ => {
                format!("expected one of: {}", self.comparisons.join(", "))
            }
        }
    }
}

mod lookahead;
mod punctuated;

use std::cell::Cell;

pub use self::lookahead::Lookahead;

use crate::buffer::{BufferToken, Cursor, TokenBuffer};

/// All available entrypoints for parsing.
pub mod entrypoint {
    use super::*;

    /// Parse `T` from a string. Will use `BaseToken` as the low-level token when parsing.
    pub fn parse_str<T: Parse<BaseToken>, BaseToken: BufferToken>(s: &str) -> Result<T, String> {
        let buffer = TokenBuffer::<BaseToken>::new(s)?;
        let parser = buffer.parser();

        T::parse(&parser)
    }
}

/// A value which can be parsed from a [`BufferParser`] containing `BaseToken`s.
pub trait Parse<BaseToken>: Sized {
    /// Parse a value with the provided parser.
    fn parse(parser: BufferParser<'_, BaseToken>) -> Result<Self, String>;
}

/// A value which represents a single `BaseToken`.
pub trait Token<BaseToken>: Sized {
    /// Determine if the cursor currently points to this token.
    fn peek(cursor: Cursor<'_, BaseToken>) -> bool;

    /// A string representation of this value.
    fn display() -> &'static str;
}

/// A reference to [`FullBufferParser`]. Simplifies the ergonomics of passing around the same
/// instance of a parser to multiple functions.
pub type BufferParser<'b, BaseToken> = &'b FullBufferParser<'b, BaseToken>;

/// A parser which operates over a [`TokenBuffer`] containing `BaseToken`s with a [`Cursor`].
#[derive(Clone)]
pub struct FullBufferParser<'b, BaseToken> {
    /// Current location of this parser.
    ///
    /// [`Cell`] provides mutable access behind a reference, which is required for
    /// [`BufferParser`].
    cursor: Cell<Cursor<'b, BaseToken>>,
}

impl<'b, BaseToken> FullBufferParser<'b, BaseToken> {
    /// Create a new parser from a [`Cursor`].
    pub fn new(cursor: Cursor<'b, BaseToken>) -> Self {
        Self {
            cursor: Cell::new(cursor),
        }
    }

    /// Parse `T` with the provided function.
    pub fn parse_with<T>(
        &'b self,
        function: fn(BufferParser<'b, BaseToken>) -> Result<T, String>,
    ) -> Result<T, String> {
        function(self)
    }

    /// Parse `T` with the [`Parse`] implementation.
    pub fn parse<T: Parse<BaseToken>>(&'b self) -> Result<T, String> {
        self.parse_with(T::parse)
    }

    /// Attempt to parse a token from the stream, only advancing the stream if the parse is
    /// successful.
    pub fn step<T>(
        &'b self,
        function: impl FnOnce(Cursor<'b, BaseToken>) -> Result<(T, Cursor<'b, BaseToken>), String>,
    ) -> Result<T, String> {
        let (result, cursor) = function(self.cursor())?;
        self.cursor.set(cursor);
        Ok(result)
    }

    pub fn lookahead(&self) -> Lookahead<'b, BaseToken> {
        Lookahead::new(self.cursor())
    }

    /// Check if the end of the buffer has been reached.
    pub fn is_empty(&self) -> bool {
        self.cursor().eof()
    }

    /// Provide a copy of the current [`Cursor`].
    fn cursor(&self) -> Cursor<'b, BaseToken> {
        self.cursor.get()
    }
}

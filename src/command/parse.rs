//! Utilities to assist with parsing items from a [`TokenBuffer`].

use std::cell::Cell;

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

pub type ParseStream<'b> = &'b ParseBuffer<'b>;

/// Wrapper over [`Cursor`] with convenience methods to assist with parsing.
#[derive(Clone)]
pub struct ParseBuffer<'b> {
    cursor: Cell<Cursor<'b>>,
}

impl<'b> ParseBuffer<'b> {
    pub fn new(cursor: Cursor<'b>) -> Self {
        Self {
            cursor: Cell::new(cursor),
        }
    }

    pub fn call<T>(
        &'b self,
        function: fn(ParseStream<'b>) -> Result<T, String>,
    ) -> Result<T, String> {
        function(self)
    }

    /// Parse a token from the stream.
    pub fn parse<T: Parse>(&'b self) -> Result<T, String> {
        T::parse(self)
    }

    /// Attempt to parse a token from the stream, only advancing the stream if the parse is
    /// successful.
    pub fn step<T>(
        &'b self,
        function: impl FnOnce(Cursor<'b>) -> Result<(T, Cursor<'b>), String>,
    ) -> Result<T, String> {
        let (result, cursor) = function(self.cursor())?;
        self.cursor.set(cursor);
        Ok(result)
    }

    pub fn lookahead(&self) -> Lookahead {
        Lookahead::new(self.cursor())
    }

    pub fn is_empty(&self) -> bool {
        self.cursor().eof()
    }

    fn cursor(&self) -> Cursor {
        self.cursor.get()
    }
}

pub trait Parse: Sized {
    fn parse(input: ParseStream) -> Result<Self, String>;
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

#[derive(Clone, Debug)]
pub struct Punctuated<T, P> {
    pairs: Vec<(T, P)>,
    last: Option<T>,
}

impl<T, P> Punctuated<T, P> {
    pub fn new() -> Self {
        Self {
            pairs: Vec::new(),
            last: None,
        }
    }

    pub fn parse_terminated(input: ParseStream) -> Result<Self, String>
    where
        T: Parse,
        P: Parse,
    {
        Self::parse_terminated_with(input, T::parse)
    }

    /// Parse a punctuated stream, which must only contain `T` and `P`.
    pub fn parse_terminated_with(
        input: ParseStream,
        parser: fn(ParseStream) -> Result<T, String>,
    ) -> Result<Self, String>
    where
        P: Parse,
    {
        let mut punctuated = Self::new();

        loop {
            if input.is_empty() {
                break;
            }

            let value = parser(input)?;

            if input.is_empty() {
                punctuated.last = Some(value);
                break;
            }

            let punctuation = input.parse::<P>()?;
            punctuated.pairs.push((value, punctuation));
        }

        Ok(punctuated)
    }

    /// Parse a punctuated stream, stopping if there is no more `P` in the stream. Trailing
    /// punctuation is not allowed.
    pub fn parse_separated_non_empty(input: ParseStream) -> Result<Self, String>
    where
        T: Parse,
        P: Token + Parse,
    {
        Self::parse_separated_non_empty_with(input, T::parse)
    }

    pub fn parse_separated_non_empty_with(
        input: ParseStream,
        parser: fn(ParseStream) -> Result<T, String>,
    ) -> Result<Self, String>
    where
        P: Token + Parse,
    {
        let mut punctuated = Self::new();

        loop {
            let value = parser(input)?;

            if !P::peek(input.cursor()) {
                punctuated.last = Some(value);
                break;
            }

            let punct = input.parse()?;
            punctuated.pairs.push((value, punct));
        }

        Ok(punctuated)
    }
}

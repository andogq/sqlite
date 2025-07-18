//! The first stage of parsing accepts a [`str`], and lexes it into low-level tokens, buffering the
//! result. The tokens are constructed from a single [`char`] and an iterator over [`char`]s,
//! allowing implementors to pull additional characters for the token (such as for multi-character
//! identifiers). At the conclusion of this stage, a [`TokenBuffer`] will be produced which can be
//! traversed for higher level parsing.

use std::iter::{self, Peekable};

use derive_more::Deref;

use crate::parse::FullBufferParser;

/// A low level token, which is directly constructed from at least one character.
pub trait BufferToken: Clone + Sized {
    /// Create a new token from a [`char`], and an iterator of additional [`char`]s.
    fn from_char(c: char, chars: &mut Peekable<impl Iterator<Item = char>>) -> Outcome<Self>;
}

/// Outcome when parsing a [`BufferToken`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Outcome<T: Sized> {
    /// Token was successfully produced.
    Token(T),
    /// This character should be skipped.
    Skip,
    /// This character was unexpected.
    Unexpected,
}

impl<T: Sized> Outcome<T> {
    pub fn map<U: Sized>(self, f: impl FnOnce(T) -> U) -> Outcome<U> {
        match self {
            Outcome::Token(token) => Outcome::Token(f(token)),
            Outcome::Skip => Outcome::Skip,
            Outcome::Unexpected => Outcome::Unexpected,
        }
    }
}

/// Buffered stream of tokens.
#[derive(Deref)]
pub struct TokenBuffer<BaseToken> {
    /// Underlying buffer containing all tokens.
    buffer: Box<[BaseToken]>,
}

impl<BaseToken> TokenBuffer<BaseToken> {
    /// Tokenise the source, and produce a new [`TokenBuffer`].
    pub fn new(source: &str) -> Result<Self, String>
    where
        BaseToken: BufferToken,
    {
        let mut chars = source.chars().peekable();

        Ok(Self::new_with_tokens(
            iter::from_fn(move || {
                let c = chars.next()?;

                match BaseToken::from_char(c, &mut chars) {
                    Outcome::Token(token) => Some(Some(Ok(token))),
                    Outcome::Skip => Some(None),
                    Outcome::Unexpected => Some(Some(Err(format!("unexpected character: {c}")))),
                }
            })
            .flatten()
            .collect::<Result<Vec<_>, _>>()?,
        ))
    }

    /// Create a new buffer with the provided tokens.
    pub(crate) fn new_with_tokens(tokens: Vec<BaseToken>) -> Self {
        Self {
            buffer: tokens.into_boxed_slice(),
        }
    }

    /// Create an empty [`TokenBuffer`].
    pub fn empty() -> Self {
        Self {
            buffer: vec![].into_boxed_slice(),
        }
    }

    /// Create a new cursor into this buffer.
    pub fn cursor(&self) -> Cursor<BaseToken> {
        Cursor::new(self)
    }

    /// Create a new stream to operate on this token buffer.
    pub fn parser(&self) -> FullBufferParser<'_, BaseToken> {
        FullBufferParser::new(self.cursor())
    }
}

/// Cursor into a [`TokenBuffer`], which is free to be advanced independently of other cursors
/// in the same buffer.
pub struct Cursor<'b, BaseToken> {
    /// Buffer that this cursor refers to.
    buffer: &'b [BaseToken],
    /// Next offset into the buffer.
    offset: usize,
}
impl<'b, BaseToken> Clone for Cursor<'b, BaseToken> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<'b, BaseToken> Copy for Cursor<'b, BaseToken> {}

impl<'b, BaseToken> Cursor<'b, BaseToken> {
    /// Create a new cursor on the provided buffer.
    pub fn new(buffer: &'b [BaseToken]) -> Self {
        Self { buffer, offset: 0 }
    }

    /// Produce the token that the cursor is currently pointed at.
    fn entry(&self) -> Option<&BaseToken> {
        self.buffer.get(self.offset)
    }

    /// Consume the current cursor, and create a new cursor which points to the next token.
    pub(crate) fn next_cursor(mut self) -> Self {
        self.offset += 1;
        self
    }

    /// Split this cursor into two cursors, one which will advance until `offset` (exclusive), and
    /// another which will start from `offset` and advance till the end of the buffer.
    pub(crate) fn split_cursor(self, offset: usize) -> (Self, Self) {
        (
            Self::new(&self.buffer[self.offset..self.offset + offset]),
            Self::new(&self.buffer[self.offset + offset..]),
        )
    }

    /// Determine if the cursor is at the end of the buffer.
    pub fn eof(&self) -> bool {
        self.offset >= self.buffer.len()
    }

    /// Produce the next token, and the next cursor.
    pub fn token(self) -> Option<(BaseToken, Self)>
    where
        BaseToken: Clone,
    {
        Some((self.entry()?.clone(), self.next_cursor()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use derive_more::From;
    use rstest::rstest;

    /// Token parsed from a character.
    #[derive(Clone)]
    struct Char<const C: char>;
    impl<const C: char> BufferToken for Char<C> {
        fn from_char(c: char, _chars: &mut Peekable<impl Iterator<Item = char>>) -> Outcome<Self> {
            if c == C {
                Outcome::Token(Self)
            } else {
                Outcome::Unexpected
            }
        }
    }

    /// Token which will skip a character during parsing.
    #[derive(Clone)]
    struct Skip<const C: char, T: BufferToken>(T);
    impl<const C: char, T: BufferToken> BufferToken for Skip<C, T> {
        fn from_char(c: char, chars: &mut Peekable<impl Iterator<Item = char>>) -> Outcome<Self> {
            if c == C {
                return Outcome::Skip;
            }

            T::from_char(c, chars).map(Self)
        }
    }

    /// Token which will parse from `a`.
    type A = Char<'a'>;
    const A: A = Char;
    /// Token which will parse from `a`, skipping any `b`.
    type ASkipB = Skip<'b', Char<'a'>>;
    const A_SKIP_B: ASkipB = Skip(A);

    mod from_source {
        use super::*;

        #[rstest]
        #[case(A, "", 0)]
        #[case(A, "a", 1)]
        #[case(A, "aa", 2)]
        #[case(A, "aaaaa", 5)]
        #[case(A_SKIP_B, "", 0)]
        #[case(A_SKIP_B, "b", 0)]
        #[case(A_SKIP_B, "ab", 1)]
        #[case(A_SKIP_B, "ba", 1)]
        #[case(A_SKIP_B, "aaabbaa", 5)]
        fn valid<T: BufferToken>(
            #[case] _token: T,
            #[case] source: &'static str,
            #[case] count: usize,
        ) {
            let buf = TokenBuffer::<T>::new(source).unwrap();
            assert_eq!(buf.len(), count);
        }

        #[rstest]
        #[case(A, "b", 'b')]
        #[case(A, "aaab", 'b')]
        #[case(A_SKIP_B, "aaabc", 'c')]
        fn unexpected<T: BufferToken>(
            #[case] _token: T,
            #[case] source: &'static str,
            #[case] c: char,
        ) {
            let Err(e) = TokenBuffer::<T>::new(source) else {
                panic!("expected `Err`, found `Ok`");
            };

            assert_eq!(e, format!("unexpected character: {c}"));
        }
    }

    mod cursor {
        use super::*;

        #[rstest]
        #[case("a", 0, true)]
        #[case("a", 1, false)]
        #[case("", 0, false)]
        #[case("aaaaa", 4, true)]
        #[case("aaaaa", 5, false)]
        fn entry(#[case] source: &str, #[case] offset: usize, #[case] present: bool) {
            let buffer = TokenBuffer::<A>::new(source).unwrap();
            let cursor = Cursor {
                buffer: &buffer,
                offset,
            };

            assert_eq!(cursor.entry().is_some(), present);
        }

        #[rstest]
        #[case(TokenBuffer::empty(), true)]
        #[case(TokenBuffer::new("a").unwrap(), false)]
        fn eof(#[case] buffer: TokenBuffer<A>, #[case] expected: bool) {
            let cursor = buffer.cursor();
            assert_eq!(cursor.eof(), expected);
        }

        mod token {
            use super::*;

            #[test]
            fn valid_token() {
                let buffer = TokenBuffer::<A>::new("aa").unwrap();
                let cursor = buffer.cursor();
                assert_eq!(cursor.offset, 0);

                let (_token, cursor) = cursor.token().unwrap();
                assert_eq!(cursor.offset, 1);
                assert!(!cursor.eof())
            }

            #[test]
            fn exhaust_tokens() {
                let buffer = TokenBuffer::<A>::new("a").unwrap();
                let cursor = buffer.cursor();
                assert_eq!(cursor.offset, 0);

                let (_token, cursor) = cursor.token().unwrap();
                assert_eq!(cursor.offset, 1);
                assert!(cursor.eof())
            }
        }
    }

    #[derive(Clone, Debug, From)]
    struct CharToken(char);

    #[rstest]
    #[case(vec!['a'.into(), 'b'.into()], 0, 1, Some('a'), Some('b'))]
    #[case(vec!['a'.into(), 'b'.into()], 1, 1, Some('b'), None)]
    #[case(vec!['a'.into(), 'b'.into()], 0, 0, None, Some('a'))]
    fn split_cursor(
        #[case] tokens: Vec<CharToken>,
        #[case] start_offset: usize,
        #[case] offset: usize,
        #[case] first_expected: Option<char>,
        #[case] second_expected: Option<char>,
    ) {
        let buffer = TokenBuffer::new_with_tokens(tokens);
        let cursor = Cursor {
            buffer: &buffer,
            offset: start_offset,
        };

        let (cursor_a, cursor_b) = cursor.split_cursor(offset);

        for (cursor, expected) in [(cursor_a, first_expected), (cursor_b, second_expected)] {
            match (cursor.token().map(|(tok, _)| tok), expected) {
                (Some(cursor), Some(expected)) => {
                    assert_eq!(cursor.0, expected);
                }
                (None, None) => {}
                (Some(_), None) => panic!("no token expected, but one produced"),
                (None, Some(_)) => panic!("token expected, but none produced"),
            }
        }
    }
}

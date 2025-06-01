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

/// Helper trait for converting between different token types. This is useful for downcasting from
/// an enum of token types, into one specific token variant.
pub trait IntoToken<T>: BufferToken {
    /// Consume the token, and produce a new representation of this token.
    fn into_token(self) -> Option<T>;
}

/// Blanket implemenation to allow [`BufferToken`]s to convert into themselves.
impl<T: BufferToken> IntoToken<Self> for T {
    fn into_token(self) -> Option<Self> {
        Some(self)
    }
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

        Ok(TokenBuffer {
            buffer: iter::from_fn(move || {
                let c = chars.next()?;

                match BaseToken::from_char(c, &mut chars) {
                    Outcome::Token(token) => Some(Some(Ok(token))),
                    Outcome::Skip => Some(None),
                    Outcome::Unexpected => Some(Some(Err(format!("unexpected character: {c}")))),
                }
            })
            .flatten()
            .collect::<Result<Vec<_>, _>>()?
            .into_boxed_slice(),
        })
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
    buffer: &'b TokenBuffer<BaseToken>,
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
    pub fn new(buffer: &'b TokenBuffer<BaseToken>) -> Self {
        Self { buffer, offset: 0 }
    }

    /// Create a new buffer with the provided offset. There are no checks whether the offset is
    /// valid for the buffer.
    fn new_with_offset(buffer: &'b TokenBuffer<BaseToken>, offset: usize) -> Self {
        Self { buffer, offset }
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

    /// If the next token matches `U`, return it along with an advanced cursor.
    pub fn token<T>(self) -> Option<(T, Self)>
    where
        BaseToken: IntoToken<T>,
    {
        Some((self.entry()?.clone().into_token()?, self.next_cursor()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

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

    /// Token which maybe `C`, or any other character.
    #[derive(Clone)]
    enum Maybe<const C: char> {
        C,
        Other(char),
    }
    impl<const C: char> BufferToken for Maybe<C> {
        fn from_char(c: char, _chars: &mut Peekable<impl Iterator<Item = char>>) -> Outcome<Self> {
            Outcome::Token(if c == C { Self::C } else { Self::Other(c) })
        }
    }
    impl<const C: char> IntoToken<Char<C>> for Maybe<C> {
        fn into_token(self) -> Option<Char<C>> {
            match self {
                Maybe::C => Some(Char),
                _ => None,
            }
        }
    }
    impl<const C: char> IntoToken<char> for Maybe<C> {
        fn into_token(self) -> Option<char> {
            match self {
                Maybe::Other(c) => Some(c),
                _ => None,
            }
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
            let cursor = Cursor::new_with_offset(&buffer, offset);

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

            #[test]
            fn into_token() {
                let buffer = TokenBuffer::<Maybe<'a'>>::new("ab").unwrap();
                let cursor = buffer.cursor();
                assert_eq!(cursor.offset, 0);

                let (_token, cursor): (A, _) = cursor.token().unwrap();
                assert_eq!(cursor.offset, 1);

                assert!(cursor.clone().token::<A>().is_none());

                let (token, cursor): (char, _) = cursor.token().unwrap();
                assert_eq!(cursor.offset, 2);
                assert_eq!(token, 'b');
            }
        }
    }
}

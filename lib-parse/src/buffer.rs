//! The first stage of parsing accepts a [`str`], and lexes it into low-level tokens, buffering the
//! result. The tokens are constructed from a single [`char`] and an iterator over [`char`]s,
//! allowing implementors to pull additional characters for the token (such as for multi-character
//! identifiers). At the conclusion of this stage, a [`TokenBuffer`] will be produced which can be
//! traversed for higher level parsing.

use std::iter::{self, Peekable};

use derive_more::Deref;

/// A low level token, which is directly constructed from at least one character.
pub trait BufferToken: Sized {
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
pub struct TokenBuffer<T> {
    /// Underlying buffer containing all tokens.
    buffer: Box<[T]>,
}

impl<T: BufferToken> TokenBuffer<T> {
    /// Tokenise the source, and produce a new [`TokenBuffer`].
    pub fn new(source: &str) -> Result<Self, String> {
        let mut chars = source.chars().peekable();

        Ok(TokenBuffer {
            buffer: iter::from_fn(move || {
                let c = chars.next()?;

                match T::from_char(c, &mut chars) {
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

    // /// Create a new stream to operate on this token buffer.
    // pub fn stream(&self) -> ParseBuffer {
    //     ParseBuffer::new(self.cursor())
    // }
    //
    // /// Create a new cursor into this buffer.
    // pub fn cursor(&self) -> Cursor {
    //     Cursor::new(self)
    // }
}

#[cfg(test)]
mod test {
    use super::*;

    use rstest::rstest;

    /// Token parsed from `a`.
    struct A;
    impl BufferToken for A {
        fn from_char(c: char, _chars: &mut Peekable<impl Iterator<Item = char>>) -> Outcome<Self> {
            if c == 'a' {
                Outcome::Token(Self)
            } else {
                Outcome::Unexpected
            }
        }
    }

    /// Token parsed from `a`, which will skip `b`.
    struct ASkipB;
    impl BufferToken for ASkipB {
        fn from_char(c: char, _chars: &mut Peekable<impl Iterator<Item = char>>) -> Outcome<Self> {
            match c {
                'a' => Outcome::Token(Self),
                'b' => Outcome::Skip,
                _ => Outcome::Unexpected,
            }
        }
    }

    #[rstest]
    #[case(A, "", 0)]
    #[case(A, "a", 1)]
    #[case(A, "aa", 2)]
    #[case(A, "aaaaa", 5)]
    #[case(ASkipB, "", 0)]
    #[case(ASkipB, "b", 0)]
    #[case(ASkipB, "ab", 1)]
    #[case(ASkipB, "ba", 1)]
    #[case(ASkipB, "aaabbaa", 5)]
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
    #[case(ASkipB, "aaabc", 'c')]
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

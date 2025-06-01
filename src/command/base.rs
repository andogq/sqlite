//! Base tokenising module, which will turn a string into a stream of [`BaseToken`]s within a
//! [`TokenBuffer`].

use std::iter::{self, Peekable};

use derive_more::Deref;

use super::{parse::*, token::is_keyword};

/// Punctuation tokens.
#[derive(Clone, Debug, PartialEq)]
pub enum Punct {
    Asterisk,
    Comma,
    Semicolon,
}

impl TryFrom<String> for Punct {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(match value.as_str() {
            "*" => Self::Asterisk,
            "," => Self::Comma,
            ";" => Self::Semicolon,
            _ => return Err(format!("unsupported punctuation: {value}")),
        })
    }
}

impl PartialEq<char> for &Punct {
    fn eq(&self, other: &char) -> bool {
        let c = match self {
            Punct::Asterisk => '*',
            Punct::Comma => ',',
            Punct::Semicolon => ';',
        };

        c == *other
    }
}

impl PartialEq<char> for Punct {
    fn eq(&self, other: &char) -> bool {
        <&Punct as PartialEq<char>>::eq(&self, other)
    }
}

/// Identifier tokens.
#[derive(Clone, Debug, Deref, PartialEq)]
pub struct Ident(String);

impl TryFrom<String> for Ident {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let mut chars = value.chars();

        // This will also check if the ident is greater than 1 character.
        let first_valid = chars
            .next()
            .filter(|c| c.is_alphabetic() || *c == '_')
            .is_some();
        let rest_valid = chars.all(|c| c.is_alphanumeric() || c == '_');

        if !first_valid || !rest_valid {
            return Err(format!("invalid ident: {value}"));
        }

        Ok(Self(value))
    }
}

impl<S: ?Sized + AsRef<str>> PartialEq<S> for Ident {
    fn eq(&self, other: &S) -> bool {
        self.0 == other.as_ref()
    }
}

impl Token for Ident {
    fn peek(cursor: Cursor) -> bool {
        cursor.ident().is_some()
    }

    fn display() -> &'static str {
        "identifier"
    }
}

impl Parse for Ident {
    fn parse(input: ParseStream) -> Result<Self, String> {
        input.step(|cursor| {
            let (ident, cursor) = cursor
                .ident()
                .ok_or_else(|| "expected identifier".to_string())?;

            // Filter out keywords
            if is_keyword(&ident) {
                return Err(format!("expected identifier, found keyword: {}", *ident));
            }

            Ok((ident, cursor))
        })
    }
}

impl Peek for Ident {
    type Token = Self;
}

/// Combination of all supported tokens.
#[derive(Clone, Debug, PartialEq)]
pub enum BaseToken {
    Punct(Punct),
    Ident(Ident),
}

impl From<Punct> for BaseToken {
    fn from(punct: Punct) -> Self {
        Self::Punct(punct)
    }
}

impl From<Ident> for BaseToken {
    fn from(ident: Ident) -> Self {
        Self::Ident(ident)
    }
}

/// Buffered stream of tokens.
#[derive(Deref)]
pub struct TokenBuffer {
    /// Underlying buffer containing all tokens.
    buffer: Box<[BaseToken]>,
}

impl TokenBuffer {
    /// Tokenise the source, and produce a new [`TokenBuffer`].
    pub fn new(source: &str) -> Result<Self, String> {
        let mut chars = source.chars().peekable();

        /// Utility to continually consume characters from a stream as long as a condition is true.
        fn take_while(
            chars: &mut Peekable<impl Iterator<Item = char>>,
            test: impl Fn(char) -> bool,
        ) -> impl Iterator<Item = char> {
            iter::from_fn(move || {
                if chars.peek().filter(|c| test(**c)).is_some() {
                    return Some(chars.next().expect("peek at valid char"));
                }

                None
            })
        }

        Ok(TokenBuffer {
            buffer: iter::from_fn(move || {
                match chars.next()? {
                    c @ ('a'..='z' | 'A'..='Z' | '_') => {
                        // Consume whilst valid.
                        let ident = iter::once(c)
                            .chain(take_while(&mut chars, |c| c.is_alphanumeric() || c == '_'))
                            .collect::<String>();

                        // Attempt to build ident token.
                        let ident = match Ident::try_from(ident) {
                            Ok(ident) => ident,
                            Err(e) => return Some(Some(Err(e))),
                        };

                        Some(Some(Ok(ident.into())))
                    }
                    c if c.is_ascii_punctuation() => {
                        // Consume until non-punctuation.
                        let punct = iter::once(c)
                            .chain(take_while(&mut chars, |c| c.is_ascii_punctuation()))
                            .collect::<String>();

                        // Attempt to build the punctuation token.
                        let punct = match Punct::try_from(punct) {
                            Ok(punct) => punct,
                            Err(e) => return Some(Some(Err(e))),
                        };

                        Some(Some(Ok(punct.into())))
                    }
                    c if c.is_whitespace() => Some(None),
                    c => Some(Some(Err(format!("unknown character: {c}")))),
                }
            })
            .flatten()
            .collect::<Result<Vec<_>, _>>()?
            .into_boxed_slice(),
        })
    }

    /// Create a new stream to operate on this token buffer.
    pub fn stream(&self) -> ParseBuffer {
        ParseBuffer::new(self.cursor())
    }

    /// Create a new cursor into this buffer.
    pub fn cursor(&self) -> Cursor {
        Cursor::new(self)
    }
}

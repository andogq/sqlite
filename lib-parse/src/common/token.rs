use std::iter::{self, Peekable};

use derive_more::{Deref, From};

use crate::{
    buffer::{BufferToken, Cursor, Outcome},
    parse::{BufferParser, Parse, Token, token::TokenRepr},
};

/// An identifier. Can begin with any letter or an underscore, and can contain any letter, number,
/// or underscore.
#[derive(Clone, Debug, Deref, PartialEq)]
pub struct Ident(String);

impl Ident {
    fn new(ident: impl ToString) -> Self {
        Self(ident.to_string())
    }
}

impl<S: ?Sized + AsRef<str>> PartialEq<S> for Ident {
    fn eq(&self, other: &S) -> bool {
        self.0 == other.as_ref()
    }
}

impl Parse<CommonToken> for Ident {
    fn parse(parser: BufferParser<'_, CommonToken>) -> Result<Self, String> {
        match parser.parse()? {
            CommonToken::Ident(ident) => Ok(ident),
            _ => Err("unexpected token (expected ident)".into()),
        }
    }
}

impl Token<CommonToken> for Ident {
    fn peek(cursor: Cursor<'_, CommonToken>) -> bool {
        let Some((token, _)) = cursor.token() else {
            return false;
        };

        matches!(token, CommonToken::Ident(_))
    }

    fn display() -> &'static str {
        "idententifier"
    }
}

impl TokenRepr<CommonToken> for Ident {
    fn from_base(base: CommonToken) -> Option<Self> {
        match base {
            CommonToken::Ident(ident) => Some(ident),
            _ => None,
        }
    }
}

/// A punctuation symbol.
#[derive(Clone, Debug, PartialEq)]
pub enum Punct {
    Asterisk,
    Comma,
    Semicolon,
}

impl<S: ?Sized + AsRef<str>> PartialEq<S> for Punct {
    fn eq(&self, other: &S) -> bool {
        let c = match self {
            Punct::Asterisk => "*",
            Punct::Comma => ",",
            Punct::Semicolon => ";",
        };

        c == other.as_ref()
    }
}

impl Parse<CommonToken> for Punct {
    fn parse(parser: BufferParser<'_, CommonToken>) -> Result<Self, String> {
        match parser.parse()? {
            CommonToken::Punct(punct) => Ok(punct),
            _ => Err("unexpected token (expected punct)".into()),
        }
    }
}

impl Token<CommonToken> for Punct {
    fn peek(cursor: Cursor<'_, CommonToken>) -> bool {
        let Some((token, _)) = cursor.token() else {
            return false;
        };

        matches!(token, CommonToken::Punct(_))
    }

    fn display() -> &'static str {
        "punctuation"
    }
}

impl TokenRepr<CommonToken> for Punct {
    fn from_base(base: CommonToken) -> Option<Self> {
        match base {
            CommonToken::Punct(punct) => Some(punct),
            _ => None,
        }
    }
}

/// A token comprising of an identifier, or a piece of punctuation. Any whitespace encountered will
/// be ignored.
#[derive(Clone, Debug, From, PartialEq)]
pub enum CommonToken {
    Ident(Ident),
    Punct(Punct),
}

impl BufferToken for CommonToken {
    fn from_char(c: char, chars: &mut Peekable<impl Iterator<Item = char>>) -> Outcome<Self> {
        match c {
            c @ ('a'..='z' | 'A'..='Z' | '_') => {
                let ident = iter::once(c)
                    .chain(crate::util::take_while(chars, |c| {
                        c.is_alphanumeric() || *c == '_'
                    }))
                    .collect::<String>();

                Outcome::Token(Ident::new(ident).into())
            }
            c if c.is_ascii_punctuation() => Outcome::Token(
                match c {
                    '*' => Punct::Asterisk,
                    ',' => Punct::Comma,
                    ';' => Punct::Semicolon,
                    _ => return Outcome::Unexpected,
                }
                .into(),
            ),
            c if c.is_whitespace() => Outcome::Skip,
            _ => Outcome::Unexpected,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use rstest::rstest;

    /// Turn the provided [`str`] into the required parameters for [`BufferToken::from_char`].
    fn prepare(s: &'static str) -> (char, Peekable<impl Iterator<Item = char>>) {
        let mut chars = s.chars().peekable();
        (chars.next().expect("at least one char"), chars)
    }

    /// Parse a token from the string, and assert that it's successfully produced.
    fn parse_token<T: BufferToken>(s: &'static str) -> T {
        let (c, mut chars) = prepare(s);
        match T::from_char(c, &mut chars) {
            Outcome::Token(token) => token,
            Outcome::Unexpected => panic!("expected `Outcome::Token`, found `Outcome::Unexpected`"),
            Outcome::Skip => panic!("expected `Outcome::Token`, found `Outcome::Skip`"),
        }
    }

    /// Attempt to parse a token from the string, and assert that [`Outcome::Unexpected`] is
    /// produced.
    fn parse_unexpected<T: BufferToken>(s: &'static str) {
        let (c, mut chars) = prepare(s);
        assert!(matches!(T::from_char(c, &mut chars), Outcome::Unexpected));
    }

    /// Attempt to parse a token from the string, and assert that [`Outcome::Skip`] is produced.
    fn parse_skip<T: BufferToken>(s: &'static str) {
        let (c, mut chars) = prepare(s);
        assert!(matches!(T::from_char(c, &mut chars), Outcome::Skip));
    }

    mod common_token {
        use super::*;

        #[rstest]
        #[case("a", Ident::new("a").into())]
        #[case("_", Ident::new("_").into())]
        #[case("abc", Ident::new("abc").into())]
        #[case("_abc", Ident::new("_abc").into())]
        #[case("abc_abc", Ident::new("abc_abc").into())]
        #[case("abc123", Ident::new("abc123").into())]
        #[case("*", Punct::Asterisk.into())]
        #[case(",", Punct::Comma.into())]
        #[case(";", Punct::Semicolon.into())]
        fn valid(#[case] token: &'static str, #[case] expected: CommonToken) {
            let token = parse_token::<CommonToken>(token);
            assert_eq!(token, expected);
        }

        #[rstest]
        #[case("!")]
        #[case("1")]
        #[case("1abc")]
        #[case("!abc")]
        fn unexpected(#[case] token: &'static str) {
            parse_unexpected::<CommonToken>(token);
        }

        #[rstest]
        #[case(" ")]
        #[case("\t")]
        #[case("\n")]
        #[case(" abc")]
        fn skip(#[case] token: &'static str) {
            parse_skip::<CommonToken>(token);
        }
    }
}

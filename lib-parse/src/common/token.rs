use std::iter::{self, Peekable};

use derive_more::{Deref, From};

use crate::buffer::{BufferToken, Outcome};

/// An identifier. Can begin with any letter or an underscore, and can contain any letter, number,
/// or underscore.
#[derive(Clone, Debug, Deref, PartialEq)]
pub struct Ident(String);

impl Ident {
    fn new(ident: impl ToString) -> Self {
        Self(ident.to_string())
    }
}

impl BufferToken for Ident {
    fn from_char(c: char, chars: &mut Peekable<impl Iterator<Item = char>>) -> Outcome<Self> {
        if !(c.is_alphabetic() || c == '_') {
            return Outcome::Unexpected;
        }

        let ident = iter::once(c)
            .chain(crate::util::take_while(chars, |c| {
                c.is_alphanumeric() || *c == '_'
            }))
            .collect::<String>();

        Outcome::Token(Self(ident))
    }
}

/// A punctuation symbol.
#[derive(Clone, Debug, PartialEq)]
pub enum Punct {
    Asterisk,
    Comma,
    Semicolon,
}

impl BufferToken for Punct {
    fn from_char(c: char, _chars: &mut Peekable<impl Iterator<Item = char>>) -> Outcome<Self> {
        Outcome::Token(match c {
            '*' => Self::Asterisk,
            ',' => Self::Comma,
            ';' => Self::Semicolon,
            _ => return Outcome::Unexpected,
        })
    }
}

/// Combination of all common tokens.
#[derive(Clone, Debug, From, PartialEq)]
pub enum CommonToken {
    Ident(Ident),
    Punct(Punct),
}

impl BufferToken for CommonToken {
    fn from_char(c: char, chars: &mut Peekable<impl Iterator<Item = char>>) -> Outcome<Self> {
        match c {
            c @ ('a'..='z' | 'A'..='Z' | '_') => Ident::from_char(c, chars).map(CommonToken::Ident),
            c if c.is_ascii_punctuation() => Punct::from_char(c, chars).map(CommonToken::Punct),
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

    mod ident {
        use super::*;

        #[rstest]
        #[case("a")]
        #[case("_")]
        #[case("abc")]
        #[case("_abc")]
        #[case("abc_abc")]
        #[case("abc123")]
        fn valid(#[case] ident: &'static str) {
            let token = parse_token::<Ident>(ident);
            assert_eq!(*token, ident);
        }

        #[rstest]
        #[case("1")]
        #[case("!")]
        #[case("1abc")]
        #[case("!abc")]
        #[case(" abc")]
        fn unexpected(#[case] ident: &'static str) {
            parse_unexpected::<Ident>(ident);
        }
    }

    mod punct {
        use super::*;

        #[rstest]
        #[case("*", Punct::Asterisk)]
        #[case(",", Punct::Comma)]
        #[case(";", Punct::Semicolon)]
        fn valid(#[case] punct: &'static str, #[case] expected: Punct) {
            let token = parse_token::<Punct>(punct);
            assert_eq!(token, expected);
        }

        #[rstest]
        #[case(" ")]
        #[case("1")]
        #[case("a")]
        #[case("a*")]
        fn unexpected(#[case] punct: &'static str) {
            parse_unexpected::<Punct>(punct);
        }
    }

    mod common_token {
        use super::*;

        #[rstest]
        #[case("abc", Ident::new("abc").into())]
        #[case("_abc", Ident::new("_abc").into())]
        #[case("abc123", Ident::new("abc123").into())]
        #[case("*", Punct::Asterisk.into())]
        fn valid(#[case] token: &'static str, #[case] expected: CommonToken) {
            let token = parse_token::<CommonToken>(token);
            assert_eq!(token, expected);
        }

        #[rstest]
        #[case("!")]
        #[case("123")]
        fn unexpected(#[case] token: &'static str) {
            parse_unexpected::<CommonToken>(token);
        }

        #[rstest]
        #[case(" ")]
        #[case("\t")]
        #[case("\n")]
        fn skip(#[case] token: &'static str) {
            parse_skip::<CommonToken>(token);
        }
    }
}

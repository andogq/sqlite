use crate::{BufferParser, Cursor, Parse, Token, TokenRepr, parse::Delimiter};

use super::token::{CommonToken, Punct};

#[derive(Clone, Copy, Debug)]
pub struct Parenthesis;
impl Delimiter<CommonToken> for Parenthesis {
    type Left = LeftParenthesis;
    type Right = RightParenthesis;

    fn new(_left: Self::Left, _right: Self::Right) -> Self {
        Self
    }
}

pub struct LeftParenthesis;
impl Parse<CommonToken> for LeftParenthesis {
    fn parse(parser: BufferParser<'_, CommonToken>) -> Result<Self, String> {
        match parser.parse()? {
            Punct::LeftSmooth => Ok(LeftParenthesis),
            _ => Err("expected `(`".into()),
        }
    }
}
impl Token<CommonToken> for LeftParenthesis {
    fn peek(cursor: Cursor<'_, CommonToken>) -> bool {
        let Some((token, _)) = cursor.token() else {
            return false;
        };

        let Some(punct) = Punct::from_base(token) else {
            return false;
        };

        matches!(punct, Punct::LeftSmooth)
    }

    fn display() -> &'static str {
        "("
    }
}

pub struct RightParenthesis;
impl Parse<CommonToken> for RightParenthesis {
    fn parse(parser: BufferParser<'_, CommonToken>) -> Result<Self, String> {
        match parser.parse()? {
            Punct::RightSmooth => Ok(RightParenthesis),
            _ => Err("expected `)`".into()),
        }
    }
}
impl Token<CommonToken> for RightParenthesis {
    fn peek(cursor: Cursor<'_, CommonToken>) -> bool {
        let Some((token, _)) = cursor.token() else {
            return false;
        };

        let Some(punct) = Punct::from_base(token) else {
            return false;
        };

        matches!(punct, Punct::RightSmooth)
    }

    fn display() -> &'static str {
        ")"
    }
}

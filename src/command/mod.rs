mod token;

use lib_parse::{common::token::*, prelude::*};

use self::token::*;

#[derive(Clone, Debug)]
pub enum ResultColumn {
    All(Token![*]),
    Column(Ident),
}

impl Parse<CommonToken> for ResultColumn {
    fn parse(input: BufferParser<'_, CommonToken>) -> Result<Self, String> {
        let mut lookahead = input.lookahead();

        if lookahead.peek::<Token![*]>() {
            Ok(Self::All(input.parse()?))
        } else if lookahead.peek::<Ident>() {
            Ok(Self::Column(input.parse()?))
        } else {
            Err(lookahead.error())
        }
    }
}

#[derive(Clone, Debug)]
pub struct QueryStatement {
    select: Token![select],
    pub result_column: Punctuated<ResultColumn, Token![,]>,
    from: Token![from],
    pub table_name: Ident,
    semicolon: Token![;],
}

impl Parse<CommonToken> for QueryStatement {
    fn parse(input: BufferParser<'_, CommonToken>) -> Result<Self, String> {
        Ok(Self {
            select: input.parse()?,
            result_column: input.parse_with(Punctuated::parse_separated_non_empty)?,
            from: input.parse()?,
            table_name: input.parse()?,
            semicolon: input.parse()?,
        })
    }
}

pub fn parse_command(command: &str) -> QueryStatement {
    lib_parse::parse_str::<QueryStatement, CommonToken>(command).unwrap()
}

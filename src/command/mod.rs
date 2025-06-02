mod token;

use lib_parse::{
    common::token::{CommonToken, Ident},
    parse::{BufferParser, Parse, punctuated::Punctuated},
};

use self::token::*;

#[derive(Clone, Debug)]
enum ResultColumn {
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
struct QueryStatement {
    select: Token![select],
    result_column: Punctuated<ResultColumn, Token![,]>,
    from: Token![from],
    table_name: Ident,
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

pub fn do_something() {
    let command = "select some_column, another_column from some_table;";

    let statement =
        lib_parse::parse::entrypoint::parse_str::<QueryStatement, CommonToken>(command).unwrap();

    dbg!(statement);
}

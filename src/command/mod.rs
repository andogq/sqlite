mod base;
mod parse;
mod token;

use self::{
    base::{Ident, TokenBuffer},
    parse::{Parse, ParseStream},
};

use crate::Token;

#[derive(Clone, Debug)]
enum ResultColumn {
    All(Token![*]),
    Column(
        // TODO: Punctuated
        Ident,
    ),
}

impl Parse for ResultColumn {
    fn parse(input: &mut ParseStream) -> Result<Self, String> {
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
    // TODO: Punctuated
    result_column: ResultColumn,
    from: Token![from],
    table_name: Ident,
    semicolon: Token![;],
}

impl Parse for QueryStatement {
    fn parse(input: &mut ParseStream) -> Result<Self, String> {
        Ok(Self {
            select: input.parse()?,
            result_column: input.parse()?,
            from: input.parse()?,
            table_name: input.parse()?,
            semicolon: input.parse()?,
        })
    }
}

pub fn do_something() {
    let command = "select some_column from some_table;";

    let buffer = TokenBuffer::new(command).unwrap();
    let mut input = buffer.stream();

    let statement = QueryStatement::parse(&mut input).unwrap();
    dbg!(statement);
}

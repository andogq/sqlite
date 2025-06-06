mod token;

use lib_parse::{
    common::{delimiter::Parenthesis, token::*},
    parse::FullBufferParser,
    prelude::*,
};

use self::token::*;

#[allow(unused)]
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

#[allow(unused)]
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

pub fn parse_command<T: Parse<CommonToken>>(command: &str) -> T {
    lib_parse::parse_str(command).unwrap()
}

#[derive(Clone, Debug)]
pub struct ColumnDef {
    pub column_name: Ident,
    pub type_name: Ident,
    pub not_null: bool,
}

impl Parse<CommonToken> for ColumnDef {
    fn parse(parser: BufferParser<'_, CommonToken>) -> Result<Self, String> {
        Ok(Self {
            column_name: parser.parse()?,
            type_name: parser.parse()?,
            not_null: {
                let mut look = parser.lookahead();

                if look.peek::<Token![not]>() {
                    parser.parse::<Token![not]>()?;
                    parser.parse::<Token![null]>()?;

                    true
                } else {
                    false
                }
            },
        })
    }
}

#[derive(Clone, Debug)]
pub struct CreateStatement {
    create: Token![create],
    table: Token![table],
    pub table_name: Ident,
    pub columns: Punctuated<ColumnDef, Token![,]>,
}

impl Parse<CommonToken> for CreateStatement {
    fn parse(parser: BufferParser<'_, CommonToken>) -> Result<Self, String> {
        Ok(Self {
            create: parser.parse()?,
            table: parser.parse()?,
            table_name: parser.parse()?,
            columns: {
                let (_parens, group) = parser.group::<Parenthesis>()?;

                group.parse_with(Punctuated::parse_terminated)?
            },
        })
    }
}

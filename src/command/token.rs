//! Tokens built into the SQL language.

use super::parse::{Cursor, Parse, ParseStream, Peek, Token};

fn keyword_impl<'b>(cursor: Cursor<'b>, token: &str) -> Option<((), Cursor<'b>)> {
    let (_, rest) = cursor.ident().filter(|(ident, _)| ident == token)?;
    Some(((), rest))
}

fn parse_keyword(input: &mut ParseStream, token: &str) -> Result<(), String> {
    input.step(|cursor| keyword_impl(cursor, token).ok_or_else(|| format!("expected `{token}`")))
}

fn peek_keyword(cursor: Cursor, token: &str) -> bool {
    keyword_impl(cursor, token).is_some()
}

fn punct_impl<'b>(mut cursor: Cursor<'b>, token: &str) -> Option<((), Cursor<'b>)> {
    assert_eq!(
        token.len(),
        1,
        "only single character punctuation currently supported"
    );

    for c in token.chars() {
        cursor = cursor
            .punct()
            .filter(|(punct, _)| punct == c)
            .map(|(_, cursor)| cursor)?;
    }

    Some(((), cursor))
}

fn parse_punct(input: &mut ParseStream, token: &str) -> Result<(), String> {
    input.step(|cursor| punct_impl(cursor, token).ok_or_else(|| format!("expected `{token}`")))
}

fn peek_punct(cursor: Cursor, token: &str) -> bool {
    punct_impl(cursor, token).is_some()
}

macro_rules! define_keywords {
    ($($token:literal pub struct $name:ident)*) => {
        $(
            #[derive(Clone, Copy, Debug, Eq, PartialEq)]
            pub struct $name;

            impl Parse for $name {
                fn parse(input: &mut ParseStream) -> Result<Self, String> {
                    parse_keyword(input, $token)?;
                    Ok(Self)
                }
            }

            impl Token for $name {
                fn peek(cursor: Cursor) -> bool {
                    peek_keyword(cursor, $token)
                }

                fn display() -> &'static str {
                    $token
                }
            }

            impl Peek for $name {
                type Token = $name;
            }
        )*

        pub fn is_keyword(s: &str) -> bool {
            match s {
                $($token)|* => true,
                _ => false,
            }
        }
    };
}

macro_rules! define_punctuation {
    ($($token:literal pub struct $name:ident)*) => {
        $(
            #[derive(Clone, Copy, Debug, Eq, PartialEq)]
            pub struct $name;

            impl Parse for $name {
                fn parse(input: &mut ParseStream) -> Result<Self, String> {
                    parse_punct(input, $token)?;
                    Ok(Self)
                }
            }

            impl Token for $name {
                fn peek(cursor: Cursor) -> bool {
                    peek_punct(cursor, $token)
                }

                fn display() -> &'static str {
                    $token
                }
            }

            impl Peek for $name {
                type Token = $name;
            }
        )*
    }
}

define_keywords! {
    "abort"             pub struct Abort
    "action"            pub struct Action
    "add"               pub struct Add
    "after"             pub struct After
    "all"               pub struct All
    "alter"             pub struct Alter
    "always"            pub struct Always
    "analyze"           pub struct Analyze
    "and"               pub struct And
    "as"                pub struct As
    "asc"               pub struct Asc
    "attach"            pub struct Attach
    "autoincrement"     pub struct Autoincrement
    "before"            pub struct Before
    "begin"             pub struct Begin
    "between"           pub struct Between
    "by"                pub struct By
    "cascade"           pub struct Cascade
    "case"              pub struct Case
    "cast"              pub struct Cast
    "check"             pub struct Check
    "collate"           pub struct Collate
    "column"            pub struct Column
    "commit"            pub struct Commit
    "conflict"          pub struct Conflict
    "constraint"        pub struct Constraint
    "create"            pub struct Create
    "cross"             pub struct Cross
    "current"           pub struct Current
    "current_date"      pub struct CurrentDate
    "current_time"      pub struct CurrentTime
    "current_timestamp" pub struct CurrentTimestamp
    "database"          pub struct Database
    "default"           pub struct Default
    "deferrable"        pub struct Deferrable
    "deferred"          pub struct Deferred
    "delete"            pub struct Delete
    "desc"              pub struct Desc
    "detach"            pub struct Detach
    "distinct"          pub struct Distinct
    "do"                pub struct Do
    "drop"              pub struct Drop
    "each"              pub struct Each
    "else"              pub struct Else
    "end"               pub struct End
    "escape"            pub struct Escape
    "except"            pub struct Except
    "exclude"           pub struct Exclude
    "exclusive"         pub struct Exclusive
    "exists"            pub struct Exists
    "explain"           pub struct Explain
    "fail"              pub struct Fail
    "filter"            pub struct Filter
    "first"             pub struct First
    "following"         pub struct Following
    "for"               pub struct For
    "foreign"           pub struct Foreign
    "from"              pub struct From
    "full"              pub struct Full
    "generated"         pub struct Generated
    "glob"              pub struct Glob
    "group"             pub struct Group
    "groups"            pub struct Groups
    "having"            pub struct Having
    "if"                pub struct If
    "ignore"            pub struct Ignore
    "immediate"         pub struct Immediate
    "in"                pub struct In
    "index"             pub struct Index
    "indexed"           pub struct Indexed
    "initially"         pub struct Initially
    "inner"             pub struct Inner
    "insert"            pub struct Insert
    "instead"           pub struct Instead
    "intersect"         pub struct Intersect
    "into"              pub struct Into
    "is"                pub struct Is
    "isnull"            pub struct Isnull
    "join"              pub struct Join
    "key"               pub struct Key
    "last"              pub struct Last
    "left"              pub struct Left
    "like"              pub struct Like
    "limit"             pub struct Limit
    "match"             pub struct Match
    "materialized"      pub struct Materialized
    "natural"           pub struct Natural
    "no"                pub struct No
    "not"               pub struct Not
    "nothing"           pub struct Nothing
    "notnull"           pub struct Notnull
    "null"              pub struct Null
    "nulls"             pub struct Nulls
    "of"                pub struct Of
    "offset"            pub struct Offset
    "on"                pub struct On
    "or"                pub struct Or
    "order"             pub struct Order
    "others"            pub struct Others
    "outer"             pub struct Outer
    "over"              pub struct Over
    "partition"         pub struct Partition
    "plan"              pub struct Plan
    "pragma"            pub struct Pragma
    "preceding"         pub struct Preceding
    "primary"           pub struct Primary
    "query"             pub struct Query
    "raise"             pub struct Raise
    "range"             pub struct Range
    "recursive"         pub struct Recursive
    "references"        pub struct References
    "regexp"            pub struct Regexp
    "reindex"           pub struct Reindex
    "release"           pub struct Release
    "rename"            pub struct Rename
    "replace"           pub struct Replace
    "restrict"          pub struct Restrict
    "returning"         pub struct Returning
    "right"             pub struct Right
    "rollback"          pub struct Rollback
    "row"               pub struct Row
    "rows"              pub struct Rows
    "savepoint"         pub struct Savepoint
    "select"            pub struct Select
    "set"               pub struct Set
    "table"             pub struct Table
    "temp"              pub struct Temp
    "temporary"         pub struct Temporary
    "then"              pub struct Then
    "ties"              pub struct Ties
    "to"                pub struct To
    "transaction"       pub struct Transaction
    "trigger"           pub struct Trigger
    "unbounded"         pub struct Unbounded
    "union"             pub struct Union
    "unique"            pub struct Unique
    "update"            pub struct Update
    "using"             pub struct Using
    "vacuum"            pub struct Vacuum
    "values"            pub struct Values
    "view"              pub struct View
    "virtual"           pub struct Virtual
    "when"              pub struct When
    "where"             pub struct Where
    "window"            pub struct Window
    "with"              pub struct With
    "without"           pub struct Without
}

define_punctuation! {
    "*" pub struct Asterisk
    "," pub struct Comma
    ";" pub struct Semicolon
}

#[macro_export]
macro_rules! Token {
    [abort]             => { $crate::command::token::Abort };
    [action]            => { $crate::command::token::Action };
    [add]               => { $crate::command::token::Add };
    [after]             => { $crate::command::token::After };
    [all]               => { $crate::command::token::All };
    [alter]             => { $crate::command::token::Alter };
    [always]            => { $crate::command::token::Always };
    [analyze]           => { $crate::command::token::Analyze };
    [and]               => { $crate::command::token::And };
    [as]                => { $crate::command::token::As };
    [asc]               => { $crate::command::token::Asc };
    [attach]            => { $crate::command::token::Attach };
    [autoincrement]     => { $crate::command::token::Autoincrement };
    [before]            => { $crate::command::token::Before };
    [begin]             => { $crate::command::token::Begin };
    [between]           => { $crate::command::token::Between };
    [by]                => { $crate::command::token::By };
    [cascade]           => { $crate::command::token::Cascade };
    [case]              => { $crate::command::token::Case };
    [cast]              => { $crate::command::token::Cast };
    [check]             => { $crate::command::token::Check };
    [collate]           => { $crate::command::token::Collate };
    [column]            => { $crate::command::token::Column };
    [commit]            => { $crate::command::token::Commit };
    [conflict]          => { $crate::command::token::Conflict };
    [constraint]        => { $crate::command::token::Constraint };
    [create]            => { $crate::command::token::Create };
    [cross]             => { $crate::command::token::Cross };
    [current]           => { $crate::command::token::Current };
    [current_date]      => { $crate::command::token::CurrentDate };
    [current_time]      => { $crate::command::token::CurrentTime };
    [current_timestamp] => { $crate::command::token::CurrentTimestamp };
    [database]          => { $crate::command::token::Database };
    [default]           => { $crate::command::token::Default };
    [deferrable]        => { $crate::command::token::Deferrable };
    [deferred]          => { $crate::command::token::Deferred };
    [delete]            => { $crate::command::token::Delete };
    [desc]              => { $crate::command::token::Desc };
    [detach]            => { $crate::command::token::Detach };
    [distinct]          => { $crate::command::token::Distinct };
    [do]                => { $crate::command::token::Do };
    [drop]              => { $crate::command::token::Drop };
    [each]              => { $crate::command::token::Each };
    [else]              => { $crate::command::token::Else };
    [end]               => { $crate::command::token::End };
    [escape]            => { $crate::command::token::Escape };
    [except]            => { $crate::command::token::Except };
    [exclude]           => { $crate::command::token::Exclude };
    [exclusive]         => { $crate::command::token::Exclusive };
    [exists]            => { $crate::command::token::Exists };
    [explain]           => { $crate::command::token::Explain };
    [fail]              => { $crate::command::token::Fail };
    [filter]            => { $crate::command::token::Filter };
    [first]             => { $crate::command::token::First };
    [following]         => { $crate::command::token::Following };
    [for]               => { $crate::command::token::For };
    [foreign]           => { $crate::command::token::Foreign };
    [from]              => { $crate::command::token::From };
    [full]              => { $crate::command::token::Full };
    [generated]         => { $crate::command::token::Generated };
    [glob]              => { $crate::command::token::Glob };
    [group]             => { $crate::command::token::Group };
    [groups]            => { $crate::command::token::Groups };
    [having]            => { $crate::command::token::Having };
    [if]                => { $crate::command::token::If };
    [ignore]            => { $crate::command::token::Ignore };
    [immediate]         => { $crate::command::token::Immediate };
    [in]                => { $crate::command::token::In };
    [index]             => { $crate::command::token::Index };
    [indexed]           => { $crate::command::token::Indexed };
    [initially]         => { $crate::command::token::Initially };
    [inner]             => { $crate::command::token::Inner };
    [insert]            => { $crate::command::token::Insert };
    [instead]           => { $crate::command::token::Instead };
    [intersect]         => { $crate::command::token::Intersect };
    [into]              => { $crate::command::token::Into };
    [is]                => { $crate::command::token::Is };
    [isnull]            => { $crate::command::token::Isnull };
    [join]              => { $crate::command::token::Join };
    [key]               => { $crate::command::token::Key };
    [last]              => { $crate::command::token::Last };
    [left]              => { $crate::command::token::Left };
    [like]              => { $crate::command::token::Like };
    [limit]             => { $crate::command::token::Limit };
    [match]             => { $crate::command::token::Match };
    [materialized]      => { $crate::command::token::Materialized };
    [natural]           => { $crate::command::token::Natural };
    [no]                => { $crate::command::token::No };
    [not]               => { $crate::command::token::Not };
    [nothing]           => { $crate::command::token::Nothing };
    [notnull]           => { $crate::command::token::Notnull };
    [null]              => { $crate::command::token::Null };
    [nulls]             => { $crate::command::token::Nulls };
    [of]                => { $crate::command::token::Of };
    [offset]            => { $crate::command::token::Offset };
    [on]                => { $crate::command::token::On };
    [or]                => { $crate::command::token::Or };
    [order]             => { $crate::command::token::Order };
    [others]            => { $crate::command::token::Others };
    [outer]             => { $crate::command::token::Outer };
    [over]              => { $crate::command::token::Over };
    [partition]         => { $crate::command::token::Partition };
    [plan]              => { $crate::command::token::Plan };
    [pragma]            => { $crate::command::token::Pragma };
    [preceding]         => { $crate::command::token::Preceding };
    [primary]           => { $crate::command::token::Primary };
    [query]             => { $crate::command::token::Query };
    [raise]             => { $crate::command::token::Raise };
    [range]             => { $crate::command::token::Range };
    [recursive]         => { $crate::command::token::Recursive };
    [references]        => { $crate::command::token::References };
    [regexp]            => { $crate::command::token::Regexp };
    [reindex]           => { $crate::command::token::Reindex };
    [release]           => { $crate::command::token::Release };
    [rename]            => { $crate::command::token::Rename };
    [replace]           => { $crate::command::token::Replace };
    [restrict]          => { $crate::command::token::Restrict };
    [returning]         => { $crate::command::token::Returning };
    [right]             => { $crate::command::token::Right };
    [rollback]          => { $crate::command::token::Rollback };
    [row]               => { $crate::command::token::Row };
    [rows]              => { $crate::command::token::Rows };
    [savepoint]         => { $crate::command::token::Savepoint };
    [select]            => { $crate::command::token::Select };
    [set]               => { $crate::command::token::Set };
    [table]             => { $crate::command::token::Table };
    [temp]              => { $crate::command::token::Temp };
    [temporary]         => { $crate::command::token::Temporary };
    [then]              => { $crate::command::token::Then };
    [ties]              => { $crate::command::token::Ties };
    [to]                => { $crate::command::token::To };
    [transaction]       => { $crate::command::token::Transaction };
    [trigger]           => { $crate::command::token::Trigger };
    [unbounded]         => { $crate::command::token::Unbounded };
    [union]             => { $crate::command::token::Union };
    [unique]            => { $crate::command::token::Unique };
    [update]            => { $crate::command::token::Update };
    [using]             => { $crate::command::token::Using };
    [vacuum]            => { $crate::command::token::Vacuum };
    [values]            => { $crate::command::token::Values };
    [view]              => { $crate::command::token::View };
    [virtual]           => { $crate::command::token::Virtual };
    [when]              => { $crate::command::token::When };
    [where]             => { $crate::command::token::Where };
    [window]            => { $crate::command::token::Window };
    [with]              => { $crate::command::token::With };
    [without]           => { $crate::command::token::Without };
    [*]                 => { $crate::command::token::Asterisk };
    [,]                 => { $crate::command::token::Comma };
    [;]                 => { $crate::command::token::Semicolon };
}

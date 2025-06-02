/// Define sets of [`Tokens`] to be parsed from a [`TokenBuffer`]. These tokens directly correspond
/// to a static string.
///
/// The macro requires a type, which the tokens will derive from. This type can be the `BaseToken`
/// of the buffer, or another token that derives from it.
///
/// The macro also accepts an optional identifier in square brackets, which it will use as the name
/// of a function which will check if a string matches any of the tokens in a set (this is useful
/// for testing for keywords in identifiers, for example).
///
/// Finally the macro takes pairs of tokens and identifiers, where the token is the raw
/// representation, and the identifier corresponds to a struct which will represent it.
///
/// ```ignore
/// define_tokens! {
///     Ident [is_keyword] {
///         [for] For
///         [let] Let
///     }
/// }
/// ```
///
/// [`Tokens`]: crate::parse::Token
/// [`TokenBuffer`]: crate::buffer::TokenBuffer
#[macro_export]
macro_rules! define_tokens {
    ($($repr:ty $([$is_fn:ident])? { $($tokens:tt)* })*) => {
        $( $crate::define_tokens!([impl] => $repr $([$is_fn])? { $($tokens)* });)*

        $crate::define_tokens!([token_macro] => { $($($tokens)*)* });
    };

    ([impl] => $repr:ty $([$is_fn:ident])? { $([$token:tt] $name:ident)* }) => {
        $(
            #[doc = concat!("Token corresponding to `", stringify!($token), "`.")]
            #[doc = concat!("Reference type with `Token![", stringify!($token), "]` instead.")]
            #[derive(::std::clone::Clone, ::std::marker::Copy, ::std::fmt::Debug, ::std::cmp::Eq, ::std::cmp::PartialEq)]
            pub struct $name;

            impl $name {
                const TOKEN: &'static str = ::std::stringify!($token);
            }

            impl<BaseToken> $crate::parse::Parse<BaseToken> for $name
            where
                for<'s> $repr: $crate::parse::Parse<BaseToken> + ::std::cmp::PartialEq<&'s str>
            {
                fn parse(parser: $crate::parse::BufferParser<'_, BaseToken>) -> Result<Self, String> {
                    let repr = parser.parse::<$repr>()?;
                    if repr == Self::TOKEN {
                        ::std::result::Result::Ok($name)
                    } else {
                        ::std::result::Result::Err(format!("expected `{}`", Self::TOKEN))
                    }
                }
            }

            impl<BaseToken> $crate::parse::Token<BaseToken> for $name
            where
                $repr: $crate::parse::token::TokenRepr<BaseToken>,
                BaseToken: ::std::clone::Clone
            {
                fn peek(cursor: $crate::buffer::Cursor<'_, BaseToken>) -> bool {
                    let Some((base, _)) = cursor.token() else {
                        return false;
                    };

                    <$repr as $crate::parse::token::TokenRepr<BaseToken>>::from_base(base).is_some()
                }

                fn display() -> &'static str {
                    Self::TOKEN
                }
            }
        )*

        $crate::define_tokens!([is_fn] => $($is_fn)? { $($token)* });
    };

    ([is_fn] => { $($token:tt)* }) => {};

    ([is_fn] => $is_fn:ident { $($token:tt)* }) => {
        pub fn $is_fn(s: &str) -> bool {
            match s {
                $(stringify!($token))|* => true,
                _ => false,
            }
        }
    };

    ([token_macro] => { $([$token:tt] $name:ident)* }) => {
        #[macro_export]
        macro_rules! _Token {
            // Include empty rule so empty tokens doesn't cause error.
            () => {};

            $([$token] => { $name };)*
        }

        // Hack to work around exporting generated macros:
        // https://github.com/rust-lang/rust/pull/52234#issuecomment-1417098097
        #[doc(hidden)]
        pub use _Token as Token;
    };
}

/// Helper trait to allow for conversion from some `BaseToken` into a value used as a
/// representation for tokens. Any types used as representation in the [`define_tokens`] macro must
/// implement this trait, as it allows the macro to automatically generated certain method
/// implementations.
pub trait TokenRepr<BaseToken>: Sized {
    /// Create this token from a base token, or returning `None` if it fails.
    ///
    /// For most base tokens which are an enum, this will just a be a match statement.
    fn from_base(base: BaseToken) -> Option<Self>;
}

impl<T> TokenRepr<T> for T {
    fn from_base(base: T) -> Option<Self> {
        Some(base)
    }
}

#[cfg(test)]
mod test {
    use derive_more::From;

    use crate::{
        buffer::TokenBuffer,
        parse::{BufferParser, Parse},
    };

    use super::*;

    #[derive(Clone)]
    struct Ident(String);
    impl<S: ?Sized + AsRef<str>> PartialEq<S> for Ident {
        fn eq(&self, other: &S) -> bool {
            self.0 == other.as_ref()
        }
    }
    impl Parse<BaseToken> for Ident {
        fn parse(parser: BufferParser<'_, BaseToken>) -> Result<Self, String> {
            Self::from_base(parser.parse::<BaseToken>()?).ok_or_else(|| "expected `ident`".into())
        }
    }
    #[derive(Clone)]
    struct Symbol(String);
    impl<S: ?Sized + AsRef<str>> PartialEq<S> for Symbol {
        fn eq(&self, other: &S) -> bool {
            self.0 == other.as_ref()
        }
    }
    impl Parse<BaseToken> for Symbol {
        fn parse(parser: BufferParser<'_, BaseToken>) -> Result<Self, String> {
            Self::from_base(parser.parse::<BaseToken>()?).ok_or_else(|| "expected `symbol`".into())
        }
    }

    #[derive(Clone, From)]
    enum BaseToken {
        Ident(Ident),
        Symbol(Symbol),
    }
    impl TokenRepr<BaseToken> for Ident {
        fn from_base(base: BaseToken) -> Option<Self> {
            match base {
                BaseToken::Ident(ident) => Some(ident),
                _ => None,
            }
        }
    }
    impl TokenRepr<BaseToken> for Symbol {
        fn from_base(base: BaseToken) -> Option<Self> {
            match base {
                BaseToken::Symbol(symbol) => Some(symbol),
                _ => None,
            }
        }
    }

    define_tokens! {
        Ident [is_keyword] {
            [something] Something
            [another] Another
        }

        Symbol {
            [,] Comma
            [;] Semicolon
        }
    }

    #[test]
    fn parse_generated_tokens() {
        let buffer = TokenBuffer::<BaseToken>::new_with_tokens(vec![
            Ident("something".into()).into(),
            Ident("another".into()).into(),
            Symbol(",".into()).into(),
        ]);
        let parser = buffer.parser();

        let _something = parser.parse::<Something>().unwrap();
        let _another = parser.parse::<Another>().unwrap();
        let _comma = parser.parse::<Comma>().unwrap();
    }

    #[test]
    fn is_fn() {
        assert!(is_keyword("something"));
        assert!(!is_keyword("nothing"));
        assert!(!is_keyword(","));
    }
}

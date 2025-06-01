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

            // 1-deep nesting: parent token into child token
            impl $crate::buffer::IntoToken<$name> for $repr where for<'s> $repr: ::std::cmp::PartialEq<&'s str> {
                fn into_token(self) -> ::std::option::Option<$name> {
                    if self == $name::TOKEN {
                        ::std::option::Option::Some($name)
                    } else {
                        ::std::option::Option::None
                    }
                }
            }

            // 2-deep nesting: grand-parent token into child token (via parent)
            impl<BaseToken> $crate::buffer::IntoToken<$name> for BaseToken where BaseToken: IntoToken<$repr> {
                fn into_token(self) -> ::std::option::Option<$name> {
                    let parent = self.into_token()?;
                    parent.into_token()
                }
            }

            impl<BaseToken> $crate::parse::Token<BaseToken> for $name where BaseToken: ::std::clone::Clone + $crate::buffer::IntoToken<Self> {
                fn peek(cursor: $crate::buffer::Cursor<'_, BaseToken>) -> bool {
                    cursor.token::<$name>().is_some()
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
        macro_rules! Token {
            // Include empty rule so empty tokens doesn't cause error.
            () => {};

            $([$token] => { $name };)*
        }
    };
}

#[cfg(test)]
mod test {
    use derive_more::From;

    use crate::buffer::{IntoToken, TokenBuffer};

    #[derive(Clone)]
    struct Ident(String);
    impl<S: ?Sized + AsRef<str>> PartialEq<S> for Ident {
        fn eq(&self, other: &S) -> bool {
            self.0 == other.as_ref()
        }
    }
    #[derive(Clone)]
    struct Symbol(String);
    impl<S: ?Sized + AsRef<str>> PartialEq<S> for Symbol {
        fn eq(&self, other: &S) -> bool {
            self.0 == other.as_ref()
        }
    }

    #[derive(Clone, From)]
    enum BaseToken {
        Ident(Ident),
        Symbol(Symbol),
    }
    impl IntoToken<Ident> for BaseToken {
        fn into_token(self) -> Option<Ident> {
            match self {
                BaseToken::Ident(ident) => Some(ident),
                _ => None,
            }
        }
    }
    impl IntoToken<Symbol> for BaseToken {
        fn into_token(self) -> Option<Symbol> {
            match self {
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

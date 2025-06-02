use super::*;

/// A sequence of `T` separated by `P`. Both an empty sequence, and a trailing `P` are
/// representations, but whether they're accepted or not is determined by the parsing
/// implementation.
#[derive(Clone, Debug)]
pub struct Punctuated<T, P> {
    pairs: Vec<(T, P)>,
    last: Option<T>,
}

impl<T, P> Punctuated<T, P> {
    /// Create a new empty instance.
    pub fn new() -> Self {
        Self {
            pairs: Vec::new(),
            last: None,
        }
    }

    /// Parse `T` from the buffer, until the buffer is empty. An empty sequence and trailing `P`
    /// are both accepted.
    pub fn parse_terminated<BaseToken>(input: BufferParser<'_, BaseToken>) -> Result<Self, String>
    where
        T: Parse<BaseToken>,
        P: Parse<BaseToken>,
    {
        Self::parse_terminated_with(input, T::parse)
    }

    /// Parse with a function until the buffer is empty. See [`Self::parse_terminated`].
    pub fn parse_terminated_with<BaseToken>(
        input: BufferParser<'_, BaseToken>,
        parser: fn(BufferParser<'_, BaseToken>) -> Result<T, String>,
    ) -> Result<Self, String>
    where
        P: Parse<BaseToken>,
    {
        let mut punctuated = Self::new();

        loop {
            if input.is_empty() {
                break;
            }

            let value = parser(input)?;

            if input.is_empty() {
                punctuated.last = Some(value);
                break;
            }

            let punctuation = input.parse::<P>()?;
            punctuated.pairs.push((value, punctuation));
        }

        Ok(punctuated)
    }

    /// Parse a punctuated stream, stopping if there is no more `P` in the stream. An empty
    /// sequence or trailing punctuation is not allowed.
    pub fn parse_separated_non_empty<BaseToken>(
        input: BufferParser<'_, BaseToken>,
    ) -> Result<Self, String>
    where
        T: Parse<BaseToken>,
        P: Token<BaseToken> + Parse<BaseToken>,
    {
        Self::parse_separated_non_empty_with(input, T::parse)
    }

    /// Parse with a function until there is no more `P`. See [`Self::parse_separated_non_empty`].
    pub fn parse_separated_non_empty_with<BaseToken>(
        input: BufferParser<'_, BaseToken>,
        parser: fn(BufferParser<'_, BaseToken>) -> Result<T, String>,
    ) -> Result<Self, String>
    where
        P: Token<BaseToken> + Parse<BaseToken>,
    {
        let mut punctuated = Self::new();

        loop {
            let value = parser(input)?;

            if !P::peek(input.cursor()) {
                punctuated.last = Some(value);
                break;
            }

            let punct = input.parse()?;
            punctuated.pairs.push((value, punct));
        }

        Ok(punctuated)
    }

    pub fn len(&self) -> usize {
        self.pairs.len() + if self.last.is_some() { 1 } else { 0 }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<T, P> Default for Punctuated<T, P> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T, P> IntoIterator for Punctuated<T, P> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        let len = self.len();

        let mut vec = Vec::with_capacity(len);
        vec.extend(self.pairs.into_iter().map(|(value, _)| value));
        vec.extend(self.last);

        assert_eq!(vec.len(), len);

        vec.into_iter()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use rstest::*;

    #[derive(Clone)]
    pub enum BaseToken {
        Value,
        Delimiter,
        Other,
    }
    #[derive(Clone)]
    struct Value;
    impl Parse<BaseToken> for Value {
        fn parse(parser: BufferParser<'_, BaseToken>) -> Result<Self, String> {
            match parser.parse()? {
                BaseToken::Value => Ok(Value),
                _ => Err("expected `value`".into()),
            }
        }
    }
    #[derive(Clone)]
    struct Delimiter;
    impl Parse<BaseToken> for Delimiter {
        fn parse(parser: BufferParser<'_, BaseToken>) -> Result<Self, String> {
            match parser.parse()? {
                BaseToken::Delimiter => Ok(Delimiter),
                _ => Err("expected `delimiter`".into()),
            }
        }
    }
    impl Token<BaseToken> for Delimiter {
        fn peek(cursor: Cursor<'_, BaseToken>) -> bool {
            let Some((token, _)) = cursor.token() else {
                return false;
            };
            matches!(token, BaseToken::Delimiter)
        }

        fn display() -> &'static str {
            "delimiter"
        }
    }

    mod parse_terminated {
        use super::*;

        #[rstest]
        #[case(vec![], 0)]
        #[case(vec![BaseToken::Value], 1)]
        #[case(vec![BaseToken::Value, BaseToken::Delimiter], 1)]
        #[case(vec![BaseToken::Value, BaseToken::Delimiter, BaseToken::Value], 2)]
        #[case(vec![BaseToken::Value, BaseToken::Delimiter, BaseToken::Value, BaseToken::Delimiter], 2)]
        fn success(#[case] tokens: Vec<BaseToken>, #[case] expected_len: usize) {
            let buffer = TokenBuffer::new_with_tokens(tokens);
            let parser = buffer.parser();

            let result: Punctuated<Value, Delimiter> =
                parser.parse_with(Punctuated::parse_terminated).unwrap();
            assert_eq!(result.len(), expected_len);
        }

        #[rstest]
        #[case(vec![BaseToken::Delimiter])]
        #[case(vec![BaseToken::Value, BaseToken::Delimiter, BaseToken::Delimiter])]
        #[case(vec![BaseToken::Value, BaseToken::Value])]
        #[case(vec![BaseToken::Value, BaseToken::Delimiter, BaseToken::Other])]
        fn failure(#[case] tokens: Vec<BaseToken>) {
            let buffer = TokenBuffer::new_with_tokens(tokens);
            let parser = buffer.parser();

            assert!(
                parser
                    .parse_with(Punctuated::<Value, Delimiter>::parse_terminated)
                    .is_err()
            );
        }
    }

    mod parse_separated_non_empty {
        use super::*;

        #[rstest]
        #[case(vec![BaseToken::Value], 1, true)]
        #[case(vec![BaseToken::Value, BaseToken::Other], 1, false)]
        #[case(vec![BaseToken::Value, BaseToken::Delimiter, BaseToken::Value], 2, true)]
        #[case(vec![BaseToken::Value, BaseToken::Delimiter, BaseToken::Value, BaseToken::Other], 2, false)]
        #[case(vec![BaseToken::Value, BaseToken::Value], 1, false)]
        fn success(
            #[case] tokens: Vec<BaseToken>,
            #[case] expected_len: usize,
            #[case] expect_eof: bool,
        ) {
            let buffer = TokenBuffer::new_with_tokens(tokens);
            let parser = buffer.parser();

            let result: Punctuated<Value, Delimiter> = parser
                .parse_with(Punctuated::parse_separated_non_empty)
                .unwrap();
            assert_eq!(result.len(), expected_len);
            assert_eq!(parser.is_empty(), expect_eof);
        }

        #[rstest]
        #[case(vec![])]
        #[case(vec![BaseToken::Value, BaseToken::Delimiter])]
        #[case(vec![BaseToken::Value, BaseToken::Delimiter, BaseToken::Other])]
        #[case(vec![BaseToken::Value, BaseToken::Delimiter, BaseToken::Delimiter])]
        fn failure(#[case] tokens: Vec<BaseToken>) {
            let buffer = TokenBuffer::new_with_tokens(tokens);
            let parser = buffer.parser();

            assert!(
                parser
                    .parse_with(Punctuated::<Value, Delimiter>::parse_separated_non_empty)
                    .is_err()
            );
        }
    }
}

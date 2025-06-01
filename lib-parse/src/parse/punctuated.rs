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
}

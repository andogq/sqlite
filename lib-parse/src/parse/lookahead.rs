use super::*;

/// Utility for determining the next token at the cursor. It will track which tokens have been
/// attempted, in order to automatically construct an error message if they all fail.
///
/// This cursor will not advance any further.
pub struct Lookahead<'b, BaseToken> {
    /// Cursor to undertake lookahead from.
    cursor: Cursor<'b, BaseToken>,
    /// All comparisons which have been attempted on this lookahead.
    comparisons: Vec<&'static str>,
}

impl<'b, BaseToken> Lookahead<'b, BaseToken> {
    /// Create a new instance with the provided cursor.
    pub(crate) fn new(cursor: Cursor<'b, BaseToken>) -> Self {
        Self {
            cursor,
            comparisons: Vec::new(),
        }
    }

    /// Peek for a token, and record that this attempt was made.
    pub fn peek<T: Token<BaseToken>>(&mut self) -> bool {
        if T::peek(self.cursor) {
            return true;
        }

        self.comparisons.push(T::display());
        false
    }

    /// Consume this instance and create an error message containing all peek attempts.
    pub fn error(self) -> String {
        match self.comparisons.len() {
            0 => {
                if self.cursor.eof() {
                    "unexpected end of input".into()
                } else {
                    "unexpected token".into()
                }
            }
            1 => {
                format!("expected {}", self.comparisons[0])
            }
            2 => {
                format!(
                    "expected {} or {}",
                    self.comparisons[0], self.comparisons[1]
                )
            }
            _ => {
                format!("expected one of: {}", self.comparisons.join(", "))
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[derive(Clone)]
    struct SomeToken;
    impl Token<SomeToken> for SomeToken {
        fn peek(cursor: Cursor<'_, SomeToken>) -> bool {
            cursor.token().is_some()
        }

        fn display() -> &'static str {
            "some token"
        }
    }

    #[derive(Clone)]
    struct OtherToken;
    impl Token<SomeToken> for OtherToken {
        fn peek(cursor: Cursor<'_, SomeToken>) -> bool {
            // Still advance the cursor for the test, but ignore the result.
            cursor.token();

            false
        }

        fn display() -> &'static str {
            "other token"
        }
    }

    #[test]
    fn can_peek() {
        let buffer = TokenBuffer::new_with_tokens(vec![SomeToken]);
        let parser = buffer.parser();
        let mut lookahead = parser.lookahead();

        assert!(lookahead.peek::<SomeToken>());
        assert!(lookahead.comparisons.is_empty());
        // Lookahead shouldn't modify the token.
        assert!(!lookahead.cursor.eof());
        assert!(!parser.is_empty());
    }

    #[test]
    fn peek_empty_buffer() {
        let buffer = TokenBuffer::empty();
        let parser = buffer.parser();
        let mut lookahead = parser.lookahead();

        assert!(!lookahead.peek::<SomeToken>());
        assert_eq!(lookahead.comparisons.len(), 1);
        assert_eq!(lookahead.error(), format!("expected some token"));
    }

    #[test]
    fn cant_peek() {
        let buffer = TokenBuffer::new_with_tokens(vec![SomeToken]);
        let parser = buffer.parser();
        let mut lookahead = parser.lookahead();

        assert!(!lookahead.peek::<OtherToken>());
        assert_eq!(lookahead.comparisons.len(), 1);
        // Lookahead shouldn't modify the token.
        assert!(!lookahead.cursor.eof());
        assert_eq!(lookahead.error(), format!("expected other token"))
    }
}

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
    pub fn new(cursor: Cursor<'b, BaseToken>) -> Self {
        Self {
            cursor,
            comparisons: Vec::new(),
        }
    }

    /// Peek for a token, and record that this attempt was made.
    pub fn peek<T: Token<BaseToken>>(&mut self) -> bool {
        self.comparisons.push(T::display());
        T::peek(self.cursor)
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

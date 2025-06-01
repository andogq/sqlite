use std::iter::{self, Peekable};

/// Utility to continually consume items from an iterator whilst a condition is true, without
/// consuming the iterator.
pub fn take_while<T>(
    chars: &mut Peekable<impl Iterator<Item = T>>,
    test: impl Fn(&T) -> bool,
) -> impl Iterator<Item = T> {
    iter::from_fn(move || {
        if chars.peek().filter(|c| test(*c)).is_some() {
            return Some(chars.next().expect("peek at valid item"));
        }

        None
    })
}

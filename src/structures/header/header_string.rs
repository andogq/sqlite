use std::marker::PhantomData;

use thiserror::Error;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

use crate::structures::{Valid, Validate, ValidityMarker};

/// Header string within a SQLite header. Requires validation.
#[derive(Clone, Debug, FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned)]
#[repr(transparent)]
pub struct HeaderString<V: ValidityMarker = Valid>([u8; 16], PhantomData<fn() -> V>);

impl<V: ValidityMarker> Validate<V> for HeaderString<V> {
    type Valid = ();
    type Error = HeaderStringError;

    fn try_get(&self) -> Result<Self::Valid, Self::Error> {
        if self.0 != HeaderString::BYTES {
            return Err(HeaderStringError(self.0));
        }

        Ok(())
    }
}

impl HeaderString {
    /// Expected bytes present in the header.
    const BYTES: [u8; 16] = *b"SQLite format 3\0";
}

#[derive(Clone, Debug, Error)]
#[error("expected '{expected:#?}', but found '{0:#?}'", expected = HeaderString::BYTES)]
pub struct HeaderStringError([u8; 16]);

#[cfg(test)]
mod test {
    use crate::structures::Invalid;

    use super::*;

    #[test]
    fn valid_from_bytes() {
        let header_string = HeaderString::<Invalid>::ref_from_bytes(b"SQLite format 3\0").unwrap();
        assert!(header_string.try_get().is_ok());
    }

    #[test]
    fn invalid_with_wrong_bytes() {
        let header_string = HeaderString::<Invalid>::ref_from_bytes(b"invalid bytes!!\0").unwrap();
        assert!(header_string.try_get().is_err());
    }
}

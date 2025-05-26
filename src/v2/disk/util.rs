use thiserror::Error;
use zerocopy::{FromBytes, Immutable, IntoBytes, Unaligned};

/// A constant [`u8`] value. During verification, if the deserialised value doesn't match the
/// constant, then an error will be raised.
#[derive(Clone, Copy, Debug, IntoBytes, FromBytes, Immutable, Unaligned, PartialEq, Eq)]
#[repr(transparent)]
pub struct ConstU8<const N: u8>(u8);

impl<const N: u8> ConstU8<N> {
    pub const fn value() -> u8 {
        N
    }

    pub fn validate(&self) -> Result<u8, ConstU8Error> {
        if self.0 != N {
            return Err(ConstU8Error {
                expected: N,
                found: self.0,
            });
        }

        Ok(N)
    }
}

/// Error produced during [`ConstU8::validate`].
#[derive(Clone, Debug, Error)]
#[error("expected const u8 value {expected} (found {found})")]
pub struct ConstU8Error {
    /// Expected constant value.
    pub expected: u8,
    /// Value that was deserialised.
    pub found: u8,
}

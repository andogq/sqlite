use std::marker::PhantomData;

use thiserror::Error;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned, big_endian::U16};

use crate::structures::{Valid, Validate, ValidityMarker};

/// Size of each database page, in bytes.
#[derive(Clone, Debug, FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned)]
#[repr(transparent)]
pub struct PageSize<V: ValidityMarker = Valid>(U16, PhantomData<fn() -> V>);

impl<V: ValidityMarker> PageSize<V> {
    /// Minumum value of page size.
    const MIN: u32 = 512;
    /// Maximum encoded page size.
    const MAX: u32 = 32768;
    /// Page size of `1` encoded.
    const VALUE_FOR_1: u32 = 65536;

    /// Encode and set the page size.
    pub fn set(&mut self, page_size: u32) -> Result<(), PageSizeError> {
        let page_size = if page_size == Self::VALUE_FOR_1 {
            1
        } else {
            if !(Self::MIN..=Self::MAX).contains(&page_size) {
                return Err(PageSizeError::Range(page_size));
            }

            if !page_size.is_power_of_two() {
                return Err(PageSizeError::PowerOfTwo(page_size));
            }

            page_size as u16
        };

        self.0 = U16::new(page_size);

        Ok(())
    }
}

impl<V: ValidityMarker> Validate<V> for PageSize<V> {
    type Valid = u32;
    type Error = PageSizeError;

    fn try_get(&self) -> Result<Self::Valid, Self::Error> {
        let n = self.0.get() as u32;

        if n == 1 {
            return Ok(Self::VALUE_FOR_1);
        }

        if !(Self::MIN..=Self::MAX).contains(&n) {
            return Err(PageSizeError::Range(n));
        }

        if !n.is_power_of_two() {
            return Err(PageSizeError::PowerOfTwo(n));
        }

        Ok(n)
    }
}

#[derive(Clone, Debug, Error)]
pub enum PageSizeError {
    #[error("page size must be between {min} and {max}, or {end} (found {0})", min = PageSize::<Valid>::MIN, max = PageSize::<Valid>::MAX, end = PageSize::<Valid>::VALUE_FOR_1)]
    Range(u32),
    #[error("page size must be a power of two (found {0})")]
    PowerOfTwo(u32),
}

#[cfg(test)]
mod test {
    use rstest::rstest;

    use crate::structures::Invalid;

    use super::*;

    #[rstest]
    #[case::one(1, 65536)]
    #[case::min(512, 512)]
    #[case::max(32768, 32768)]
    #[case::power_of_two(4096, 4096)]
    fn decode_and_validate(#[case] raw: u16, #[case] expected: u32) {
        let bytes = raw.to_be_bytes();
        let page_size = PageSize::<Invalid>::ref_from_bytes(&bytes).unwrap();
        assert_eq!(page_size.try_get().unwrap(), expected);
    }

    #[rstest]
    #[case::zero(0)]
    #[case::below_min(511)]
    #[case::not_power_of_two(4095)]
    fn decode_invalid(#[case] raw: u16) {
        let bytes = raw.to_be_bytes();
        let page_size = PageSize::<Invalid>::ref_from_bytes(&bytes).unwrap();
        assert!(page_size.try_get().is_err());
    }

    #[rstest]
    #[case::one(65536, 1)]
    #[case::min(512, 512)]
    #[case::max(32768, 32768)]
    fn set(#[case] new_page_size: u32, #[case] expected: u16) {
        let mut page_size = PageSize::<Invalid>(U16::ZERO, PhantomData);
        page_size.set(new_page_size).unwrap();
        assert_eq!(page_size.0.get(), expected);
    }

    #[rstest]
    #[case::under_min(256)]
    #[case::not_power_of_two(1234)]
    fn set_invalid(#[case] new_page_size: u32) {
        let mut page_size = PageSize::<Invalid>(U16::ZERO, PhantomData);
        assert!(page_size.set(new_page_size).is_err());
    }
}

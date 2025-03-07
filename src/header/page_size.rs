use derive_more::Deref;
use thiserror::Error;

/// Minimum size of the SQLite page.
const PAGE_SIZE_MIN: u16 = 512;
/// Maximum size of the SQLite page.
const PAGE_SIZE_MAX: u16 = 32768;
/// Size of the SQLite page if the reported size is `1`.
const PAGE_SIZE_1: usize = 65536;

#[derive(Clone, Debug, Error)]
pub enum PageSizeError {
    #[error("page size must be between 512 and 32768 inclusive: {0}")]
    NotInRange(u16),
    #[error("page size must be a power of 2: {0}")]
    NotPowerOfTwo(u16),
}

/// Size of a database page.
#[derive(Clone, Copy, Debug, Deref)]
pub struct PageSize(usize);

impl TryFrom<u16> for PageSize {
    type Error = PageSizeError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if !(PAGE_SIZE_MIN..=PAGE_SIZE_MAX).contains(&value) && value != 1 {
            return Err(PageSizeError::NotInRange(value));
        }

        if !value.is_power_of_two() {
            return Err(PageSizeError::NotPowerOfTwo(value));
        }

        let value = if value == 1 {
            PAGE_SIZE_1
        } else {
            value as usize
        };

        Ok(Self(value))
    }
}

impl From<PageSize> for usize {
    fn from(size: PageSize) -> Self {
        size.0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use rstest::*;

    #[rstest]
    #[case::min(512)]
    #[case(1024)]
    #[case(2048)]
    #[case(16384)]
    #[case::max(32768)]
    fn good(#[case] raw: u16) {
        let page_size = PageSize::try_from(raw).expect("valid page size");
        assert_eq!(*page_size, raw as usize);
    }

    #[test]
    fn one() {
        let page_size = PageSize::try_from(1).expect("valid page size");
        assert_eq!(*page_size, 65536);
    }

    #[rstest]
    #[case::zero(0, |e| matches!(e, PageSizeError::NotInRange(_)))]
    #[case::small(256, |e| matches!(e, PageSizeError::NotInRange(_)))]
    #[case::big(65535, |e| matches!(e, PageSizeError::NotInRange(_)))]
    #[case::non_power(1234, |e| matches!(e, PageSizeError::NotPowerOfTwo(_)))]
    fn bad(#[case] raw: u16, #[case] matcher: fn(PageSizeError) -> bool) {
        let error = PageSize::try_from(raw).expect_err("should be error test");
        assert!(matcher(error));
    }
}

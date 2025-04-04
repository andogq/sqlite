use std::num::NonZero;

use cuisiner::{ByteBoolean, ByteOrder, ConstU8, Cuisiner, CuisinerError, Reserved};
use derive_more::Deref;
use zerocopy::{U16, U32};

/// Expected size of the SQLite header in bytes.
pub const SQLITE_HEADER_SIZE: usize = 100;

/// Header of a SQLite file.
#[derive(Clone, Cuisiner, Debug)]
#[cuisiner(assert(size = SQLITE_HEADER_SIZE))]
pub struct SqliteHeader {
    /// The header string.
    #[cuisiner(assert(offset = 0, size = 16))]
    pub header_string: HeaderString,
    /// Size of each page.
    #[cuisiner(assert(offset = 16, size = 2))]
    pub page_size: PageSize,
    /// File format write version.
    #[cuisiner(assert(offset = 18, size = 1))]
    pub file_format_write_version: FileFormatVersion,
    /// File format read version.
    #[cuisiner(assert(offset = 19, size = 1))]
    pub file_format_read_version: FileFormatVersion,
    /// Number of bytes for reserved space at the end of each page.
    #[cuisiner(assert(offset = 20, size = 1))]
    pub page_end_padding: Option<NonZero<u8>>,
    /// Maximum embedded payload fraction.
    #[cuisiner(assert(offset = 21, size = 1))]
    pub max_payload_fraction: ConstU8<64>,
    /// Minimum embedded payload fraction.
    #[cuisiner(assert(offset = 22, size = 1))]
    pub min_payload_fraction: ConstU8<32>,
    /// Leaf payload fraction.
    #[cuisiner(assert(offset = 23, size = 1))]
    pub leaf_payload_fraction: ConstU8<32>,
    /// File change counter.
    #[cuisiner(assert(offset = 24, size = 4))]
    pub file_change_counter: u32,
    /// Number of pages in the database.
    #[cuisiner(assert(offset = 28, size = 4))]
    pub page_count: u32,
    /// Page number of the first freelist trunk page.
    #[cuisiner(assert(offset = 32, size = 4))]
    pub freelist_trunk_page: u32,
    /// Total number of freelist pages.
    #[cuisiner(assert(offset = 36, size = 4))]
    pub freelist_page_count: u32,
    /// Schema cookie.
    #[cuisiner(assert(offset = 40, size = 4))]
    pub schema_cookie: u32,
    /// Schema format number.
    #[cuisiner(assert(offset = 44, size = 4))]
    pub schema_format: SchemaFormat,
    /// Default page cache size.
    #[cuisiner(assert(offset = 48, size = 4))]
    pub default_page_cache_size: u32,
    /// Page number of the largest root b-tree page. Will be [`None`] if not in auto-vacuum or
    /// incremental-vacuum modes.
    #[cuisiner(assert(offset = 52, size = 4))]
    pub largest_root_btree_page: Option<NonZero<u32>>,
    /// Text encoding.
    #[cuisiner(assert(offset = 56, size = 4))]
    pub text_encoding: TextEncoding,
    /// User version as per `user_version_pragma`.
    #[cuisiner(assert(offset = 60, size = 4))]
    pub user_version: u32,
    /// `true` for incremental-vacuum mode.
    #[cuisiner(assert(offset = 64, size = 4))]
    pub incremental_vacuum_mode: ByteBoolean<4>,
    /// Application ID as per `PRAGMA application_id`.
    #[cuisiner(assert(offset = 68, size = 4))]
    pub application_id: u32,
    /// Reserved for expansion.
    #[cuisiner(assert(offset = 72, size = 20))]
    _reserved: Reserved<20>,
    /// `version-valid-for` number.
    #[cuisiner(assert(offset = 92, size = 4))]
    pub version_valid_for: u32,
    /// SQLite version number.
    #[cuisiner(assert(offset = 96, size = 4))]
    pub sqlite_version_number: SqliteVersionNumber,
}

#[derive(Clone, Debug)]
pub struct HeaderString;
impl HeaderString {
    const BYTES: [u8; 16] = *b"SQLite format 3\0";
}
impl Cuisiner for HeaderString {
    type Raw<B: ByteOrder> = [u8; 16];

    fn try_from_raw<B: ByteOrder>(raw: Self::Raw<B>) -> Result<Self, CuisinerError> {
        if raw != Self::BYTES {
            return Err(CuisinerError::Validation(format!(
                "invalid header string: {raw:?}"
            )));
        }

        Ok(Self)
    }

    fn try_to_raw<B: ByteOrder>(self) -> Result<Self::Raw<B>, CuisinerError> {
        Ok(Self::BYTES)
    }
}

#[derive(Clone, Debug, Deref)]
pub struct PageSize(u32);
impl PageSize {
    /// Minumum value of page size.
    const MIN: u32 = 512;
    /// Maximum encoded page size.
    const MAX: u32 = 32768;
    /// Page size of `1` encoded.
    const VALUE_FOR_1: u32 = 65536;
}
impl Cuisiner for PageSize {
    type Raw<B: ByteOrder> = U16<B>;

    fn try_from_raw<B: ByteOrder>(raw: Self::Raw<B>) -> Result<Self, CuisinerError> {
        Ok(Self(match raw.get() as u32 {
            1 => Self::VALUE_FOR_1,
            n @ Self::MIN..=Self::MAX if n.is_power_of_two() => n,
            n => {
                return Err(CuisinerError::Validation(format!(
                    "page size must be a power of 2 between {min} and {max} (found {n})",
                    min = Self::MIN,
                    max = Self::MAX
                )));
            }
        }))
    }

    fn try_to_raw<B: ByteOrder>(self) -> Result<Self::Raw<B>, CuisinerError> {
        Ok(U16::new(match self.0 {
            Self::VALUE_FOR_1 => 1,
            n @ Self::MIN..=Self::MAX => n as u16,
            n => {
                return Err(CuisinerError::Validation(format!(
                    "page size must be a power of 2 between {min} and {max} (found {n})",
                    min = Self::MIN,
                    max = Self::MAX,
                )));
            }
        }))
    }
}

#[derive(Clone, Cuisiner, Debug)]
#[cuisiner(repr = u8)]
pub enum FileFormatVersion {
    Legacy = 1,
    Wal = 2,
}

#[derive(Clone, Cuisiner, Debug)]
#[cuisiner(repr = u32)]
pub enum SchemaFormat {
    V1 = 1,
    V2 = 2,
    V3 = 3,
    V4 = 4,
}

#[derive(Clone, Cuisiner, Debug)]
#[cuisiner(repr = u32)]
pub enum TextEncoding {
    Utf8 = 1,
    Utf16Le = 2,
    Utf16Be = 3,
}

#[derive(Clone, Debug)]
pub struct SqliteVersionNumber {
    major: u16,
    minor: u16,
    patch: u16,
}
impl From<u32> for SqliteVersionNumber {
    fn from(value: u32) -> Self {
        Self {
            major: (value / 1_000_000) as u16,
            minor: (value % 1_000_000 / 1_000) as u16,
            patch: (value % 1_000) as u16,
        }
    }
}
impl From<SqliteVersionNumber> for u32 {
    fn from(version: SqliteVersionNumber) -> Self {
        version.major as u32 * 1_000_000 + version.minor as u32 * 1_000 + version.patch as u32
    }
}
impl Cuisiner for SqliteVersionNumber {
    type Raw<B: ByteOrder> = U32<B>;

    fn try_from_raw<B: ByteOrder>(raw: Self::Raw<B>) -> Result<Self, CuisinerError> {
        Ok(Self::from(raw.get()))
    }

    fn try_to_raw<B: ByteOrder>(self) -> Result<Self::Raw<B>, CuisinerError> {
        Ok(U32::new(self.into()))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use rstest::*;

    #[rstest]
    #[case(0, 0, 0)]
    #[case(1, 2, 3)]
    #[case(3, 49, 1)]
    #[case(999, 999, 999)]
    fn sqlite_version_number(#[case] major: u32, #[case] minor: u32, #[case] patch: u32) {
        let version = major * 1_000_000 + minor * 1_000 + patch;
        let version = SqliteVersionNumber::from(version);

        assert_eq!(major as u16, version.major);
        assert_eq!(minor as u16, version.minor);
        assert_eq!(patch as u16, version.patch);
    }
}

mod file_format_version;
mod header_string;
mod page_size;
mod schema_format;
mod sqlite_version_number;
mod text_encoding;

use std::marker::PhantomData;

use assert_layout::assert_layout;
use header_string::HeaderStringError;
use page_size::PageSizeError;
use text_encoding::TextEncodingRaw;
use thiserror::Error;
use zerocopy::{Immutable, IntoBytes, KnownLayout, TryFromBytes, big_endian::*};

use self::{
    file_format_version::FileFormatVersionRaw, header_string::HeaderString, page_size::PageSize,
    schema_format::SchemaFormatRaw, sqlite_version_number::SqliteVersionNumber,
};
use super::{Invalid, Valid, Validate, ValidityMarker, util::*};

/// Expected size of the SQLite header in bytes.
pub const SQLITE_HEADER_SIZE: usize = 100;

/// Header of a SQLite file.
#[derive(Clone, Debug, TryFromBytes, IntoBytes, KnownLayout, Immutable)]
#[assert_layout(size = SQLITE_HEADER_SIZE, generics = "Valid")]
#[repr(C)]
pub struct SqliteHeader<V: ValidityMarker = Valid> {
    /// The header string.
    #[assert_layout(offset = 0, size = 16)]
    pub header_string: HeaderString<V>,
    /// Size of each page.
    #[assert_layout(offset = 16, size = 2)]
    pub page_size: PageSize<V>,
    /// File format write version.
    #[assert_layout(offset = 18, size = 1)]
    pub file_format_write_version: FileFormatVersionRaw<V>,
    /// File format read version.
    #[assert_layout(offset = 19, size = 1)]
    pub file_format_read_version: FileFormatVersionRaw<V>,
    /// Number of bytes for reserved space at the end of each page.
    #[assert_layout(offset = 20, size = 1)]
    pub page_end_padding: Optional<u8>,
    /// Maximum embedded payload fraction.
    #[assert_layout(offset = 21, size = 1)]
    pub max_payload_fraction: ConstU8<64>,
    /// Minimum embedded payload fraction.
    #[assert_layout(offset = 22, size = 1)]
    pub min_payload_fraction: ConstU8<32>,
    /// Leaf payload fraction.
    #[assert_layout(offset = 23, size = 1)]
    pub leaf_payload_fraction: ConstU8<32>,
    /// File change counter.
    #[assert_layout(offset = 24, size = 4)]
    pub file_change_counter: U32,
    /// Number of pages in the database.
    #[assert_layout(offset = 28, size = 4)]
    pub page_count: U32,
    /// Page number of the first freelist trunk page.
    #[assert_layout(offset = 32, size = 4)]
    pub freelist_trunk_page: U32,
    /// Total number of freelist pages.
    #[assert_layout(offset = 36, size = 4)]
    pub freelist_page_count: U32,
    /// Schema cookie.
    #[assert_layout(offset = 40, size = 4)]
    pub schema_cookie: U32,
    /// Schema format number.
    #[assert_layout(offset = 44, size = 4)]
    pub schema_format: SchemaFormatRaw<V>,
    /// Default page cache size.
    #[assert_layout(offset = 48, size = 4)]
    pub default_page_cache_size: U32,
    /// Page number of the largest root b-tree page. Will be [`None`] if not in auto-vacuum or
    /// incremental-vacuum modes.
    #[assert_layout(offset = 52, size = 4)]
    pub largest_root_btree_page: Optional<U32>,
    /// Text encoding.
    #[assert_layout(offset = 56, size = 4)]
    pub text_encoding: TextEncodingRaw<V>,
    /// User version as per `user_version_pragma`.
    #[assert_layout(offset = 60, size = 4)]
    pub user_version: U32,
    /// `true` for incremental-vacuum mode.
    #[assert_layout(offset = 64, size = 4)]
    pub incremental_vacuum_mode: ByteBoolean<4>,
    /// Application ID as per `PRAGMA application_id`.
    #[assert_layout(offset = 68, size = 4)]
    pub application_id: U32,
    /// Reserved for expansion.
    #[assert_layout(offset = 72, size = 20)]
    _reserved: Reserved<20>,
    /// `version-valid-for` number.
    #[assert_layout(offset = 92, size = 4)]
    pub version_valid_for: U32,
    /// SQLite version number.
    #[assert_layout(offset = 96, size = 4)]
    pub sqlite_version_number: SqliteVersionNumber,

    validity: PhantomData<fn() -> V>,
}

impl SqliteHeader<Invalid> {
    pub fn validate(&self) -> Result<&SqliteHeader<Valid>, SqliteHeaderError> {
        self.header_string.try_get()?;
        self.page_size.try_get()?;
        self.max_payload_fraction
            .try_get()
            .map_err(|source| SqliteHeaderError::ConstU8 {
                field: "max_payload_fraction",
                source,
            })?;
        self.min_payload_fraction
            .try_get()
            .map_err(|source| SqliteHeaderError::ConstU8 {
                field: "min_payload_fraction",
                source,
            })?;
        self.leaf_payload_fraction
            .try_get()
            .map_err(|source| SqliteHeaderError::ConstU8 {
                field: "leaf_payload_fraction",
                source,
            })?;
        self._reserved
            .try_get()
            .map_err(|source| SqliteHeaderError::Reserved {
                field: "_reserved",
                source,
            })?;

        Ok(
            // SAFETY: `Invalid`/`Valid` are zero-sized types (enums with no variants), contained
            // in `PhantomData`. Transmuting does not require any re-interpretation of bytes.
            unsafe { std::mem::transmute::<&SqliteHeader<Invalid>, &SqliteHeader<Valid>>(self) },
        )
    }
}

impl SqliteHeader {
    /// Try read the header from the provided buffer. The buffer must be exactly the correct size
    /// for the header.
    pub fn try_read(buf: &[u8]) -> Result<&Self, SqliteHeaderError> {
        let invalid_header =
            SqliteHeader::<Invalid>::try_ref_from_bytes(&buf).map_err(|e| match e {
                zerocopy::ConvertError::Size(_) => BinaryError::Size,
                zerocopy::ConvertError::Validity(_) => BinaryError::Validity,
                zerocopy::ConvertError::Alignment(_) => {
                    unreachable!("zerocopy `try_ref_from_bytes` should be infallibale")
                }
            })?;
        invalid_header.validate()
    }
}

#[derive(Clone, Debug, Error)]
pub enum SqliteHeaderError {
    #[error(transparent)]
    HeaderString(#[from] HeaderStringError),
    #[error(transparent)]
    PageSize(#[from] PageSizeError),
    #[error("invalid const value for {field}, expected {expected} (found {found})", expected = source.expected, found = source.found)]
    ConstU8 {
        field: &'static str,
        #[source]
        source: ConstU8Error,
    },
    #[error("expected {bytes} zero bytes for {field}", bytes = source.0)]
    Reserved {
        field: &'static str,
        #[source]
        source: ReservedError,
    },
    #[error(transparent)]
    Binary(#[from] BinaryError),
}

#[derive(Clone, Debug, Error)]
pub enum BinaryError {
    #[error("Invalid size for type")]
    Size,
    #[error("Invalid bytes for type")]
    Validity,
}

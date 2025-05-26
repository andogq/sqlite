use assert_layout::assert_layout;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use thiserror::Error;
use zerocopy::{Immutable, IntoBytes, KnownLayout, TryFromBytes, big_endian::*};

use super::util::{ConstU8, ConstU8Error};

/// Expected size of the SQLite header in bytes.
pub const SQLITE_HEADER_SIZE: usize = 100;

/// String expected to be present at the beginning of the header.
pub const HEADER_STRING: [u8; 16] = *b"SQLite format 3\0";

/// Header of a SQLite file.
#[derive(Clone, Debug, TryFromBytes, IntoBytes, KnownLayout, Immutable)]
#[assert_layout(size = SQLITE_HEADER_SIZE)]
#[repr(C)]
pub struct SqliteHeader {
    /// The header string.
    #[assert_layout(offset = 0, size = 16)]
    header_string: [u8; HEADER_STRING.len()],
    /// Size of each page.
    #[assert_layout(offset = 16, size = 2)]
    page_size: U16,
    /// File format write version.
    #[assert_layout(offset = 18, size = 1)]
    file_format_write_version: u8,
    /// File format read version.
    #[assert_layout(offset = 19, size = 1)]
    file_format_read_version: u8,
    /// Number of bytes for reserved space at the end of each page.
    #[assert_layout(offset = 20, size = 1)]
    page_end_padding: u8,
    /// Maximum embedded payload fraction.
    #[assert_layout(offset = 21, size = 1)]
    max_payload_fraction: ConstU8<64>,
    /// Minimum embedded payload fraction.
    #[assert_layout(offset = 22, size = 1)]
    min_payload_fraction: ConstU8<32>,
    /// Leaf payload fraction.
    #[assert_layout(offset = 23, size = 1)]
    leaf_payload_fraction: ConstU8<32>,
    /// File change counter.
    #[assert_layout(offset = 24, size = 4)]
    file_change_counter: U32,
    /// Number of pages in the database.
    #[assert_layout(offset = 28, size = 4)]
    page_count: U32,
    /// Page number of the first freelist trunk page.
    #[assert_layout(offset = 32, size = 4)]
    freelist_trunk_page: U32,
    /// Total number of freelist pages.
    #[assert_layout(offset = 36, size = 4)]
    freelist_page_count: U32,
    /// Schema cookie.
    #[assert_layout(offset = 40, size = 4)]
    schema_cookie: U32,
    /// Schema format number.
    #[assert_layout(offset = 44, size = 4)]
    schema_format: U32,
    /// Default page cache size.
    #[assert_layout(offset = 48, size = 4)]
    default_page_cache_size: U32,
    /// Page number of the largest root b-tree page. Will be [`None`] if not in auto-vacuum or
    /// incremental-vacuum modes.
    #[assert_layout(offset = 52, size = 4)]
    largest_root_btree_page: U32,
    /// Text encoding.
    #[assert_layout(offset = 56, size = 4)]
    text_encoding: U32,
    /// User version as per `user_version_pragma`.
    #[assert_layout(offset = 60, size = 4)]
    user_version: U32,
    /// `true` for incremental-vacuum mode.
    #[assert_layout(offset = 64, size = 4)]
    incremental_vacuum_mode: U32,
    /// Application ID as per `PRAGMA application_id`.
    #[assert_layout(offset = 68, size = 4)]
    application_id: U32,
    /// Reserved for expansion.
    #[assert_layout(offset = 72, size = 20)]
    reserved: [u8; 20],
    /// `version-valid-for` number.
    #[assert_layout(offset = 92, size = 4)]
    version_valid_for: U32,
    /// SQLite version number.
    #[assert_layout(offset = 96, size = 4)]
    sqlite_version_number: U32,
}

impl SqliteHeader {
    /// Try read the header from the provided buffer. The buffer must be exactly the correct size
    /// for the header.
    pub fn read_from_buffer(buf: &[u8]) -> Result<Self, SqliteHeaderError> {
        // Read the header.
        let header = SqliteHeader::try_read_from_bytes(buf).map_err(|e| match e {
            zerocopy::ConvertError::Size(_) => BinaryError::Size,
            zerocopy::ConvertError::Validity(_) => BinaryError::Validity,
            zerocopy::ConvertError::Alignment(_) => {
                unreachable!("zerocopy `try_ref_from_bytes` should be infallibale")
            }
        })?;

        // Validate the header.
        header.validate()?;

        Ok(header)
    }

    /// Validate the current instance of this header.
    fn validate(&self) -> Result<(), SqliteHeaderError> {
        if self.header_string != HEADER_STRING {
            return Err(SqliteHeaderError::HeaderString(self.header_string));
        }

        {
            let page_size = self.page_size.get();

            if page_size != 1 && !(512..=32768).contains(&page_size) {
                return Err(PageSizeError::Range(page_size).into());
            }

            if !page_size.is_power_of_two() {
                return Err(PageSizeError::PowerOfTwo(page_size).into());
            }
        }

        FileFormatVersion::try_from_primitive(self.file_format_read_version).map_err(|e| {
            EnumError {
                field: "file_format_read_version",
                value: e.number,
            }
        })?;

        FileFormatVersion::try_from_primitive(self.file_format_write_version).map_err(|e| {
            EnumError {
                field: "file_format_write_version",
                value: e.number,
            }
        })?;

        SchemaFormat::try_from_primitive(self.schema_format.get()).map_err(|e| EnumError {
            field: "schema_format",
            value: e.number,
        })?;

        self.max_payload_fraction
            .validate()
            .map_err(|source| SqliteHeaderError::ConstU8 {
                field: "max_payload_fraction",
                source,
            })?;
        self.min_payload_fraction
            .validate()
            .map_err(|source| SqliteHeaderError::ConstU8 {
                field: "min_payload_fraction",
                source,
            })?;
        self.leaf_payload_fraction
            .validate()
            .map_err(|source| SqliteHeaderError::ConstU8 {
                field: "leaf_payload_fraction",
                source,
            })?;

        TextEncoding::try_from_primitive(self.text_encoding.get()).map_err(|e| EnumError {
            field: "text_encoding",
            value: e.number,
        })?;

        if !self.reserved.iter().all(|&b| b == 0) {
            return Err(SqliteHeaderError::Reserved(self.reserved));
        }

        Ok(())
    }

    /// Get the page size of this database.
    pub fn page_size(&self) -> u32 {
        let n = self.page_size.get() as u32;

        if n == 1 {
            return 65536;
        }

        n
    }

    pub fn page_end_padding(&self) -> u8 {
        self.page_end_padding
    }

    pub fn page_count(&self) -> u32 {
        self.page_count.get()
    }

    /// Get the (major, minor, patch) version of this database.
    pub fn sqlite_version_number(&self) -> (u16, u16, u16) {
        let version = self.sqlite_version_number.get();

        (
            (version / 1_000_000) as u16,
            (version % 1_000_000 / 1_000) as u16,
            (version % 1_000) as u16,
        )
    }
}

#[derive(Clone, Copy, Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum FileFormatVersion {
    Legacy = 1,
    Wal = 2,
}

#[derive(Clone, Copy, Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u32)]
pub enum SchemaFormat {
    V1 = 1,
    V2 = 2,
    V3 = 3,
    V4 = 4,
}

#[derive(Clone, Copy, Debug, IntoPrimitive, TryFromPrimitive)]
#[repr(u32)]
pub enum TextEncoding {
    Utf8 = 1,
    Utf16Le = 2,
    Utf16Be = 3,
}

#[derive(Clone, Debug, Error)]
pub enum SqliteHeaderError {
    #[error("invalid header string (expected '{HEADER_STRING:#?}', found '{0:#?}')")]
    HeaderString([u8; 16]),
    #[error(transparent)]
    PageSize(#[from] PageSizeError),
    #[error("invalid const value for {field} (expected {expected}, found {found})", expected = source.expected, found = source.found)]
    ConstU8 {
        field: &'static str,
        #[source]
        source: ConstU8Error,
    },
    #[error("expected reserved 0x00 bytes (found {0:#?})")]
    Reserved([u8; 20]),
    #[error(transparent)]
    EnumU8(#[from] EnumError<u8>),
    #[error(transparent)]
    EnumU32(#[from] EnumError<u32>),
    #[error(transparent)]
    Binary(#[from] BinaryError),
}

#[derive(Clone, Debug, Error)]
#[error("invalid value for {field} (found {value})")]
pub struct EnumError<T> {
    field: &'static str,
    value: T,
}

#[derive(Clone, Debug, Error)]
pub enum PageSizeError {
    #[error(
        "page size must be between {min} and {max}, or {end} (found {0})",
        min = 512,
        max = 32768,
        end = 1
    )]
    Range(u16),
    #[error("page size must be a power of two (found {0})")]
    PowerOfTwo(u16),
}

#[derive(Clone, Debug, Error)]
pub enum BinaryError {
    #[error("Invalid size for type")]
    Size,
    #[error("Invalid bytes for type")]
    Validity,
}

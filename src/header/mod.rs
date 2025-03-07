mod page_size;
mod raw;

use anyhow::Error;
use derive_more::TryFrom;
use thiserror::Error;

pub use self::{page_size::PageSize, raw::RawDbHeader};

/// Header of a SQLite file.
///
/// This is the fully parsed and validated version of the header. See [`RawDbHeader`] for the disk
/// representation of the header.
#[allow(unused)]
pub struct DbHeader {
    /// Size of each page.
    pub page_size: PageSize,
    /// File format write version.
    pub file_format_write_version: FileFormatVersion,
    /// File format read version.
    pub file_format_read_version: FileFormatVersion,
    /// Number of bytes for reserved space at the end of each page.
    pub reserved_padding: u8,
    /// File change counter.
    pub file_change_counter: u32,
    /// The size of the database file in pages.
    pub database_page_count: u32,
    // Page number of the first freelist trunk page.
    pub first_freelist_trunk_page: u32,
    /// Total number of freelist pages.
    pub freelist_trunk_page_count: u32,
    /// The schema cookie.
    pub schema_cookie: u32,
    /// THe schema format number.
    pub schema_format_number: u32,
    /// Default page cache size.
    pub default_page_cache_size: u32,
    /// The page number of the largest b-tree page when in auto-vacuum or incremental-vacuum mode,
    /// or zero otherwise.
    pub largest_root_btree_page: u32,
    /// The database text encoding.
    pub database_text_encoding: u32,
    /// The user version as set by the `user_version` pragma.
    pub user_version: u32,
    /// True (non-zero) for incremental-vacuum mode. False (zero) otherwise.
    pub incremental_vacuum_mode: bool,
    /// The application ID set by `application_id` pragma.
    pub application_id: u32,
    /// The `version-valid-for` number.
    pub version_valid_for: u32,
    /// `SQLITE_VERSION_NUMBER`.
    pub sqlite_version: SqliteVersionNumber,
}

#[derive(Debug, Error)]
pub enum DbHeaderError {
    #[error("invalid header string (expected {HEADER_STRING}, found {0})")]
    InvalidHeaderString(String),

    #[error("header[72:91] must all be zero")]
    ReservedNotZero,

    #[error(transparent)]
    Other(#[from] Error),
}

/// Header string found in the documentation.
const HEADER_STRING: &str = "SQLite format 3\0";

impl TryFrom<RawDbHeader> for DbHeader {
    type Error = DbHeaderError;

    fn try_from(header: RawDbHeader) -> Result<Self, Self::Error> {
        // Validate the header string of the file.
        let header_string = String::from_utf8_lossy(&header.header_string);
        if header_string != HEADER_STRING {
            return Err(DbHeaderError::InvalidHeaderString(header_string.into()));
        }

        if !header.reserved.into_iter().all(|b| b == 0b00) {
            return Err(DbHeaderError::ReservedNotZero);
        }

        Ok(Self {
            page_size: header.page_size.get().try_into().map_err(Error::from)?,
            file_format_write_version: header
                .file_format_write_version
                .try_into()
                .map_err(Error::from)?,
            file_format_read_version: header
                .file_format_write_version
                .try_into()
                .map_err(Error::from)?,
            reserved_padding: header.reserved_padding,
            file_change_counter: header.file_change_counter.get(),
            database_page_count: header.database_page_count.get(),
            first_freelist_trunk_page: header.first_freelist_trunk_page.get(),
            freelist_trunk_page_count: header.freelist_trunk_page_count.get(),
            schema_cookie: header.schema_cookie.get(),
            schema_format_number: header.schema_format_number.get(),
            default_page_cache_size: header.default_page_cache_size.get(),
            largest_root_btree_page: header.largest_root_btree_page.get(),
            database_text_encoding: header.database_text_encoding.get(),
            user_version: header.user_version.get(),
            // True if the value isn't zero.
            incremental_vacuum_mode: header.incremental_vacuum_mode.get() != 0,
            application_id: header.application_id.get(),
            version_valid_for: header.version_valid_for.get(),
            sqlite_version: header.sqlite_version.get().into(),
        })
    }
}

/// Version of the file.
#[derive(Clone, Copy, Debug, TryFrom)]
#[try_from(repr)]
#[repr(u8)]
pub enum FileFormatVersion {
    Legacy = 1,
    Wal = 2,
}

#[derive(Clone, Copy, Debug, TryFrom)]
pub enum SchemaFormatNumber {
    V1 = 1,
    V2 = 2,
    V3 = 3,
    V4 = 4,
}

#[derive(Clone, Copy, Debug, TryFrom)]
pub enum TextEncoding {
    Utf8 = 1,
    Utf16Le = 2,
    Utf16Be = 3,
}

/// Version of SQLite in semver.
///
/// See [SQLite Documentation](https://www.sqlite.org/c3ref/c_source_id.html).
#[derive(Clone, Copy, Debug)]
pub struct SqliteVersionNumber {
    pub major: u16,
    pub minor: u16,
    pub patch: u16,
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

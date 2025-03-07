use static_assertions::const_assert_eq;
use zerocopy::{FromBytes, big_endian::*};

/// Raw database header which is read to and from the disk. This representation has no validation
/// of the fields, and is solely a representation of the memory.
///
/// All fields are encoded as big endian.
///
/// See [SQLite Documentation](https://www.sqlite.org/fileformat2.html#the_database_header).
#[derive(Clone, Debug, Default, FromBytes)]
#[repr(C)]
pub struct RawDbHeader {
    /// Should be `SQLite format 3\000`.
    pub header_string: [u8; 16],
    /// Size of each page.
    pub page_size: U16,
    /// File format write version.
    pub file_format_write_version: u8,
    /// File format read version.
    pub file_format_read_version: u8,
    /// Number of bytes for reserved space at the end of each page.
    pub reserved_padding: u8,
    /// Maximum embedded payload fraction. Must be 64.
    pub max_payload_fraction: u8,
    /// Minimum embedded payload fraction. Must be 32.
    pub min_payload_fraction: u8,
    /// Leaf payload fraction. Must be 32.
    pub leaf_payload_fraction: u8,
    /// File change counter.
    pub file_change_counter: U32,
    /// The size of the database file in pages.
    pub database_page_count: U32,
    // Page number of the first freelist trunk page.
    pub first_freelist_trunk_page: U32,
    /// Total number of freelist pages.
    pub freelist_trunk_page_count: U32,
    /// The schema cookie.
    pub schema_cookie: U32,
    /// THe schema format number.
    pub schema_format_number: U32,
    /// Default page cache size.
    pub default_page_cache_size: U32,
    /// The page number of the largest b-tree page when in auto-vacuum or incremental-vacuum mode,
    /// or zero otherwise.
    pub largest_root_btree_page: U32,
    /// The database text encoding.
    pub database_text_encoding: U32,
    /// The user version as set by the `user_version` pragma.
    pub user_version: U32,
    /// True (non-zero) for incremental-vacuum mode. False (zero) otherwise.
    pub incremental_vacuum_mode: U32,
    /// The application ID set by `application_id` pragma.
    pub application_id: U32,
    /// Reserved, should be zero.
    pub reserved: [u8; 20],
    /// The `version-valid-for` number.
    pub version_valid_for: U32,
    /// `SQLITE_VERSION_NUMBER`.
    pub sqlite_version: U32,
}

// SQLite header is 100 bytes.
const_assert_eq!(size_of::<RawDbHeader>(), 100);

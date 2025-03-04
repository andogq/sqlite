use std::{fs::File, io::Read, mem, slice};

use static_assertions::{assert_impl_all, const_assert_eq};
use zerocopy::{FromBytes, big_endian::*};

const DATABASE: &str = "test.db";

/// Version of the file.
#[derive(Clone, Copy, Debug)]
enum FileFormatVersion {
    Legacy = 1,
    Wal = 2,
}

/// Size of a database page.
#[derive(Clone, Copy, Debug)]
struct PageSize(u32);

/// Minimum size of the SQLite page.
const PAGE_SIZE_MIN: u16 = 512;
/// Maximum size of the SQLite page.
const PAGE_SIZE_MAX: u16 = 32768;
/// Size of the SQLite page if the reported size is `1`.
const PAGE_SIZE_1: u32 = 65536;

#[derive(Debug, thiserror::Error)]
enum PageSizeError {
    #[error("page size must be between 512 and 32768 inclusive: {0}")]
    NotInRange(u16),
    #[error("page size must be a power of 2: {0}")]
    NotPowerOfTwo(u16),
}

impl TryFrom<u16> for PageSize {
    type Error = PageSizeError;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        if !(PAGE_SIZE_MIN..=PAGE_SIZE_MAX).contains(&value) {
            return Err(PageSizeError::NotInRange(value));
        }

        if !value.is_power_of_two() {
            return Err(PageSizeError::NotPowerOfTwo(value));
        }

        let value = if value == 1 {
            PAGE_SIZE_1
        } else {
            value as u32
        };

        Ok(Self(value))
    }
}

impl From<PageSize> for u32 {
    fn from(size: PageSize) -> Self {
        size.0
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
enum SchemaFormatNumber {
    V1 = 1,
    V2 = 2,
    V3 = 3,
    V4 = 4,
    Other(u32),
}

#[derive(Clone, Copy, Debug)]
#[repr(u32)]
enum TextEncoding {
    Utf8 = 1,
    Utf16Le = 2,
    Utf16Be = 3,
    Other(u32),
}

#[derive(Clone, Debug, Default, FromBytes)]
#[repr(C)]
struct DbHeader {
    /// Should be `SQLite format 3\000`.
    header_string: [u8; 16],
    /// Size of each page.
    page_size: U16,
    /// File format write version.
    file_format_write_version: u8,
    /// File format read version.
    file_format_read_version: u8,
    /// Number of bytes for reserved space at the end of each page.
    reserved_padding: u8,
    /// Maximum embedded payload fraction. Must be 64.
    max_payload_fraction: u8,
    /// Minimum embedded payload fraction. Must be 32.
    min_payload_fraction: u8,
    /// Leaf payload fraction. Must be 32.
    leaf_payload_fraction: u8,
    /// File change counter.
    file_change_counter: U32,
    /// The size of the database file in pages.
    database_page_count: U32,
    // Page number of the first freelist trunk page.
    first_freelist_trunk_page: U32,
    /// Total number of freelist pages.
    freelist_trunk_page_count: U32,
    /// The schema cookie.
    schema_cookie: U32,
    /// THe schema format number.
    schema_format_number: U32,
    /// Default page cache size.
    default_page_cache_size: U32,
    /// The page number of the largest b-tree page when in auto-vacuum or incremental-vacuum mode,
    /// or zero otherwise.
    largest_root_btree_page: U32,
    /// The database text encoding.
    database_text_encoding: U32,
    /// The user version as set by the `user_version` pragma.
    user_version: U32,
    /// True (non-zero) for incremental-vacuum mode. False (zero) otherwise.
    incremental_vacuum_mode: U32,
    /// The application ID set by `application_id` pragma.
    application_id: U32,
    /// Reserved, should be zero.
    _reserved: [u8; 20],
    /// The `version-valid-for` number.
    version_valid_for: U32,
    /// `SQLITE_VERSION_NUMBER`.
    sqlite_version: U32,
}

// SQLite header is 100 bytes.
const_assert_eq!(size_of::<DbHeader>(), 100);
assert_impl_all!(DbHeader: Sized);

trait PageStorage {
    fn read_page(&self, page_id: usize);
}

struct Pager {
    storage: Box<dyn PageStorage>,

    header: DbHeader,
}

impl Pager {
    pub fn new() -> Self {
        todo!()
    }
}

fn read_header(reader: impl Read) -> DbHeader {
    DbHeader::read_from_io(reader).unwrap()
}

fn main() {
    println!("Hello, world!");

    let mut file = File::open(DATABASE).unwrap();
    let header = read_header(&mut file);

    dbg!(header);
}

//! Page structures as they exist on disk.

use assert_layout::assert_layout;
use derive_more::Deref;
use zerocopy::{Immutable, IntoBytes, KnownLayout, TryFromBytes, big_endian::*};

/// Page header for leaf pages.
#[derive(Clone, Debug, TryFromBytes, IntoBytes, KnownLayout, Immutable)]
#[assert_layout(size = 8)]
#[repr(C)]
pub struct DiskLeafPageHeader {
    #[assert_layout(offset = 0, size = 1)]
    pub flag: u8,
    #[assert_layout(offset = 1, size = 2)]
    pub first_freeblock: U16,
    #[assert_layout(offset = 3, size = 2)]
    pub cell_count: U16,
    #[assert_layout(offset = 5, size = 2)]
    pub cell_content_area_offset: U16,
    #[assert_layout(offset = 7, size = 1)]
    pub fragmented_free_bytes_count: u8,
}

/// Page header for interior pages.
#[derive(Clone, Debug, TryFromBytes, IntoBytes, KnownLayout, Immutable, Deref)]
#[assert_layout(size = 12)]
#[repr(C)]
pub struct DiskInteriorPageHeader {
    // NOTE: Interior pages are a super set of leaf pages.
    #[deref]
    #[assert_layout(offset = 0, size = 8)]
    leaf_header: DiskLeafPageHeader,

    #[assert_layout(offset = 8, size = 4)]
    pub right_page_pointer: U32,
}

use std::ops::Deref;

use assert_layout::assert_layout;
use derive_more::Deref;
use zerocopy::{Immutable, IntoBytes, KnownLayout, TryFromBytes, Unaligned, big_endian::*};

use super::util::ConstU8;

pub enum Table {}
pub enum Index {}

pub trait PageType {
    const INTERIOR_FLAG: u8;
    const LEAF_FLAG: u8;

    type InteriorPageHeader;
    type LeafPageHeader;
    type PageHeader;
}

impl PageType for Table {
    const INTERIOR_FLAG: u8 = 0x05;
    const LEAF_FLAG: u8 = 0x0d;

    type InteriorPageHeader = InteriorPageHeader<{ Self::INTERIOR_FLAG }>;
    type LeafPageHeader = LeafPageHeader<{ Self::LEAF_FLAG }>;
    type PageHeader = PageHeader<{ Self::LEAF_FLAG }, { Self::INTERIOR_FLAG }>;
}

impl PageType for Index {
    const INTERIOR_FLAG: u8 = 0x02;
    const LEAF_FLAG: u8 = 0x0a;

    type InteriorPageHeader = InteriorPageHeader<{ Self::INTERIOR_FLAG }>;
    type LeafPageHeader = LeafPageHeader<{ Self::LEAF_FLAG }>;
    type PageHeader = PageHeader<{ Self::LEAF_FLAG }, { Self::INTERIOR_FLAG }>;
}

#[derive(Clone, Debug, TryFromBytes, IntoBytes, KnownLayout, Immutable, Unaligned)]
#[assert_layout(size = 7)]
#[repr(C)]
pub struct RawPageHeader {
    #[assert_layout(offset = 0, size = 2)]
    pub first_freeblock: U16,
    #[assert_layout(offset = 2, size = 2)]
    pub cell_count: U16,
    #[assert_layout(offset = 4, size = 2)]
    pub cell_content_area_offset: U16,
    #[assert_layout(offset = 6, size = 1)]
    pub fragmented_free_bytes_count: u8,
}

#[derive(Clone, Debug, TryFromBytes, IntoBytes, KnownLayout, Immutable, Deref)]
#[assert_layout(size = 8)]
#[repr(C)]
pub struct LeafPageHeader<const FLAG: u8> {
    #[assert_layout(offset = 0, size = 1)]
    flag: ConstU8<FLAG>,

    #[deref]
    #[assert_layout(offset = 1, size = 7)]
    header: RawPageHeader,
}

#[derive(Clone, Debug, TryFromBytes, IntoBytes, KnownLayout, Immutable, Deref)]
#[assert_layout(size = 12)]
#[repr(C)]
pub struct InteriorPageHeader<const FLAG: u8> {
    #[assert_layout(offset = 0, size = 1)]
    flag: ConstU8<FLAG>,

    #[deref]
    #[assert_layout(offset = 1, size = 7)]
    header: RawPageHeader,

    #[assert_layout(offset = 8, size = 4)]
    pub right_page_pointer: U32,
}

pub enum PageHeader<const LEAF_FLAG: u8, const INTERIOR_FLAG: u8> {
    Leaf(LeafPageHeader<LEAF_FLAG>),
    Interior(InteriorPageHeader<INTERIOR_FLAG>),
}

impl<const LEAF_FLAG: u8, const INTERIOR_FLAG: u8> PageHeader<LEAF_FLAG, INTERIOR_FLAG> {
    pub fn try_read_from_prefix(source: &[u8]) -> Result<(Self, &[u8]), ()> {
        match source[0] {
            n if n == LEAF_FLAG => {
                let (header, suffix) =
                    LeafPageHeader::try_read_from_prefix(source).map_err(|_| ())?;

                Ok((Self::Leaf(header), suffix))
            }
            n if n == INTERIOR_FLAG => {
                let (header, suffix) =
                    InteriorPageHeader::try_read_from_prefix(source).map_err(|_| ())?;
                Ok((Self::Interior(header), suffix))
            }
            n => panic!("unknown flag {n}"),
        }
    }
}

impl<const LEAF_FLAG: u8, const INTERIOR_FLAG: u8> Deref for PageHeader<LEAF_FLAG, INTERIOR_FLAG> {
    type Target = RawPageHeader;

    fn deref(&self) -> &Self::Target {
        match self {
            PageHeader::Leaf(leaf) => &leaf.header,
            PageHeader::Interior(interior) => &interior.header,
        }
    }
}

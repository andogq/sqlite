mod disk;

use std::{marker::PhantomData, num::NonZero, ops::Deref};

use derive_more::Deref;
use zerocopy::{FromBytes, TryFromBytes, big_endian::*};

use super::pager::PageBuffer;

pub enum Table {}
pub enum Index {}

pub trait PageType {
    const FLAG: u8;
    const INTERIOR_FLAG: u8;
    const LEAF_FLAG: u8;

    fn is_table() -> bool {
        false
    }

    fn is_index() -> bool {
        false
    }
}

impl PageType for Table {
    const FLAG: u8 = 0x05;
    const INTERIOR_FLAG: u8 = 0x05;
    const LEAF_FLAG: u8 = 0x0d;

    fn is_table() -> bool {
        true
    }
}
impl PageType for Index {
    const FLAG: u8 = 0x02;
    const INTERIOR_FLAG: u8 = 0x02;
    const LEAF_FLAG: u8 = 0x0a;

    fn is_index() -> bool {
        true
    }
}

pub trait PageExt {
    /// Create a new page from the provided buffer.
    fn from_buffer(buffer: PageBuffer) -> Self;

    /// Consume the page, and produce the underlying buffer.
    fn into_buffer(self) -> PageBuffer;
}

#[derive(Clone, Debug)]
pub enum Page<T: PageType> {
    Leaf(LeafPage<T>),
    Interior(InteriorPage<T>),
}

impl<T: PageType> PageExt for Page<T> {
    fn from_buffer(buffer: PageBuffer) -> Self {
        let flag = PageFlag::new(buffer[0]).expect("valid page flag");

        // NOTE: Inner `from_buffer` implementation will ensure that the flag conforms to `T`.
        match flag.kind_flag {
            PageKindFlag::Leaf => Self::Leaf(LeafPage::from_buffer(buffer)),
            PageKindFlag::Interior => Self::Interior(InteriorPage::from_buffer(buffer)),
        }
    }

    fn into_buffer(self) -> PageBuffer {
        match self {
            Page::Leaf(leaf_page) => leaf_page.into_buffer(),
            Page::Interior(interior_page) => interior_page.into_buffer(),
        }
    }
}

impl<T: PageType> Deref for Page<T> {
    type Target = PageCommon<T>;

    fn deref(&self) -> &Self::Target {
        match self {
            Page::Leaf(leaf_page) => leaf_page,
            Page::Interior(interior_page) => interior_page,
        }
    }
}

#[derive(Clone, Debug)]
enum PageKindFlag {
    Leaf,
    Interior,
}

impl PageKindFlag {
    const MASK: u8 = 0b1000;

    pub const fn new(flag: u8) -> Option<Self> {
        match flag & Self::MASK {
            0b1000 => Some(Self::Leaf),
            0b0000 => Some(Self::Interior),
            _ => None,
        }
    }
}

#[derive(Clone, Debug)]
enum PageTypeFlag {
    Table,
    Index,
}

impl PageTypeFlag {
    const MASK: u8 = 0b0111;

    pub const fn new(flag: u8) -> Option<Self> {
        match flag & Self::MASK {
            0b101 => Some(Self::Table),
            0b010 => Some(Self::Index),
            _ => None,
        }
    }

    pub fn is<T: PageType>(&self) -> bool {
        match self {
            PageTypeFlag::Table if T::is_table() => true,
            PageTypeFlag::Index if T::is_index() => true,
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct PageFlag {
    flag: u8,

    pub kind_flag: PageKindFlag,
    pub type_flag: PageTypeFlag,
}
impl PageFlag {
    pub fn new(flag: u8) -> Option<Self> {
        Some(Self {
            flag,
            kind_flag: PageKindFlag::new(flag)?,
            type_flag: PageTypeFlag::new(flag)?,
        })
    }
}

#[derive(Clone, Debug)]
pub struct PageCommon<T: PageType> {
    pub flag: PageFlag,

    /// Start of the first freeblock within this page.
    pub first_freeblock: Option<NonZero<u16>>,

    /// Number of cells in this page.
    pub cell_count: u16,

    /// Offset to the cell content area within this page.
    cell_content_area_offset: NonZero<u32>,

    /// Number of free bytes in the cell content area.
    pub free_bytes: u8,

    /// Buffer that contains the data of this page.
    buffer: PageBuffer,

    /// Marker for the type of this page ([`Table`] or [`Index`]).
    page_type: PhantomData<T>,
}

impl<T: PageType> PageCommon<T> {
    /// Parse the `first_freeblock` value.
    fn first_freeblock(first_freeblock: U16) -> Option<NonZero<u16>> {
        NonZero::new(first_freeblock.get())
    }

    /// Parse the `cell_count` value.
    fn cell_count(cell_count: U16) -> u16 {
        cell_count.get()
    }

    /// Parse the `cell_content_area_offset` value.
    fn cell_content_area_offset(cell_content_area_offset: U16) -> NonZero<u32> {
        NonZero::new(cell_content_area_offset.get() as u32)
            .unwrap_or(NonZero::new(2u32.pow(16)).unwrap())
    }

    /// Calculate the length of the header.
    const fn header_length(&self) -> usize {
        match self.flag.kind_flag {
            PageKindFlag::Leaf => size_of::<disk::DiskLeafPageHeader>(),
            PageKindFlag::Interior => size_of::<disk::DiskInteriorPageHeader>(),
        }
    }

    /// Produce a slice that begins after the page header.
    pub fn after_header(&self) -> &[u8] {
        let header_length = self.header_length();

        &self.buffer[header_length..]
    }

    /// Produce an iterator of pointers into the cell content area. The pointers will be relative
    /// to the cell content area (that is, the buffer returned by [`Self::cell_content_area`]).
    pub fn cell_content_pointers(&self) -> impl Iterator<Item = usize> {
        // Determine the length of the cell content pointer array.
        let length = self.cell_count as usize * size_of::<U16>();

        // Slice the buffer to include the cell content pointer array, which begins immediately
        // after the header.
        let buf = &self.after_header()[..length];

        // Cast the slice into an array of big-endian u16s
        <[U16]>::ref_from_bytes_with_elems(buf, self.cell_count as usize)
            .unwrap()
            .iter()
            // Fetch the value
            .map(|pointer| pointer.get() as usize)
            // Adjust pointer to be relative to the cell content area
            .map(|pointer| pointer - self.cell_content_area_offset.get() as usize)
    }

    /// Return a slice to the cell content area.
    pub fn cell_content_area(&self) -> &[u8] {
        let offset = self.cell_content_area_offset.get() as usize;

        // Slice into the raw buffer, as `cell_content_area_offset` includes additional offset for
        // header on first page.
        &self.buffer.raw()[offset..]
    }
}

#[derive(Clone, Debug, Deref)]
pub struct LeafPage<T: PageType> {
    #[deref]
    common: PageCommon<T>,
}

impl<T: PageType> PageExt for LeafPage<T> {
    fn from_buffer(buffer: PageBuffer) -> Self {
        let (header, _) = disk::DiskLeafPageHeader::try_ref_from_prefix(&buffer[..]).unwrap();

        let Some(flag) = PageFlag::new(header.flag).filter(|flag| {
            matches!(flag.kind_flag, PageKindFlag::Leaf) && flag.type_flag.is::<T>()
        }) else {
            panic!("invalid page flag in header: {}", header.flag);
        };

        Self {
            common: PageCommon {
                flag,
                first_freeblock: PageCommon::<T>::first_freeblock(header.first_freeblock),
                cell_count: PageCommon::<T>::cell_count(header.cell_count),
                cell_content_area_offset: PageCommon::<T>::cell_content_area_offset(
                    header.cell_content_area_offset,
                ),
                free_bytes: header.fragmented_free_bytes_count,
                buffer,
                page_type: PhantomData,
            },
        }
    }

    fn into_buffer(self) -> PageBuffer {
        self.common.buffer
    }
}

#[derive(Clone, Debug, Deref)]
pub struct InteriorPage<T: PageType> {
    #[deref]
    common: PageCommon<T>,

    /// Pointer to the right most page.
    pub right_pointer: u32,
}

impl<T: PageType> PageExt for InteriorPage<T> {
    fn from_buffer(buffer: PageBuffer) -> Self {
        let header = disk::DiskInteriorPageHeader::try_ref_from_bytes(&buffer).unwrap();

        let Some(flag) = PageFlag::new(header.flag).filter(|flag| {
            matches!(flag.kind_flag, PageKindFlag::Interior) && flag.type_flag.is::<T>()
        }) else {
            panic!("invalid page flag in header: {}", header.flag);
        };

        Self {
            right_pointer: header.right_page_pointer.get(),
            common: PageCommon {
                flag,
                first_freeblock: PageCommon::<T>::first_freeblock(header.first_freeblock),
                cell_count: PageCommon::<T>::cell_count(header.cell_count),
                cell_content_area_offset: PageCommon::<T>::cell_content_area_offset(
                    header.cell_content_area_offset,
                ),
                free_bytes: header.fragmented_free_bytes_count,
                buffer,
                page_type: PhantomData,
            },
        }
    }

    fn into_buffer(self) -> PageBuffer {
        self.common.buffer
    }
}

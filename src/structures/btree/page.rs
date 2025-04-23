use std::marker::PhantomData;

use assert_layout::assert_layout;
use thiserror::Error;
use zerocopy::{
    FromBytes, Immutable, KnownLayout,
    big_endian::{U16, U32},
};

use crate::{PageId, memory::*, structures::header::SQLITE_HEADER_SIZE};

use super::{
    PageType, TreeKind,
    cell::{Index, Table},
};

#[derive(Clone)]
pub struct Page<K: TreeKind> {
    pub page_id: PageId,
    pub disk_page: MemoryPage,
    pub kind: PhantomData<K>,
}

impl<K: TreeKind> Page<K> {
    pub fn new(page_id: PageId, disk_page: MemoryPage) -> Self {
        Self {
            page_id,
            disk_page,
            kind: PhantomData,
        }
    }

    pub fn operate<'p, 'b>(&'p self) -> PageOperation<'p, 'b, K>
    where
        'p: 'b,
    {
        PageOperation::new(self)
    }
}

pub struct PageOperation<'p, 'b, K: TreeKind> {
    page: &'p Page<K>,
    buf: MemoryPageRef<'b>,
}

impl<'p, 'b, K: TreeKind> PageOperation<'p, 'b, K>
where
    'p: 'b,
{
    fn new(page: &'p Page<K>) -> Self {
        Self {
            page,
            buf: page.disk_page.buffer(),
        }
    }

    fn header_offset(&self) -> usize {
        if self.page.page_id.is_header_page() {
            SQLITE_HEADER_SIZE
        } else {
            0
        }
    }

    fn after_header_offset(&self) -> usize {
        self.header_offset() + size_of::<PageHeader<K>>()
    }

    /// Parse the page header from the buffer.
    pub fn header(&self) -> &PageHeader<K> {
        let buf = &self.buf[self.header_offset()..];

        // Read out the base of the header.
        let (header, _) = PageHeader::<K>::read_from_prefix(buf).unwrap();

        header
    }

    /// Fetch the right pointer of the page, if it's present.
    pub fn get_right_pointer(&self) -> Option<u32> {
        match self.header().get_page_type() {
            PageType::Interior => {
                // Advance buffer beyond page header.
                let buf = &self.buf[self.after_header_offset()..];

                // Fetch the right pointer.
                let (right_pointer, _) = U32::ref_from_prefix(buf).unwrap();
                Some(right_pointer.get())
            }
            PageType::Leaf => None,
        }
    }

    /// Calculate the slice of the cell pointer array.
    fn get_cell_pointer_array(&self) -> &[U16] {
        let header = self.header();

        // Calculate the offset past the page header, and the optional right pointer.
        let offset = self.after_header_offset()
            + match header.get_page_type() {
                PageType::Leaf => 0,
                PageType::Interior => size_of::<U32>(),
            };
        let buf = &self.buf[offset..];

        // Parse out the pointer array.
        let (cell_pointer_array, _) =
            <[U16]>::ref_from_prefix_with_elems(buf, header.cell_count.get().into()).unwrap();
        cell_pointer_array
    }

    pub fn get_cell_buffer(&self, cell_number: usize) -> MemoryPage {
        let header = self.header();

        if cell_number >= header.cell_count.get() as usize {
            panic!("offset larger than available cells");
        }

        let cell_pointer_array = self.get_cell_pointer_array();

        // Select the relevant area of the cell content area.
        let offset = cell_pointer_array[cell_number].get() as usize;

        let buf = self.page.disk_page.slice(offset..);

        buf
    }
}

#[derive(Clone, Debug, PartialEq, Eq, FromBytes, KnownLayout, Immutable)]
#[assert_layout(size = 8, generics = "Table", generics = "Index")]
#[repr(C)]
pub struct PageHeader<K: TreeKind> {
    /// Flag indicating the kind of the page.
    #[assert_layout(offset = 0, size = 1)]
    page_flag: u8,

    // Common fields for all pages.
    #[assert_layout(offset = 1, size = 2)]
    first_freeblock: U16,
    #[assert_layout(offset = 3, size = 2)]
    cell_count: U16,
    #[assert_layout(offset = 5, size = 2)]
    cell_content_offset: U16,
    #[assert_layout(offset = 7, size = 1)]
    fragmented_bytes: u8,

    kind: PhantomData<K>,
}

impl<K: TreeKind> PageHeader<K> {
    pub(super) fn read_from_prefix(buf: &[u8]) -> Result<(&Self, &[u8]), PageHeaderError> {
        let (header, data) = Self::ref_from_prefix(buf).unwrap();
        header.validate()?;
        Ok((header, data))
    }

    fn validate(&self) -> Result<(), PageHeaderError> {
        if (self.page_flag & K::MASK) != K::MASK {
            return Err(PageHeaderError::InvalidFlag(self.page_flag));
        }

        Ok(())
    }

    pub(super) fn get_page_type(&self) -> PageType {
        PageType::from_page_flag(self.page_flag)
    }
}

#[derive(Clone, Debug, Error)]
pub enum PageHeaderError {
    #[error("invalid page flag: {0}")]
    InvalidFlag(u8),
}

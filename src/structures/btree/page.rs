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
    cell::{Index, PageCell, PageCtx, Table},
};

#[derive(Clone)]
pub struct Page<K: TreeKind> {
    pub page_id: PageId,
    pub page_ctx: PageCtx,
    pub disk_page: MemoryPage,
    pub kind: PhantomData<fn() -> K>,
}

impl<K: TreeKind> Page<K> {
    pub fn new(page_id: PageId, page_ctx: PageCtx, disk_page: MemoryPage) -> Self {
        Self {
            page_id,
            page_ctx,
            disk_page,
            kind: PhantomData,
        }
    }
}

impl<K: TreeKind> Process for Page<K> {
    type Data<'a> = PageContent<'a, K>;

    fn get_page_ref(&self) -> MemoryPageRef<'_> {
        self.disk_page.buffer()
    }
}

/// Content of a page, containing references to the underlying buffers. This struct is intended to
/// be constructed using [`FromMemoryPageRef`].
///
/// `'r` is the lifetime of the underlying memory page reference that the instance was constructed
/// from.
#[derive(Clone)]
pub struct PageContent<'r, K: TreeKind> {
    page_ctx: &'r PageCtx,
    /// Page header.
    pub header: &'r PageHeader<K>,
    /// Right pointer, only present on interior pages.
    pub right_pointer: Option<u32>,
    /// Array of offsets pointing to each page.
    pub pointer_array: &'r [U16],
    /// Buffer containing cell content.
    pub content_buffer: &'r [u8],
    /// Reference to the underlying [`MemoryPageRef`].
    ///
    /// This is a bit hacky, and should be removed. Alternatively, the lifetime of the reference
    /// should be `'c`, similar to [`FromMemoryPageRef::from_ref`].
    pub page_ref: &'r MemoryPageRef<'r>,
}

impl<'r, K: TreeKind> FromMemoryPageRef<'r, &'r Page<K>> for PageContent<'r, K> {
    fn from_ref<'c: 'r>(page: &'r Page<K>, page_ref: &'c MemoryPageRef<'r>) -> Self {
        let header_offset = if page.page_id.is_header_page() {
            SQLITE_HEADER_SIZE
        } else {
            0
        };

        let (header, buf) = PageHeader::<K>::read_from_prefix(&page_ref[header_offset..]).unwrap();

        let (right_pointer, buf) = match header.page_type() {
            PageType::Interior => {
                // Fetch the right pointer.
                let (right_pointer, buf) = U32::ref_from_prefix(buf).unwrap();
                (Some(right_pointer.get()), buf)
            }
            PageType::Leaf => (None, buf),
        };

        // Parse out the pointer array.
        let (pointer_array, _) =
            <[U16]>::ref_from_prefix_with_elems(buf, header.cell_count.get().into()).unwrap();

        Self {
            page_ctx: &page.page_ctx,
            header,
            right_pointer,
            pointer_array,
            content_buffer: &page_ref[header.cell_content_offset() as usize..],
            page_ref,
        }
    }
}

impl<'r, K: TreeKind> PageContent<'r, K> {
    pub fn get_cell(&self, i: usize) -> K::Cell<'r> {
        let offset =
            self.pointer_array[i].get() as usize - self.header.cell_content_offset() as usize;

        K::Cell::from_buffer(
            self.page_ctx,
            &self.content_buffer[offset..],
            self.header.page_type(),
        )
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

    pub fn page_type(&self) -> PageType {
        PageType::from_page_flag(self.page_flag)
    }

    pub fn cell_content_offset(&self) -> u16 {
        self.cell_content_offset.get()
    }
}

#[derive(Clone, Debug, Error)]
pub enum PageHeaderError {
    #[error("invalid page flag: {0}")]
    InvalidFlag(u8),
}

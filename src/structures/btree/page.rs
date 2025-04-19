use std::marker::PhantomData;

use assert_layout::assert_layout;
use thiserror::Error;
use zerocopy::{FromBytes, Immutable, KnownLayout, big_endian::U16};

use super::{Index, PageType, Table, TreeKind};

#[derive(Clone, Debug)]
pub struct Page<'p, K: TreeKind> {
    pub(super) header: &'p PageHeader<K>,
    pub(super) right_pointer: Option<u32>,
    pub(super) data: &'p [u8],
    pub(super) kind: PhantomData<K>,
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
    pub fn read_from_prefix(buf: &[u8]) -> Result<(&Self, &[u8]), PageHeaderError> {
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

    pub fn get_page_type(&self) -> PageType {
        PageType::from_page_flag(self.page_flag)
    }
}

#[derive(Clone, Debug, Error)]
pub enum PageHeaderError {
    #[error("invalid page flag: {0}")]
    InvalidFlag(u8),
}

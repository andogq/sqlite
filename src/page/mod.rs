pub mod storage;

use std::num::NonZero;

use anyhow::Error;
use derive_more::{Deref, TryFrom};
use static_assertions::const_assert_eq;
use thiserror::Error;
use zerocopy::{FromBytes, big_endian::*};

use self::storage::{PageStorage, StorageError};
use crate::{
    RawDbHeader,
    header::{DbHeader, DbHeaderError, RAW_HEADER_SIZE},
};

#[derive(Debug, Error)]
pub enum PagerError {
    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error(transparent)]
    DbHeader(#[from] DbHeaderError),

    #[error(transparent)]
    PageHeader(#[from] PageHeaderError),
}

pub struct Pager {
    storage: Box<dyn PageStorage>,

    header: DbHeader,
}

impl Pager {
    pub fn new(mut storage: impl 'static + PageStorage) -> Result<Self, PagerError> {
        let header_bytes = storage.read_start(RAW_HEADER_SIZE)?;
        let header: DbHeader = RawDbHeader::read_from_prefix(&header_bytes)
            .map(|(header, _)| header)
            .expect("header_bytes correct size for RawDbHeader")
            .try_into()?;

        storage.set_page_size(*header.page_size as u32);

        Ok(Self {
            storage: Box::new(storage),
            header,
        })
    }

    pub fn get_page_header(&mut self, page_id: u32) -> Result<PageHeader, PagerError> {
        let mut page = self.storage.read_page(page_id)?;

        if page_id == 0 {
            // Since the database header resides in the first page, offset if loading the first
            // page.
            page = &page[RAW_HEADER_SIZE..];
        }

        Ok(PageHeader::try_read(page)?)
    }
}

#[derive(Debug, Clone)]
pub enum PageHeader {
    Leaf(LeafPageHeader),
    Interior(InteriorPageHeader),
}

impl PageHeader {
    /// Try to read a page header from the provided bytes.
    pub fn try_read(bytes: &[u8]) -> Result<Self, PageHeaderError> {
        // Peek ahead at the header kind
        let page_type = PageType::try_from(*bytes.first().ok_or(PageHeaderError::NotEnoughBytes)?)
            .map_err(|e| PageHeaderError::InvalidPageType(e.input))?;

        Ok(if page_type.is_leaf() {
            Self::Leaf(
                RawLeafPageHeader::read_from_prefix(bytes)
                    .map(|(header, _)| header)
                    .map_err(|_| PageHeaderError::NotEnoughBytes)?
                    .try_into()?,
            )
        } else {
            Self::Interior(
                RawInteriorPageHeader::read_from_prefix(bytes)
                    .map(|(header, _)| header)
                    .map_err(|_| PageHeaderError::NotEnoughBytes)?
                    .try_into()?,
            )
        })
    }
}

#[derive(Debug, Error)]
pub enum PageHeaderError {
    #[error("invalid page type: {0}")]
    InvalidPageType(u8),

    #[error("not enough bytes to read header")]
    NotEnoughBytes,

    #[error(transparent)]
    Other(#[from] Error),
}

/// Page header which is present at the start of every page. This structure represents the raw
/// binary stored on disk, and has not had any validation on any of its fields.
#[derive(Clone, Debug, FromBytes)]
#[repr(C)]
struct RawLeafPageHeader {
    /// Flag indicating the type of the page.
    pub page_type: u8,
    /// Start of the first freeblock on the page.
    pub first_freeblock: U16,
    /// Number of cells in the page.
    pub cell_count: U16,
    /// Start of the cell content area.
    pub cell_content_offset: U16,
    /// Number of fragmented free bytes in the cell content area.
    pub cell_content_free: u8,
}
const_assert_eq!(size_of::<RawLeafPageHeader>(), 8);

#[derive(Clone, Debug)]
pub struct LeafPageHeader {
    pub page_type: PageType,
    pub first_freeblock: Option<NonZero<u16>>,
    pub cell_count: u16,
    pub cell_content_offset: NonZero<u32>,
    pub cell_content_free: u8,
}

impl TryFrom<RawLeafPageHeader> for LeafPageHeader {
    type Error = PageHeaderError;

    fn try_from(header: RawLeafPageHeader) -> Result<Self, Self::Error> {
        Ok(Self {
            page_type: PageType::try_from(header.page_type)
                .map_err(|e| PageHeaderError::InvalidPageType(e.input))?,
            // If `first_freeblock` is 0, then there are no free blocks.
            first_freeblock: NonZero::try_from(header.first_freeblock.get()).ok(),
            cell_count: header.cell_count.get(),
            // If `cell_content_offset` is 0, then it is interpreted as 65536.
            cell_content_offset: NonZero::try_from(header.cell_content_offset.get() as u32)
                .unwrap_or(NonZero::new(65536).unwrap()),
            cell_content_free: header.cell_content_free,
        })
    }
}

/// Page header for interior pages, which is a superset of [`RawPageHeader`].
#[derive(Clone, Debug, Deref, FromBytes)]
#[repr(C)]
struct RawInteriorPageHeader {
    /// Common fields from base page header.
    #[deref]
    header: RawLeafPageHeader,
    /// Right most pointer, only used for interior b-tree pages.
    pub right_page: U32,
}
const_assert_eq!(size_of::<RawInteriorPageHeader>(), 12);

#[derive(Clone, Debug, Deref)]
pub struct InteriorPageHeader {
    #[deref]
    header: LeafPageHeader,
    pub right_page: u32,
}

impl TryFrom<RawInteriorPageHeader> for InteriorPageHeader {
    type Error = PageHeaderError;

    fn try_from(header: RawInteriorPageHeader) -> Result<Self, Self::Error> {
        Ok(Self {
            header: header.header.try_into()?,
            right_page: header.right_page.get(),
        })
    }
}

#[derive(Clone, Copy, Debug, TryFrom)]
#[try_from(repr)]
#[repr(u8)]
pub enum PageType {
    InteriorIndex = 0x02,
    InteriorTable = 0x05,
    LeafIndex = 0x0a,
    LeafTable = 0x0d,
}

impl PageType {
    pub fn is_interior(&self) -> bool {
        matches!(self, Self::InteriorIndex | Self::InteriorTable)
    }

    pub fn is_leaf(&self) -> bool {
        matches!(self, Self::LeafIndex | Self::LeafTable)
    }
}

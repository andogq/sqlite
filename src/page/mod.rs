//! Within SQLite, the b-tree pages (loosely) have two attributes: [`PageType`] and
//! [`PageFamily`]. Any combination of these attributes are possible, however will result in
//! different data and behaviour in the node. For a given b-tree, all [`Page`]s within the
//! b-tree will have the same attributes, as will all [`Cell`]s.
//!
//! [`PageType`] refers to the function of the page. [`Index`] pages will contain a key of some
//! arbitrary length. [`Table`] pages include a variable length key, and an additional payload
//! for each key.
//!
//! [`PageFamily`] refers to the 'kind' of node in a b-tree. [`Leaf`] nodes do not contain any
//! other pointers, whilst [`Interior`] nodes have pointers to other nodes in conjunction to
//! their keys.

mod cell;
mod page_flag;
pub mod storage;

use std::{fmt::Debug, num::NonZero};

use cuisiner::{Cuisiner, CuisinerError};
use thiserror::Error;
use zerocopy::BigEndian;

use self::{
    page_flag::*,
    storage::{PageStorage, StorageError},
};
use crate::header::{SQLITE_HEADER_SIZE, SqliteHeader};

pub use self::page_flag::{Index, Interior, Leaf, Table};

#[derive(Debug, Error)]
pub enum PagerError {
    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error(transparent)]
    Cuisiner(#[from] CuisinerError),
}

pub struct Pager {
    storage: Box<dyn PageStorage>,

    header: SqliteHeader,
}

impl Pager {
    pub fn new(mut storage: impl 'static + PageStorage) -> Result<Self, PagerError> {
        let header_bytes = storage.read_start(SQLITE_HEADER_SIZE)?;
        let header = SqliteHeader::from_bytes::<BigEndian>(&header_bytes)?;

        storage.set_page_size(*header.page_size);

        Ok(Self {
            storage: Box::new(storage),
            header,
        })
    }

    pub fn get_page_header<T: PageType, F: PageFamily>(
        &mut self,
        page_id: u32,
    ) -> Result<Page<T, F>, PagerError>
    where
        (T, F): PageFlag,
    {
        let mut page = self.storage.read_page(page_id)?;

        if page_id == 0 {
            // Since the database header resides in the first page, offset if loading the first
            // page.
            page = &page[SQLITE_HEADER_SIZE..];
        }

        Ok(Page::from_bytes::<BigEndian>(page)?)
    }
}

/// B-tree page, which will contain keys, and potentially associated data in the case of
/// [`Table`] pages. [`Interior`] pages will include a pointer to other pages.
#[derive(Cuisiner, Debug, PartialEq, Eq)]
#[cuisiner(assert(
    generics = "Index, Leaf",
    generics = "Table, Leaf",
    generics = "Index, Interior",
    generics = "Table, Interior",
    leaf(size = 8, generics = "Index, Leaf", generics = "Table, Leaf"),
    interior(size = 12, generics = "Index, Interior", generics = "Table, Interior")
))]
pub struct Page<T: PageType, F: PageFamily>
where
    (T, F): PageFlag,
{
    /// Flag indicating the kind of the page.
    #[cuisiner(assert(offset = 0, size = 1))]
    page_flag: <(T, F) as PageFlag>::Value,

    // Common fields for all pages.
    #[cuisiner(assert(offset = 1, size = 2))]
    first_freeblock: Option<NonZero<u16>>,
    #[cuisiner(assert(offset = 3, size = 2))]
    cell_count: u16,
    #[cuisiner(assert(offset = 5, size = 2))]
    cell_content_offset: NonZero<u16>,
    #[cuisiner(assert(offset = 7, size = 1))]
    fragmented_bytes: u8,

    // Fields unique to page variation.
    #[cuisiner(assert(offset = 8, size = 0))]
    type_info: T::PageData,
    #[cuisiner(assert(offset = 8, leaf(size = 0), interior(size = 4)))]
    family_info: F::PageData,
}

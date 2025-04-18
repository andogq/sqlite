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

use std::{fmt::Debug, num::NonZero};

use assert_layout::assert_layout;
use thiserror::Error;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned, big_endian::U16};

use self::page_flag::*;

pub use self::page_flag::{Index, Interior, Leaf, Table};

use super::util::Optional;

/// B-tree page, which will contain keys, and potentially associated data in the case of
/// [`Table`] pages. [`Interior`] pages will include a pointer to other pages.
#[derive(Clone, Debug, PartialEq, Eq, FromBytes, KnownLayout, Immutable)]
#[assert_layout(
    generics = "Index, Leaf",
    generics = "Table, Leaf",
    generics = "Index, Interior",
    generics = "Table, Interior",
    leaf(size = 8, generics = "Index, Leaf", generics = "Table, Leaf"),
    interior(size = 12, generics = "Index, Interior", generics = "Table, Interior")
)]
#[repr(C)]
pub struct PageHeader<T: PageType, F: PageFamily>
where
    (T, F): PageFlag,
{
    /// Flag indicating the kind of the page.
    #[assert_layout(offset = 0, size = 1)]
    page_flag: <(T, F) as PageFlag>::Value,

    // Common fields for all pages.
    #[assert_layout(offset = 1, size = 2)]
    first_freeblock: Optional<U16>,
    #[assert_layout(offset = 3, size = 2)]
    cell_count: U16,
    #[assert_layout(offset = 5, size = 2)]
    cell_content_offset: CellContentOffset,
    #[assert_layout(offset = 7, size = 1)]
    fragmented_bytes: u8,

    // Fields unique to page variation.
    #[assert_layout(offset = 8, size = 0)]
    type_info: T::PageData,
    #[assert_layout(offset = 8, leaf(size = 0), interior(size = 4))]
    family_info: F::PageData,
}

#[derive(Clone, Copy, Debug, IntoBytes, FromBytes, Immutable, Unaligned, PartialEq, Eq)]
#[repr(transparent)]
pub struct CellContentOffset(U16);
impl CellContentOffset {
    pub fn get(&self) -> NonZero<u32> {
        NonZero::new(self.0.get() as u32).unwrap_or(NonZero::new(65536).unwrap())
    }
}

#[derive(Clone, Debug)]
pub struct Page<'a, T: PageType, F: PageFamily>
where
    (T, F): PageFlag,
{
    header: &'a PageHeader<T, F>,
    content: &'a [u8],
}

impl<'a, T: PageType, F: PageFamily> Page<'a, T, F>
where
    (T, F): PageFlag,
    PageHeader<T, F>: FromBytes + KnownLayout + Immutable,
{
    pub fn try_read(buf: &'a [u8]) -> Result<Self, PageError> {
        let (header, content) = PageHeader::<T, F>::ref_from_prefix(buf)?;

        Ok(Self { header, content })
    }
}

#[derive(Debug, Clone)]
pub enum AnyPage<'a> {
    IndexLeaf(Page<'a, Index, Leaf>),
    TableLeaf(Page<'a, Table, Leaf>),
    IndexInterior(Page<'a, Index, Interior>),
    TableInterior(Page<'a, Table, Interior>),
}

impl<'a> AnyPage<'a> {
    pub fn try_read(buf: &'a [u8]) -> Result<Self, PageError> {
        [
            (
                <(Index, Leaf) as PageFlag>::Value::value(),
                (|bytes| Ok(AnyPage::IndexLeaf(Page::try_read(bytes)?)))
                    as fn(&'a [u8]) -> Result<_, _>,
            ),
            (<(Table, Leaf) as PageFlag>::Value::value(), |bytes| {
                Ok(AnyPage::TableLeaf(Page::try_read(bytes)?))
            }),
            (<(Index, Interior) as PageFlag>::Value::value(), |bytes| {
                Ok(AnyPage::IndexInterior(Page::try_read(bytes)?))
            }),
            (<(Table, Interior) as PageFlag>::Value::value(), |bytes| {
                Ok(AnyPage::TableInterior(Page::try_read(bytes)?))
            }),
        ]
        .into_iter()
        .find(|(flag, _)| *flag == buf[0])
        .ok_or(PageError::UnknownFlag(buf[0]))
        .and_then(|(_, read)| read(buf))
    }
}

#[derive(Debug, Clone)]
pub enum TablePage<'a> {
    Leaf(Page<'a, Table, Leaf>),
    Interior(Page<'a, Table, Interior>),
}

#[derive(Debug, Error)]
pub enum PageError {
    #[error("unknown page flag: {0}")]
    UnknownFlag(u8),
    #[error("invalid alignment for page")]
    Alignment,
    #[error("invalid size for page")]
    Size,
}

impl<Src, Dst: ?Sized> From<zerocopy::CastError<Src, Dst>> for PageError {
    fn from(error: zerocopy::CastError<Src, Dst>) -> Self {
        match error {
            zerocopy::ConvertError::Alignment(_) => PageError::Alignment,
            zerocopy::ConvertError::Size(_) => PageError::Size,
            zerocopy::ConvertError::Validity(_) => unreachable!(),
        }
    }
}

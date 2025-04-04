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

pub mod storage;

use std::{fmt::Debug, num::NonZero};

use cuisiner::{ConstU8, Cuisiner, CuisinerError};
use thiserror::Error;
use zerocopy::BigEndian;

use self::storage::{PageStorage, StorageError};
use crate::header::{SQLITE_HEADER_SIZE, SqliteHeader};

pub use self::{
    page_family::{Interior, Leaf},
    page_type::{Index, Table},
};

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
        (T, F): PageFlagValue,
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
    (T, F): PageFlagValue,
{
    /// Flag indicating the kind of the page.
    #[cuisiner(assert(offset = 0, size = 1))]
    page_flag: <(T, F) as PageFlagValue>::Value,

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

/// Trait to contain the page flag value for a given [`PageType`] and [`PageFamily`]
/// combination.
pub trait PageFlagValue {
    type Value: Cuisiner + Debug + PartialEq + Eq;
}

/// Helper macro to assert that a page of a given type is the correct size, and assign a page
/// flag value. According to the SQLite documentation, [`Leaf`] pages are 8 bytes, whilst
/// [`Interior`] pages are 12 bytes.
macro_rules! impl_page_type {
    (($page_type:ty, $page_family:ty) { size = $size:expr, value = $value:expr }) => {
        static_assertions::const_assert_eq!(
            std::mem::size_of::<<Page<$page_type, $page_family> as Cuisiner>::Raw<BigEndian>>(),
            $size,
        );

        impl PageFlagValue for ($page_type, $page_family) {
            type Value = ConstU8<$value>;
        }
    };
}

impl_page_type!((Index, Interior) {
    size = 12,
    value = 0x02
});
impl_page_type!((Index, Leaf) {
    size = 8,
    value = 0x05
});
impl_page_type!((Table, Interior) {
    size = 12,
    value = 0x0a
});
impl_page_type!((Table, Leaf) {
    size = 8,
    value = 0x0d
});

/// A piece of data within a [`Page`]. Although there are two attributes, the cell's 'payload'
/// is dependent on the combination of the two attributes. See [`Payload`].
struct Cell<T: PageType, F: PageFamily>
where
    (T, F): Payload,
{
    type_data: T::CellData,
    family_data: F::CellData,
    payload_data: <(T, F) as Payload>::Data,
}

mod page_type {
    use super::*;

    /// Marker trait for different 'types' of pages. Type corresponds to the purpose of the page,
    /// such as [`Table`] or [`Index`].
    pub trait PageType {
        /// Data required for [`Page`]s of this type.
        type PageData: Cuisiner;

        /// Data contained in [`Cell`]s originating from [`Page`]s of this type.
        type CellData: Cuisiner;
    }

    /// Each entry in a table has a 64 bit key, and arbitrary data.
    #[derive(Debug)]
    pub enum Table {}
    impl PageType for Table {
        type PageData = ();
        type CellData = TableCellData;
    }

    #[derive(Cuisiner)]
    pub struct TableCellData {
        // TODO: make varint
        rowid: u32,
    }

    impl<F: PageFamily> Cell<Table, F>
    where
        (Table, F): Payload,
    {
        pub fn get_row_id(&self) -> u32 {
            self.type_data.rowid
        }
    }

    /// Each entry contains an arbitrarily long key.
    #[derive(Debug)]
    pub enum Index {}
    impl PageType for Index {
        type PageData = ();
        type CellData = ();
    }
}
use page_type::*;

mod page_family {
    use super::*;

    /// Marker trait for different 'families' of pages. The family indicates the relation to other
    /// [`Page`]s in the B-Tree, such as [`Leaf`] if it has no descendants, or [`Interior`] if it
    /// does.
    pub trait PageFamily {
        /// Data required for [`Page`]s of this family.
        type PageData: Cuisiner;

        /// Data contained in [`Cell`]s originating from [`Page`]s of this family.
        type CellData: Cuisiner;
    }

    /// A leaf [`Page`] has no pointers to other pages, however it's [`Cell`]s hold keys and/or content
    /// for [`Table`]s and [`Index`]es.
    #[derive(Debug)]
    pub enum Leaf {}
    impl PageFamily for Leaf {
        type PageData = ();
        type CellData = ();
    }

    /// An interior [`Page`] contains keys, and pointers to child [`Page`]s.
    #[derive(Debug)]
    pub struct Interior {}
    impl PageFamily for Interior {
        type PageData = InteriorPageData;
        type CellData = InteriorCellData;
    }

    #[derive(Cuisiner, Debug)]
    pub struct InteriorPageData {
        right_pointer: u32,
    }

    #[derive(Cuisiner)]
    pub struct InteriorCellData {
        left_child: u32,
    }

    impl<T: PageType> Cell<T, Interior>
    where
        (T, Interior): Payload,
    {
        pub fn get_left_child(&self) -> u32 {
            self.family_data.left_child
        }
    }
}
use page_family::*;

mod cell_payload {
    use super::*;

    pub struct PayloadCellData {
        // TODO: varint
        length: u32,
        bytes: Vec<u8>,
        overflow: u32,
    }

    /// The type of the payload differs depending on [`PageType`] and [`PageFamily`]. This
    /// trait is to be implemented for any combination of the two attributes.
    pub trait Payload {
        /// The type of the data for an attribute combination.
        type Data;
    }
    impl Payload for (Table, Leaf) {
        type Data = PayloadCellData;
    }
    impl Payload for (Table, Interior) {
        type Data = ();
    }
    impl Payload for (Index, Leaf) {
        type Data = PayloadCellData;
    }
    impl Payload for (Index, Interior) {
        type Data = PayloadCellData;
    }

    impl<T: PageType, F: PageFamily> Cell<T, F>
    where
        (T, F): Payload<Data = PayloadCellData>,
    {
        pub fn get_length(&self) -> u32 {
            self.payload_data.length
        }

        pub fn get_bytes(&self) -> &[u8] {
            &self.payload_data.bytes
        }

        pub fn get_overflow(&self) -> u32 {
            self.payload_data.overflow
        }
    }
}
use cell_payload::*;

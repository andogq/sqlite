pub mod storage;

use std::num::NonZero;

use anyhow::Error;
use cuisiner::{Cuisiner, CuisinerError};
use derive_more::{Deref, TryFrom};
use static_assertions::const_assert_eq;
use thiserror::Error;
use zerocopy::{BigEndian, FromBytes, big_endian::*};

use self::storage::{PageStorage, StorageError};
use crate::header::{SQLITE_HEADER_SIZE, SqliteHeader};

#[derive(Debug, Error)]
pub enum PagerError {
    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error(transparent)]
    PageHeader(#[from] PageHeaderError),

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

    pub fn get_page_header(&mut self, page_id: u32) -> Result<PageHeader, PagerError> {
        let mut page = self.storage.read_page(page_id)?;

        if page_id == 0 {
            // Since the database header resides in the first page, offset if loading the first
            // page.
            page = &page[SQLITE_HEADER_SIZE..];
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

mod page2 {
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

    use std::{marker::PhantomData, num::NonZero};

    /// B-tree page, which will contain keys, and potentially associated data in the case of
    /// [`Table`] pages. [`Interior`] pages will include a pointer to other pages.
    struct Page<T: PageType, F: PageFamily> {
        // Common fields for all pages.
        first_freeblock: Option<NonZero<u16>>,
        cell_count: u16,
        cell_content_offset: NonZero<u16>,
        fragmented_bytes: u8,

        // Fields unique to page variation.
        type_info: T::PageData,
        family_info: F::PageData,
    }

    impl<T: PageType, F: PageFamily> AsRaw for Page<T, F> {
        type Raw = raw::RawPage<T, F>;

        fn try_from_raw(raw: Self::Raw) -> Self {
            Self {
                first_freeblock: <_ as AsRaw>::try_from_raw(raw.first_freeblock),
                cell_count: <_ as AsRaw>::try_from_raw(raw.cell_count),
                cell_content_offset: <_ as AsRaw>::try_from_raw(raw.cell_content_offset),
                fragmented_bytes: <_ as AsRaw>::try_from_raw(raw.fragmented_bytes),
                type_info: <_ as AsRaw>::try_from_raw(raw.type_info),
                family_info: <_ as AsRaw>::try_from_raw(raw.family_info),
            }
        }
    }

    mod raw {
        use derive_where::derive_where;
        use static_assertions::const_assert_eq;
        use zerocopy::{FromBytes, big_endian::*};

        use super::*;

        #[derive(FromBytes)]
        #[derive_where(Clone, Debug)]
        #[repr(C)]
        pub struct RawPage<T: PageType, F: PageFamily> {
            pub page_type: u8,
            pub first_freeblock: U16,
            pub cell_count: U16,
            pub cell_content_offset: U16,
            pub fragmented_bytes: u8,

            pub type_info: <T::PageData as AsRaw>::Raw,
            pub family_info: <F::PageData as AsRaw>::Raw,
        }

        const_assert_eq!(size_of::<RawPage<Table, Leaf>>(), 8);
        const_assert_eq!(size_of::<RawPage<Index, Leaf>>(), 8);
        const_assert_eq!(size_of::<RawPage<Table, Interior>>(), 12);
        const_assert_eq!(size_of::<RawPage<Index, Interior>>(), 12);
    }

    trait AsRaw {
        type Raw: Clone + std::fmt::Debug + FromBytes;

        fn try_from_raw(raw: Self::Raw) -> Self;
    }

    impl AsRaw for () {
        type Raw = ();

        fn try_from_raw(raw: Self::Raw) -> Self {
            raw
        }
    }

    macro_rules! raw_number {
        ($ty:ty: $raw:ty) => {
            raw_number!($ty: $raw => |n| n.get());
        };

        ($ty:ty: $raw:ty => |$i:ident| $convert:expr) => {
            impl AsRaw for $ty {
                type Raw = $raw;

                fn try_from_raw($i: Self::Raw) -> Self {
                    $convert
                }
            }

            impl AsRaw for Option<NonZero<$ty>> {
                type Raw = $raw;

                fn try_from_raw(raw: Self::Raw) -> Self {
                    let n = <$ty as AsRaw>::try_from_raw(raw);
                    NonZero::try_from(n).ok()
                }
            }

            impl AsRaw for NonZero<$ty> {
                type Raw = $raw;

                fn try_from_raw(raw: Self::Raw) -> Self {
                    <Option<NonZero<$ty>> as AsRaw>::try_from_raw(raw).unwrap()
                }
            }
        };
    }

    raw_number!(u8: u8 => |n| n);
    raw_number!(u16: zerocopy::big_endian::U16);
    raw_number!(u32: zerocopy::big_endian::U32);
    raw_number!(u64: zerocopy::big_endian::U64);
    raw_number!(u128: zerocopy::big_endian::U128);

    struct PageFlag<T: PageType, F: PageFamily>(PhantomData<fn() -> (T, F)>);

    macro_rules! impl_page_flag {
        (($ty:ident, $fam:ident) = $flag:expr) => {
            impl AsRaw for PageFlag<$ty, $fam> {
                type Raw = u8;

                fn try_from_raw(raw: Self::Raw) -> Self {
                    if raw != $flag {
                        panic!("unvalid flag {raw}");
                    }

                    Self(PhantomData::default())
                }
            }
        };
    }
    impl_page_flag!((Index, Interior) = 0x02);
    impl_page_flag!((Index, Leaf) = 0x05);
    impl_page_flag!((Table, Interior) = 0x0a);
    impl_page_flag!((Table, Leaf) = 0x0d);

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
            type PageData: AsRaw;

            /// Data contained in [`Cell`]s originating from [`Page`]s of this type.
            type CellData: AsRaw;
        }

        /// Each entry in a table has a 64 bit key, and arbitrary data.
        pub enum Table {}
        impl PageType for Table {
            type PageData = ();
            type CellData = TableCellData;
        }

        pub struct TableCellData {
            // TODO: make varint
            rowid: u32,
        }

        #[derive(Clone, Debug, FromBytes)]
        #[repr(C)]
        pub struct RawTableCellData {
            // TODO: Varint
            rowid: zerocopy::big_endian::U32,
        }

        impl AsRaw for TableCellData {
            type Raw = RawTableCellData;

            fn try_from_raw(raw: Self::Raw) -> Self {
                todo!()
            }
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
            type PageData: AsRaw;

            /// Data contained in [`Cell`]s originating from [`Page`]s of this family.
            type CellData: AsRaw;
        }

        /// A leaf [`Page`] has no pointers to other pages, however it's [`Cell`]s hold keys and/or content
        /// for [`Table`]s and [`Index`]es.
        pub enum Leaf {}
        impl PageFamily for Leaf {
            type PageData = ();
            type CellData = ();
        }

        /// An interior [`Page`] contains keys, and pointers to child [`Page`]s.
        pub struct Interior {}
        impl PageFamily for Interior {
            type PageData = InteriorPageData;
            type CellData = InteriorCellData;
        }

        pub struct InteriorPageData {
            right_pointer: u32,
        }

        #[derive(Clone, Debug, FromBytes)]
        #[repr(C)]
        pub struct RawInteriorPageData {
            right_pointer: zerocopy::big_endian::U32,
        }

        impl AsRaw for InteriorPageData {
            type Raw = RawInteriorPageData;

            fn try_from_raw(raw: Self::Raw) -> Self {
                todo!()
            }
        }

        pub struct InteriorCellData {
            left_child: u32,
        }

        #[derive(Clone, Debug, FromBytes)]
        #[repr(C)]
        pub struct RawInteriorCellData {
            left_child: zerocopy::big_endian::U32,
        }

        impl AsRaw for InteriorCellData {
            type Raw = RawInteriorCellData;

            fn try_from_raw(raw: Self::Raw) -> Self {
                todo!()
            }
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
    use zerocopy::FromBytes;

    fn my_test(c1: Cell<Table, Leaf>, c2: Cell<Index, Leaf>) {
        c1.get_row_id();
    }
}

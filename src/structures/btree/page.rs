use std::marker::PhantomData;

use assert_layout::assert_layout;
use thiserror::Error;
use zerocopy::{
    FromBytes, Immutable, KnownLayout,
    big_endian::{U16, U32},
};

use crate::disk::{PageBuffer, PageSlice};

use super::{Index, PageType, Table, TreeKind};

#[derive(Clone)]
pub struct Page<K: TreeKind> {
    pub disk_page: crate::disk::Page,
    pub kind: PhantomData<K>,
}

impl<K: TreeKind> Page<K> {
    pub fn new(disk_page: crate::disk::Page) -> Self {
        Self {
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
    buf: PageBuffer<'b>,
}

impl<'p, 'b, K: TreeKind> PageOperation<'p, 'b, K>
where
    'p: 'b,
{
    pub fn new(page: &'p Page<K>) -> Self {
        Self {
            page,
            buf: page.disk_page.buffer(),
        }
    }

    pub fn header(&self) -> (&PageHeader<K>, Option<u32>, &[u8]) {
        // Read out the base of the header.
        let (header, buf) = PageHeader::<K>::read_from_prefix(&self.buf).unwrap();

        // Optionally read the right pointer.
        let (right_pointer, buf) = match header.get_page_type() {
            PageType::Interior => {
                // Fetch the right pointer which is present after the header.
                let (right_pointer, buf) = U32::ref_from_prefix(buf).unwrap();
                (Some(right_pointer.get()), buf)
            }
            PageType::Leaf => (None, buf),
        };

        (header, right_pointer, buf)
    }

    pub fn get_cell_buffer(&self, cell_number: usize) -> PageSlice {
        let (header, _, buf) = self.header();

        if cell_number >= header.cell_count.get() as usize {
            panic!("offset larger than available cells");
        }

        let (cell_pointer_array, _) =
            <[U16]>::ref_from_prefix_with_elems(buf, header.cell_count.get() as usize).unwrap();

        // Select the relevant area of the cell content area.
        let offset = cell_pointer_array[cell_number].get() as usize;

        self.page.disk_page.slice(offset..)
    }

    pub fn get_cell(&self, cell_number: usize) -> (Option<u32>, K::Cell<'_>) {
        let (header, _, buf) = self.header();

        if cell_number >= header.cell_count.get() as usize {
            panic!("offset larger than available cells");
        }

        let (cell_pointer_array, _) =
            <[U16]>::ref_from_prefix_with_elems(buf, header.cell_count.get() as usize).unwrap();
        let cell_content_area = &self.buf[header.cell_content_offset.get() as usize..];

        // Select the relevant area of the cell content area.
        let offset = cell_pointer_array[cell_number].get() as usize;
        let buf = &cell_content_area[offset..];

        let page_type = header.get_page_type();

        let (left_pointer, buf) = match page_type {
            PageType::Interior => {
                let (left_pointer, buf) = U32::ref_from_prefix(buf).unwrap();
                (Some(left_pointer.get()), buf)
            }
            PageType::Leaf => (None, buf),
        };

        let (cell, _) = K::Cell::from_buffer(buf, page_type);

        (left_pointer, cell)
    }
}

pub struct TableCell<'p> {
    /// Row ID.
    rowid: VarInt,
    /// Payload of the cell, only present on leaf pages.
    payload: Option<Payload<'p>>,
}

impl<'p> PageCell<'p> for TableCell<'p> {
    fn from_buffer(buf: &'p [u8], page_type: PageType) -> (Self, &'p [u8]) {
        let (length_or_rowid, buf) = VarInt::from_buffer(buf);

        match page_type {
            PageType::Interior => (
                Self {
                    rowid: length_or_rowid,
                    payload: None,
                },
                buf,
            ),
            PageType::Leaf => {
                let length = length_or_rowid;
                let (rowid, buf) = VarInt::from_buffer(buf);
                let (payload, buf) = Payload::from_buf_with_length(buf, length);

                (
                    Self {
                        rowid,
                        payload: Some(payload),
                    },
                    buf,
                )
            }
        }
    }

    fn get_debug(&self) -> usize {
        self.rowid.0 as usize
    }
}

pub struct IndexCell<'p> {
    /// Payload of the cell.
    payload: Payload<'p>,
}

impl<'p> PageCell<'p> for IndexCell<'p> {
    fn from_buffer(buf: &'p [u8], page_type: PageType) -> (Self, &'p [u8]) {
        let (payload, buf) = Payload::from_buf(buf);
        (Self { payload }, buf)
    }

    fn get_debug(&self) -> usize {
        self.payload.length.0 as usize
    }
}

pub trait PageCell<'p>: Sized {
    fn from_buffer(buf: &'p [u8], page_type: PageType) -> (Self, &'p [u8]);
    fn get_debug(&self) -> usize;
}

pub struct Payload<'p> {
    length: VarInt,
    payload: &'p [u8],
    overflow_page: Option<usize>,
}

impl<'p> Payload<'p> {
    fn from_buf_with_length(buf: &'p [u8], length: VarInt) -> (Self, &'p [u8]) {
        // TODO: Calculate payload length

        (
            Self {
                length,
                payload: &buf[0..0],
                overflow_page: None,
            },
            buf,
        )
    }

    fn from_buf(buf: &'p [u8]) -> (Self, &'p [u8]) {
        let (length, buf) = VarInt::from_buffer(buf);

        Self::from_buf_with_length(buf, length)
    }
}

struct VarInt(i64);
impl VarInt {
    pub fn from_buffer(mut buf: &[u8]) -> (Self, &[u8]) {
        let mut value: i64 = 0;

        for (i, b) in buf.iter().take(9).enumerate() {
            let mask = 0xffu8 >> (1 - (i / 8));
            let shift = 7 + (i / 8);
            buf = &buf[1..];

            value = (value << shift) + (b & mask) as i64;

            if b >> 7 == 0 {
                break;
            }
        }

        (Self(value), buf)
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

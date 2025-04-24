pub mod index;
pub mod table;

use std::cmp::Ordering;

use zerocopy::{FromBytes, big_endian::U32};

use crate::structures::{VarInt, header::SqliteHeader};

use super::{PageType, TreeKind};

pub use self::{
    index::{Index, IndexCell},
    table::{Table, TableCell},
};

pub trait PageCell<'p>: Sized {
    fn from_buffer(ctx: &'_ PageCtx, buf: &'p [u8], page_type: PageType) -> Self;
    fn get_debug(&self) -> usize;
}

#[derive(Clone, Debug)]
pub struct Payload<'p> {
    /// Total size of the payload, including any overflow.
    payload_size: usize,

    /// Payload included in the page.
    payload: &'p [u8],

    /// Page of the overflow.
    overflow_page: Option<usize>,
}

impl<'p> Payload<'p> {
    /// Read the payload from the start of the provided buffer.
    fn from_buf_with_payload_size<K: PayloadCalculation>(
        ctx: &'_ PageCtx,
        buf: &'p [u8],
        payload_size: usize,
    ) -> Self {
        // U: The usable size of a database page (the total page size less the reserved space at
        // the end of each page).
        let usable_space = ctx.page_size as usize - ctx.page_end_padding as usize;

        // X: The maximum amount of payload that can be stored directly on the b-tree page without
        // spilling onto an overflow page.
        let max_page_payload = K::max_page_payload(usable_space);

        // M: The minimum amount of payload that must be stored onthe btree page before spilling is
        // allowed.
        let min_page_payload = ((usable_space - 12) * 32 / 255) - 23;

        let k = (min_page_payload as isize
            + ((payload_size as isize - min_page_payload as isize) % (usable_space as isize - 4)))
            as usize;

        let (stored, overflow_page) = match (
            (payload_size).cmp(&max_page_payload),
            k.cmp(&max_page_payload),
        ) {
            (Ordering::Less | Ordering::Equal, _) => (payload_size, None),
            (Ordering::Greater, Ordering::Less | Ordering::Equal) => (k, Some(payload_size - k)),
            (Ordering::Greater, Ordering::Greater) => {
                (min_page_payload, Some(payload_size - min_page_payload))
            }
        };

        let payload = &buf[..stored];
        let overflow_page = overflow_page.map(|_| {
            let (overflow_page, _) = U32::ref_from_prefix(&buf[stored..]).unwrap();
            overflow_page.get() as usize
        });

        Self {
            payload_size,
            payload,
            overflow_page,
        }
    }

    fn from_buf<K: PayloadCalculation>(ctx: &'_ PageCtx, buf: &'p [u8]) -> Self {
        let (length, buf) = VarInt::from_buffer(buf);

        Self::from_buf_with_payload_size::<K>(ctx, buf, *length as usize)
    }

    pub fn debug(&self) {
        assert!(self.overflow_page.is_none());

        let (header_length, buf) = VarInt::from_buffer(self.payload);
        let remaining_header_length = *header_length as usize - (self.payload.len() - buf.len());

        let mut header_buf = &buf[..remaining_header_length];

        while !header_buf.is_empty() {
            let (serial_type, buf) = VarInt::from_buffer(header_buf);
            header_buf = buf;

            println!(
                "{}",
                match *serial_type {
                    0 => "NULL",
                    1 => "i8",
                    2 => "i16",
                    3 => "i24",
                    4 => "i32",
                    5 => "i48",
                    6 => "i64",
                    7 => "f64",
                    8 => "0",
                    9 => "1",
                    10 | 11 => "reserved",
                    n @ 12.. if n % 2 == 0 => "BLOB",
                    n @ 13.. if n % 2 == 1 => "text",
                    _ => unreachable!(),
                }
            )
        }

        dbg!(remaining_header_length);
    }
}

trait PayloadCalculation: TreeKind {
    fn max_page_payload(usable_space: usize) -> usize;
}

impl PayloadCalculation for Table {
    fn max_page_payload(usable_space: usize) -> usize {
        usable_space - 35
    }
}

impl PayloadCalculation for Index {
    fn max_page_payload(usable_space: usize) -> usize {
        ((usable_space - 12) * 64 / 255) - 23
    }
}

/// Relevant information from the header when working with pages.
#[derive(Clone)]
pub struct PageCtx {
    page_size: u32,
    page_end_padding: u8,
    page_count: u32,
}

impl From<&SqliteHeader> for PageCtx {
    fn from(header: &SqliteHeader) -> Self {
        Self {
            page_size: header.page_size(),
            page_end_padding: header.page_end_padding(),
            page_count: header.page_count(),
        }
    }
}

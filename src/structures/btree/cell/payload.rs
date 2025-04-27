use std::{cmp::Ordering, num::NonZeroUsize};

use zerocopy::{FromBytes, big_endian::U32};

use crate::{
    memory::{
        Chain, MemoryPage,
        pager::{PageId, Pager},
    },
    structures::{VarInt, btree::TreeKind},
};

use super::{PageCtx, index::Index, table::Table};

#[derive(Clone)]
pub struct Payload {
    /// Total size of the payload, including any overflow.
    payload_size: usize,

    // Chain containing payload data,
    data: Chain,
}

impl Payload {
    pub fn data(&self) -> Chain {
        self.data.clone()
    }

    /// Read the payload from the start of the provided buffer.
    pub(super) fn from_buf_with_payload_size<K: PayloadCalculation>(
        ctx: &PageCtx,
        buf: MemoryPage,
        payload_size: usize,
        pager: Pager,
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

        // Calculate bytes stored, and bytes on overflow page.
        let (stored, overflow) = match (
            (payload_size).cmp(&max_page_payload),
            k.cmp(&max_page_payload),
        ) {
            (Ordering::Less | Ordering::Equal, _) => (payload_size, None),
            (Ordering::Greater, Ordering::Less | Ordering::Equal) => (k, Some(payload_size - k)),
            (Ordering::Greater, Ordering::Greater) => {
                (min_page_payload, Some(payload_size - min_page_payload))
            }
        };

        let payload = buf.slice(..stored);
        let overflow_page = overflow.map(|_| {
            // Read the overflow page number, which is stored at the end of the usable data.
            let buf = buf.buffer();
            let overflow_page = U32::ref_from_bytes(&buf[stored..stored + 4]).unwrap();

            PageId::new(NonZeroUsize::new(overflow_page.get() as usize).unwrap())
        });

        Self {
            payload_size,
            data: Chain::new(pager, payload, overflow_page),
        }
    }

    pub(super) fn from_buf<K: PayloadCalculation>(
        ctx: &PageCtx,
        buf: MemoryPage,
        pager: Pager,
    ) -> Self {
        let (length, buf) = VarInt::from_page(buf);

        Self::from_buf_with_payload_size::<K>(ctx, buf, *length as usize, pager)
    }

    // pub fn debug(&self) {
    //     let (header_length, buf) = VarInt::from_buffer(self.payload);
    //     let remaining_header_length = *header_length as usize - (self.payload.len() - buf.len());
    //
    //     let mut header_buf = &buf[..remaining_header_length];
    //
    //     while !header_buf.is_empty() {
    //         let (serial_type, buf) = VarInt::from_buffer(header_buf);
    //         header_buf = buf;
    //
    //         println!(
    //             "{}",
    //             match *serial_type {
    //                 0 => "NULL",
    //                 1 => "i8",
    //                 2 => "i16",
    //                 3 => "i24",
    //                 4 => "i32",
    //                 5 => "i48",
    //                 6 => "i64",
    //                 7 => "f64",
    //                 8 => "0",
    //                 9 => "1",
    //                 10 | 11 => "reserved",
    //                 n @ 12.. if n % 2 == 0 => "BLOB",
    //                 n @ 13.. if n % 2 == 1 => "text",
    //                 _ => unreachable!(),
    //             }
    //         )
    //     }
    //
    //     dbg!(remaining_header_length);
    // }
}

pub trait PayloadCalculation: TreeKind {
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

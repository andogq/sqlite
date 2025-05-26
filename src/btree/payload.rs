use std::cmp::Ordering;

use zerocopy::{FromBytes, big_endian::U32};

use crate::{
    DbCtx,
    btree::page::{Index, Page, PageType, Table},
    pager::Pager,
};

#[derive(Clone)]
pub struct Payload<T: PageType> {
    /// Total size of the payload, including any overflow.
    pub length: usize,

    /// Page that contains the start of the payload.
    base_page: Page<T>,
    /// Offset into the cell content area to the beginning of the payload.
    base_offset: usize,
    /// End offset (exclusive) of the payload within the base page.
    base_offset_end: usize,

    /// ID of the next page in the chain.
    next_page: Option<u32>,
}

impl<T: PayloadCalculation> Payload<T> {
    /// Read the payload from the start of the provided buffer.
    pub fn from_buf_with_payload_size(
        ctx: DbCtx,
        page: Page<T>,
        offset: usize,
        payload_size: usize,
    ) -> Self {
        // U: The usable size of a database page (the total page size less the reserved space at
        // the end of each page).
        let usable_space = ctx.page_size - ctx.page_end_padding;

        // X: The maximum amount of payload that can be stored directly on the b-tree page without
        // spilling onto an overflow page.
        let max_page_payload = T::max_page_payload(usable_space);

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

        // Calculate where the payload would stop
        let base_offset_end = offset + stored;

        // If overflow, determine the next page.
        let next_page = overflow.map(|_| {
            // Read the overflow page number, which is stored at the end of the usable data.
            let next_page = U32::ref_from_bytes(
                &page.cell_content_area()[base_offset_end..base_offset_end + size_of::<U32>()],
            )
            .unwrap();

            next_page.get()
        });

        Self {
            length: payload_size,
            base_page: page,
            base_offset: offset,
            base_offset_end,
            next_page,
        }
    }

    /// Copy the contents of the payload into the provided buffer. The buffer must be equal to
    /// [`Payload::length`].
    pub fn copy_to_slice(&self, _pager: Pager, buf: &mut [u8]) {
        assert_eq!(buf.len(), self.length, "provided buffer must fit payload");

        // TODO: Support overflow payloads.
        assert!(
            self.next_page.is_none(),
            "only support non-overflow payloads for now"
        );

        // Copy into the slice.
        buf.copy_from_slice(
            &self.base_page.cell_content_area()[self.base_offset..self.base_offset_end],
        );
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

pub trait PayloadCalculation: PageType {
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

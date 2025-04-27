use crate::{
    memory::{Chain, MemoryPage, pager::Pager},
    structures::{
        VarInt,
        btree::{PageType, TreeKind},
    },
};

use super::{PageCell, PageCtx, Payload};

#[derive(Clone, Copy, Debug)]
pub enum Table {}

impl TreeKind for Table {
    const MASK: u8 = 0b101;
    type Cell = TableCell;
}

pub struct TableCell {
    /// Row ID.
    rowid: VarInt,
    /// Payload of the cell, only present on leaf pages.
    payload: Option<Payload>,
}

impl PageCell for TableCell {
    fn from_buffer(ctx: &PageCtx, buf: MemoryPage, page_type: PageType, pager: Pager) -> Self {
        let (length_or_rowid, buf) = VarInt::from_page(buf);

        match page_type {
            PageType::Interior => Self {
                rowid: length_or_rowid,
                payload: None,
            },
            PageType::Leaf => {
                let length = length_or_rowid;
                let (rowid, buf) = VarInt::from_page(buf);
                let payload =
                    Payload::from_buf_with_payload_size::<Table>(ctx, buf, *length as usize, pager);

                Self {
                    rowid,
                    payload: Some(payload),
                }
            }
        }
    }

    fn payload(&self) -> Option<Chain> {
        Some(self.payload.as_ref()?.data())
    }
}

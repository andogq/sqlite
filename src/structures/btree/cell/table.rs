use crate::structures::{
    VarInt,
    btree::{PageType, TreeKind},
};

use super::{PageCell, PageCtx, Payload};

#[derive(Clone, Copy, Debug)]
pub enum Table {}

impl TreeKind for Table {
    const MASK: u8 = 0b101;
    type Cell<'p> = TableCell<'p>;
}

pub struct TableCell<'p> {
    /// Row ID.
    rowid: VarInt,
    /// Payload of the cell, only present on leaf pages.
    payload: Option<Payload<'p>>,
}

impl<'p> PageCell<'p> for TableCell<'p> {
    fn from_buffer(ctx: &'_ PageCtx, buf: &'p [u8], page_type: PageType) -> Self {
        let (length_or_rowid, buf) = VarInt::from_buffer(buf);

        match page_type {
            PageType::Interior => Self {
                rowid: length_or_rowid,
                payload: None,
            },
            PageType::Leaf => {
                let length = length_or_rowid;
                let (rowid, buf) = VarInt::from_buffer(buf);
                let payload =
                    Payload::from_buf_with_payload_size::<Table>(ctx, buf, *length as usize);

                Self {
                    rowid,
                    payload: Some(payload),
                }
            }
        }
    }

    fn get_debug(&self) -> usize {
        dbg!(&self.payload);
        *self.rowid as usize
    }
}

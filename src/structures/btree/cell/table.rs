use crate::structures::{
    VarInt,
    btree::{PageType, TreeKind},
};

use super::{PageCell, Payload};

#[derive(Debug)]
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
        *self.rowid as usize
    }
}

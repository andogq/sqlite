use crate::structures::btree::{PageType, TreeKind};

use super::{PageCell, Payload};

#[derive(Debug)]
pub enum Index {}

impl TreeKind for Index {
    const MASK: u8 = 0b010;
    type Cell<'p> = IndexCell<'p>;
}

pub struct IndexCell<'p> {
    /// Payload of the cell.
    payload: Payload<'p>,
}

impl<'p> PageCell<'p> for IndexCell<'p> {
    fn from_buffer(buf: &'p [u8], _page_type: PageType) -> (Self, &'p [u8]) {
        let (payload, buf) = Payload::from_buf(buf);
        (Self { payload }, buf)
    }

    fn get_debug(&self) -> usize {
        *self.payload.length as usize
    }
}

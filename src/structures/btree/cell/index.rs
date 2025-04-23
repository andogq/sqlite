use crate::structures::btree::{PageType, TreeKind};

use super::{PageCell, PageCtx, Payload};

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
    fn from_buffer(ctx: &'_ PageCtx, buf: &'p [u8], _page_type: PageType) -> Self {
        let payload = Payload::from_buf::<Index>(ctx, buf);
        Self { payload }
    }

    fn get_debug(&self) -> usize {
        self.payload.payload_size
    }
}

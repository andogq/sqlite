use crate::{
    memory::{Chain, MemoryPage, pager::Pager},
    structures::btree::{PageType, TreeKind},
};

use super::{PageCell, PageCtx, Payload};

#[derive(Clone, Copy, Debug)]
pub enum Index {}

impl TreeKind for Index {
    const MASK: u8 = 0b010;
    type Cell = IndexCell;
}

pub struct IndexCell {
    /// Payload of the cell.
    payload: Payload,
}

impl PageCell for IndexCell {
    fn from_buffer(ctx: &PageCtx, buf: MemoryPage, _page_type: PageType, pager: Pager) -> Self {
        let payload = Payload::from_buf::<Index>(ctx, buf, pager);
        Self { payload }
    }

    fn payload(&self) -> Option<Chain> {
        Some(self.payload.data())
    }
}

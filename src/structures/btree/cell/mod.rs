pub mod index;
mod payload;
pub mod table;

use crate::{
    memory::{Chain, MemoryPage, pager::Pager},
    structures::header::SqliteHeader,
};

use super::PageType;

pub use self::{index::Index, payload::Payload, table::Table};

pub trait PageCell: Sized {
    fn from_buffer(ctx: &PageCtx, buf: MemoryPage, page_type: PageType, pager: Pager) -> Self;
    fn payload(&self) -> Option<Chain>;
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

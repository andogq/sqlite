use std::iter;

use page::PageType;
use zerocopy::{FromBytes, big_endian::*};

use self::{
    page::{Page, PageExt, Table},
    payload::Payload,
};

use crate::{ctx::Ctx, disk::var_int::VarInt};

pub mod page;
pub mod payload;

/// Traverse a B-Tree from a root page, producing an iterator of cells.
pub fn traverse<T: Traversable>(ctx: Ctx, page: Page<T>) -> impl Iterator<Item = T::Cell> {
    let mut stack = vec![page];
    let mut leaf_iter = None;

    std::iter::from_fn(move || {
        match &mut leaf_iter {
            None => {
                match stack.pop()? {
                    Page::Leaf(leaf_page) => {
                        // Buffer all of the pointers into a vec, so they can be referred to from
                        // the iterator.
                        let ptrs = leaf_page.cell_content_pointers().collect::<Vec<_>>();
                        let ctx = ctx.clone();

                        leaf_iter = Some(ptrs.into_iter().map(move |ptr| {
                            let content = &leaf_page.cell_content_area()[ptr..];

                            T::cell_from_content(
                                ctx.clone(),
                                content,
                                leaf_page.clone().to_page(),
                                ptr,
                            )
                        }));
                    }
                    Page::Interior(interior_page) => {
                        // Capture the current end of the array, so later pages don't jump ahead.
                        let insert_point = stack.len();

                        interior_page
                            .cell_content_pointers()
                            .map({
                                let cell_content = interior_page.cell_content_area();
                                |ptr| &cell_content[ptr..]
                            })
                            .map(|cell_content| {
                                let (left_pointer, _cell_content) =
                                    U32::read_from_prefix(cell_content).unwrap();
                                left_pointer.get()
                            })
                            .chain(iter::once(interior_page.right_pointer))
                            .for_each(|ptr| {
                                stack.insert(
                                    insert_point,
                                    Page::from_buffer(ctx.pager.get_page(ptr)),
                                );
                            });
                    }
                }
            }
            Some(iter) => {
                if let Some(next) = iter.next() {
                    return Some(Some(next));
                }
                leaf_iter = None;
            }
        }

        Some(None)
    })
    .flatten()
}

pub trait Traversable: PageType {
    type Cell;

    /// Create a cell from the provided content buffer. Also includes the originating page, and the
    /// offset within the cell content area that the cell is located.
    fn cell_from_content(
        ctx: Ctx,
        content: &[u8],
        page: Page<Self>,
        cell_offset: usize,
    ) -> Self::Cell;
}

pub struct TableCell {
    pub row_id: i64,
    pub payload: Payload<Table>,
}

impl Traversable for Table {
    type Cell = TableCell;

    fn cell_from_content(
        ctx: Ctx,
        content: &[u8],
        page: Page<Self>,
        cell_offset: usize,
    ) -> Self::Cell {
        let (payload_size, buf) = VarInt::from_buffer(content);
        let (row_id, payload) = VarInt::from_buffer(buf);

        let payload_offset = cell_offset + (content.len() - payload.len());

        TableCell {
            row_id: *row_id,
            payload: Payload::from_buf_with_payload_size(
                ctx,
                page,
                payload_offset,
                *payload_size as usize,
            ),
        }
    }
}

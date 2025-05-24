pub mod cell;
pub mod page;

use std::{fmt::Debug, marker::PhantomData};

use cell::{PageCell, PageCtx, Table, table::TableCell};

use crate::memory::{
    pager::{PageId, Pager},
    *,
};

use self::page::*;

pub struct BTree<K: TreeKind> {
    pager: Pager,
    root_page: PageId,
    page_ctx: PageCtx,
    kind: PhantomData<fn() -> K>,
}

impl<K: TreeKind> BTree<K> {
    pub fn new(pager: Pager, root_page: PageId, page_ctx: PageCtx) -> Self {
        Self {
            pager,
            root_page,
            page_ctx,
            kind: PhantomData,
        }
    }

    pub fn get_page(&self, page_id: PageId) -> Page<K> {
        let disk_page = self.pager.get(page_id).unwrap().unwrap();
        Page::new(page_id, self.page_ctx.clone(), disk_page)
    }

    fn get_left_page(&self) -> Page<K> {
        let mut page = self.get_page(self.root_page);

        loop {
            let Some(left_page) = page.process(|page| {
                let (left_page, _) = page.get_cell(0, self.pager.clone());

                left_page
            }) else {
                break;
            };

            page = self.get_page(left_page);
        }

        page
    }
}

impl BTree<Table> {
    /// Fetch the cell that corresponds with the provided row ID.
    pub fn get(&self, row_id: i64) -> Option<TableCell> {
        let mut root_page = Some(self.get_page(self.root_page));

        while let Some(page) = root_page {
            root_page = page.process(|page| {
                let available_cells = page.pointer_array.len();
                let mid = available_cells / 2;

                None
            });
        }

        todo!()
    }

    pub fn scan(&self) -> impl Iterator<Item = TableCell> {
        let mut stack = Vec::new();

        // Initialise the stack with the root page
        {
            let mut page_id = self.root_page;

            loop {
                // Save this page to the stack
                stack.push((page_id, 0));

                let Some(left_page) = self.get_page(page_id).process(|page| {
                    let (left_page, _) = page.get_cell(0, self.pager.clone());

                    left_page
                }) else {
                    // If no more left page, then the current page is a leaf page.
                    break;
                };

                page_id = left_page;
            }
        }

        let mut page = Some(self.get_left_page());

        std::iter::from_fn(move || {
            let (page_id, child_i) = stack.pop()?;

            let (left_page, cell) = self.get_page(page_id).process(|page| {
                match page.header.page_type() {
                    PageType::Leaf => {
                        // Process all children, and add parent with next child
                    }
                    PageType::Interior => {
                        // Add parent and child to stack
                    }
                }

                page.get_cell(child_i, self.pager.clone())
            });

            if let Some(left_page) = left_page {
                // This is an interior page, so add it to the stack.
                stack.push((page_id, child_i + 1));
            }

            // Fetch the children from the current page
            let children = page.take()?.process(|page| {
                let child_count = page.pointer_array.len();

                (0..child_count)
                    .map(|i| page.get_cell(i, self.pager.clone()))
                    .map(|(_, cell)| cell)
                    .collect::<Vec<_>>()
            });

            // TODO: Update to next page

            Some(children.into_iter())
        })
        .flatten()
    }
}

pub struct BTreeWalker<'b, K: TreeKind> {
    tree: &'b BTree<K>,
    current_page: Page<K>,
    current_cell: usize,
}

impl<'b, K: TreeKind> BTreeWalker<'b, K> {
    pub fn new(tree: &'b BTree<K>) -> Self {
        let page = tree.get_page(tree.root_page);

        Self {
            tree,
            current_page: page,
            current_cell: 0,
        }
    }

    pub fn get_cell(&self) -> Option<K::Cell> {
        let (_, cell) = self.current_page.process(|data: PageContent<K>| {
            data.get_cell(self.current_cell, self.tree.pager.clone())
        });

        Some(cell)
    }
}

pub trait TreeKind: 'static + Clone + Debug {
    const MASK: u8;
    type Cell: PageCell;
}

#[derive(Clone, Copy)]
pub enum PageType {
    Leaf,
    Interior,
}

impl PageType {
    pub fn from_page_flag(flag: u8) -> Self {
        match (flag >> 3) & 1 == 1 {
            false => Self::Interior,
            true => Self::Leaf,
        }
    }
}

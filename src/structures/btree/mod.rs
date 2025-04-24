pub mod cell;
pub mod page;

use std::{fmt::Debug, marker::PhantomData};

use cell::{PageCell, PageCtx};

use crate::memory::{
    pager::{PageId, Pager},
    *,
};

use self::page::*;

pub struct BTree<K: TreeKind> {
    pager: Pager,
    root_page: PageId,
    kind: PhantomData<fn() -> K>,
}

impl<K: TreeKind> BTree<K> {
    pub fn new(pager: Pager, root_page: PageId) -> Self {
        Self {
            pager,
            root_page,
            kind: PhantomData,
        }
    }

    pub fn get_page(&self, page_id: PageId) -> Page<K> {
        let disk_page = self.pager.get(page_id).unwrap().unwrap();
        Page::new(page_id, disk_page)
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

    pub fn get_cell(&self) -> Option<CellRef<K>> {
        let cell = self.current_page.process(|data: PageContent<K>| {
            let offset = data.pointer_array[self.current_cell].get() as usize;
            let buf = data.page_ref.page().slice(offset..);

            CellRef {
                page: buf,
                page_type: data.header.page_type(),
                kind: PhantomData,
            }
        });

        Some(cell)
    }
}

pub struct CellRef<K: TreeKind> {
    page: MemoryPage,
    page_type: PageType,
    kind: PhantomData<fn() -> K>,
}

impl<K: TreeKind> CellRef<K> {
    pub fn get(&self, ctx: &PageCtx) -> usize {
        let buf = self.page.buffer();
        let cell = K::Cell::from_buffer(ctx, &buf, self.page_type);
        cell.get_debug()
    }
}

pub trait TreeKind: 'static + Clone + Debug {
    const MASK: u8;
    type Cell<'p>: PageCell<'p>;
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

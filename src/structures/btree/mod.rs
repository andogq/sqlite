pub mod page;

use std::marker::PhantomData;

use zerocopy::{FromBytes, big_endian::U32};

use crate::disk::{PageId, SomePager};

use self::page::*;

pub struct BTree<K: TreeKind> {
    pager: Box<dyn SomePager>,
    kind: PhantomData<fn() -> K>,
}

impl<K: TreeKind> BTree<K> {
    pub fn new(pager: impl 'static + SomePager) -> Self {
        Self {
            pager: Box::new(pager),
            kind: PhantomData,
        }
    }

    pub fn new_with_pager(pager: Box<dyn SomePager>) -> Self {
        Self {
            pager,
            kind: PhantomData,
        }
    }

    pub fn get_page(&mut self, page_id: PageId) -> Page<'_, K> {
        let buf = self.pager.get(page_id).unwrap().unwrap();

        let (header, data) = PageHeader::read_from_prefix(buf).unwrap();

        let (right_pointer, data) = match header.get_page_type() {
            PageType::Interior => {
                // Fetch the right pointer which is present after the header.
                let (right_pointer, data) = U32::ref_from_prefix(data).unwrap();
                (Some(right_pointer.get()), data)
            }
            PageType::Leaf => (None, data),
        };

        Page {
            header,
            right_pointer,
            data,
            kind: PhantomData,
        }
    }
}

pub trait TreeKind {
    const MASK: u8;
}

#[derive(Debug)]
pub enum Table {}
#[derive(Debug)]
pub enum Index {}

impl TreeKind for Table {
    const MASK: u8 = 0b101;
}
impl TreeKind for Index {
    const MASK: u8 = 0b010;
}

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

pub struct Payload<'p> {
    size: u64,
    payload: &'p [u8],
    overflow: Option<u32>,
}

pub struct Cell<'p> {
    left_pointer: Option<u32>,
    payload: Option<Payload<'p>>,
}

pub mod memory;
pub mod structures;
mod v2;

use std::{fs::File, iter};

use v2::{
    btree::{InteriorPage, LeafPage, Page, PageExt, Table},
    disk::var_int::VarInt,
    pager::Pager,
};
use zerocopy::{FromBytes, big_endian::*};

// use structures::{
//     VarInt,
//     btree::cell::{PageCell, PageCtx, Table},
// };

// use crate::{
//     memory::pager::*,
//     structures::btree::{BTree, BTreeWalker},
// };

const DATABASE: &str = "test.db";

trait Scan {
    type Cell<'a>
    where
        Self: 'a;

    fn scan(&self, pager: Pager, f: impl FnMut(Self::Cell<'_>) + Clone);
}

struct TableCell<'buf> {
    row_id: i64,
    payload_length: usize,
    payload: &'buf [u8],
}

impl Scan for LeafPage<Table> {
    type Cell<'a> = TableCell<'a>;

    // TODO: Return an iterator over `Self::Cell<'a>` once `Page`s are stored somewhere and
    // referenced with Rc.
    fn scan(&self, _pager: Pager, mut f: impl FnMut(Self::Cell<'_>) + Clone) {
        let cell_content = self.cell_content_area();

        self.cell_content_pointers()
            .map(|ptr| &cell_content[ptr..])
            .for_each(|cell_content| {
                let (payload_length, cell_content) = VarInt::from_buffer(cell_content);
                let (row_id, payload) = VarInt::from_buffer(cell_content);

                // TODO: Trim payload and account for overflow

                f(TableCell {
                    row_id: *row_id,
                    payload_length: *payload_length as usize,
                    payload,
                })
            });
    }
}

impl Scan for InteriorPage<Table> {
    type Cell<'a> = TableCell<'a>;

    fn scan(&self, pager: Pager, f: impl FnMut(Self::Cell<'_>) + Clone) {
        self.cell_content_pointers()
            .map({
                let cell_content = self.cell_content_area();
                |ptr| &cell_content[ptr..]
            })
            .map(|cell_content| {
                let (left_pointer, _cell_content) = U32::read_from_prefix(cell_content).unwrap();
                left_pointer.get()
            })
            .chain(iter::once(self.right_pointer))
            .for_each(move |page_id| {
                let buf = pager.get_page(page_id);
                Page::<Table>::from_buffer(buf).scan(pager.clone(), f.clone())
            })
    }
}

impl Scan for Page<Table> {
    type Cell<'a> = TableCell<'a>;

    fn scan(&self, pager: Pager, f: impl FnMut(Self::Cell<'_>) + Clone) {
        match self {
            Page::Leaf(leaf_page) => {
                leaf_page.scan(pager, f);
            }
            Page::Interior(interior_page) => interior_page.scan(pager.clone(), f),
        }
    }
}

fn main() {
    let file = File::open(DATABASE).unwrap();

    let pager = Pager::new(file);

    {
        // Read the first page into memory.
        let root_page = pager.get_page(0);

        let page = Page::<Table>::from_buffer(root_page);
        dbg!(page.cell_count);

        page.scan(pager, |cell| {
            println!(
                "row id: {}, payload length: {}",
                cell.row_id, cell.payload_length
            );
        });
    }

    // let pager = Pager::bootstrap(file).unwrap();
    // let header = pager.get_header().unwrap();
    //
    // let btree = BTree::<Table>::new(
    //     pager,
    //     PageId::FIRST,
    //     header.header(|header| PageCtx::from(header)),
    // );
    //
    // let walker = BTreeWalker::new(&btree);
    // let cell = walker.get_cell().unwrap();
    //
    // let cell_content = cell.payload().unwrap();
    //
    // let mut header_length_buf = [0; 9];
    // cell_content.copy_to_slice(0, &mut header_length_buf);
    //
    // let (header_length, _) = VarInt::from_buffer(&header_length_buf);
    //
    // let mut header_buf = vec![0; *header_length as usize];
    // cell_content.copy_to_slice(0, &mut header_buf);
    //
    // let mut header = header_buf.as_slice();
    // while !header.is_empty() {
    //     let (serial_type, rest) = VarInt::from_buffer(header);
    //     header = rest;
    //
    //     println!(
    //         "{}",
    //         match *serial_type {
    //             0 => "NULL",
    //             1 => "i8",
    //             2 => "i16",
    //             3 => "i24",
    //             4 => "i32",
    //             5 => "i48",
    //             6 => "i64",
    //             7 => "f64",
    //             8 => "0",
    //             9 => "1",
    //             10 | 11 => "reserved",
    //             n @ 12.. if n % 2 == 0 => "BLOB",
    //             n @ 13.. if n % 2 == 1 => "text",
    //             _ => unreachable!(),
    //         }
    //     );
    // }
}

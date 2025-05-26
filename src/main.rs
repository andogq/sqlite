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
    fn scan(&self, pager: &mut Pager);
}

impl Scan for LeafPage<Table> {
    fn scan(&self, _pager: &mut Pager) {
        let cell_content = self.cell_content_area();

        self.cell_content_pointers()
            .map(|ptr| &cell_content[ptr..])
            .map(|cell_content| {
                let (payload_length, cell_content) = VarInt::from_buffer(cell_content);
                let (row_id, _cell_content) = VarInt::from_buffer(cell_content);

                (*row_id, *payload_length)
            })
            .for_each(|(row_id, payload_length)| {
                println!("Found row {row_id} (length: {payload_length})");
            });
    }
}

impl Scan for InteriorPage<Table> {
    fn scan(&self, pager: &mut Pager) {
        let cell_content = self.cell_content_area();

        self.cell_content_pointers()
            .map(|ptr| &cell_content[ptr..])
            .map(|cell_content| {
                let (left_pointer, _cell_content) = U32::read_from_prefix(cell_content).unwrap();
                left_pointer.get()
            })
            .chain(iter::once(self.right_pointer))
            .fold(pager.new_page_buffer(), |mut temp_page, page_id| {
                println!("child page: {page_id}");

                // Load the child page.
                pager.get_page(page_id, &mut temp_page);

                // Parse out the header.
                let child_page = Page::<Table>::from_buffer(temp_page);

                // Scan the child page.
                child_page.scan(pager);

                child_page.into_buffer()
            });
    }
}

impl Scan for Page<Table> {
    fn scan(&self, pager: &mut Pager) {
        match self {
            Page::Leaf(leaf_page) => leaf_page.scan(pager),
            Page::Interior(interior_page) => interior_page.scan(pager),
        }
    }
}

fn main() {
    let file = File::open(DATABASE).unwrap();

    let mut pager = Pager::new(file);

    {
        let mut root_page = pager.new_page_buffer();

        // Read the first page into memory.
        pager.get_page(0, &mut root_page);

        let page = Page::<Table>::from_buffer(root_page);
        dbg!(page.cell_count);

        page.scan(&mut pager);
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

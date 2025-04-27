mod memory;
mod structures;

use std::fs::File;

use structures::{
    VarInt,
    btree::cell::{PageCell, PageCtx, Table},
};

use crate::{
    memory::pager::*,
    structures::btree::{BTree, BTreeWalker},
};

const DATABASE: &str = "test.db";

fn main() {
    let file = File::open(DATABASE).unwrap();

    let pager = Pager::bootstrap(file).unwrap();
    let header = pager.get_header().unwrap();

    let btree = BTree::<Table>::new(
        pager,
        PageId::FIRST,
        header.header(|header| PageCtx::from(header)),
    );

    let walker = BTreeWalker::new(&btree);
    let cell = walker.get_cell().unwrap();

    let cell_content = cell.payload().unwrap();

    let mut header_length_buf = [0; 9];
    cell_content.copy_to_slice(0, &mut header_length_buf);

    let (header_length, _) = VarInt::from_buffer(&header_length_buf);

    let mut header_buf = vec![0; *header_length as usize];
    cell_content.copy_to_slice(0, &mut header_buf);

    let mut header = header_buf.as_slice();
    while !header.is_empty() {
        let (serial_type, rest) = VarInt::from_buffer(header);
        header = rest;

        println!(
            "{}",
            match *serial_type {
                0 => "NULL",
                1 => "i8",
                2 => "i16",
                3 => "i24",
                4 => "i32",
                5 => "i48",
                6 => "i64",
                7 => "f64",
                8 => "0",
                9 => "1",
                10 | 11 => "reserved",
                n @ 12.. if n % 2 == 0 => "BLOB",
                n @ 13.. if n % 2 == 1 => "text",
                _ => unreachable!(),
            }
        );
    }
}

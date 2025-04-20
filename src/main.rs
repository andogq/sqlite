mod memory;
mod pager;
mod structures;
mod varint;

use std::{
    fs::File,
    io::{Read, Seek},
};

use pager::{PageId, Pager, SomePager};
use structures::{
    btree::{BTree, BTreeWalker, Table},
    header::{SQLITE_HEADER_SIZE, SqliteHeader},
};

const DATABASE: &str = "test.db";

fn main() {
    let file = File::open(DATABASE).unwrap();

    let pager = bootstrap(file).unwrap();
    let btree = BTree::<Table>::new_with_pager(pager, PageId::FIRST);

    let walker = BTreeWalker::new(&btree);
    let cell = walker.get_cell().unwrap();
    dbg!(cell.get());
}

fn bootstrap<'source>(
    source: impl 'source + Seek + Read,
) -> Result<Box<dyn 'source + SomePager>, std::io::Error> {
    let mut pager = Pager::new(source, SQLITE_HEADER_SIZE, 16);

    let page = pager.get(PageId::HEADER)?.unwrap();
    let buf = &page.buffer();

    let header = SqliteHeader::read_from_buffer(buf).unwrap();
    let page_size = header.page_size();

    // Update the pager.
    pager.update_page_size(page_size as usize);

    Ok(Box::new(pager))
}

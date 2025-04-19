mod disk;
mod structures;
mod varint;

use std::{
    fs::File,
    io::{Read, Seek},
};

use disk::{PageId, Pager, SomePager};
use structures::{
    btree::{BTree, Table},
    header::{SQLITE_HEADER_SIZE, SqliteHeader},
};

const DATABASE: &str = "test.db";

fn main() {
    let file = File::open(DATABASE).unwrap();

    let pager = bootstrap(file).unwrap();
    let mut btree = BTree::<Table>::new_with_pager(pager);

    let page = btree.get_page(PageId::FIRST);

    dbg!(&page);
}

fn bootstrap<'source>(
    source: impl 'source + Seek + Read,
) -> Result<Box<dyn 'source + SomePager>, std::io::Error> {
    let mut pager = Pager::new(source, SQLITE_HEADER_SIZE, 16);

    let header = SqliteHeader::read_from_buffer(pager.get(PageId::HEADER)?.unwrap()).unwrap();
    let page_size = header.page_size();

    // Update the pager.
    pager.update_page_size(page_size as usize);

    Ok(Box::new(pager))
}

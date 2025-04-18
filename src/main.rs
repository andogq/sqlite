mod disk;
mod structures;
mod varint;

use std::{
    fs::File,
    io::{Read, Seek},
};

use disk::{PageId, Pager, SomePager};
use structures::{
    header::{SQLITE_HEADER_SIZE, SqliteHeader},
    page::AnyPage,
};

// use self::page::{Leaf, Pager, Table, storage::readable::ReadableStorage};

const DATABASE: &str = "test.db";

fn main() {
    let file = File::open(DATABASE).unwrap();

    let mut pager = bootstrap(file).unwrap();
    let buf = &pager.get(PageId(0)).unwrap().unwrap()[SQLITE_HEADER_SIZE..];
    let page = AnyPage::try_read(buf).unwrap();

    dbg!(&page);
    dbg!("remaining", buf.len());

    match page {
        AnyPage::IndexLeaf(_) => println!("index leaf"),
        AnyPage::TableLeaf(_) => println!("table leaf"),
        AnyPage::IndexInterior(_) => println!("index interior"),
        AnyPage::TableInterior(_) => println!("table interior"),
    }

    // let mut pager = Pager::new(ReadableStorage::new(file)).unwrap();
    //
    // let page = pager.get_page_header::<Table, Leaf>(0).unwrap();
    // dbg!(page);
}

fn bootstrap<'source>(
    source: impl 'source + Seek + Read,
) -> Result<Box<dyn 'source + SomePager>, std::io::Error> {
    let mut pager = Pager::new(source, SQLITE_HEADER_SIZE, 16);

    let header = SqliteHeader::read_from_buffer(pager.get(PageId(0))?.unwrap()).unwrap();
    let page_size = header.page_size();

    // Update the pager.
    pager.update_page_size(page_size as usize);

    Ok(Box::new(pager))
}

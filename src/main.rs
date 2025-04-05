mod header;
mod page;
mod varint;

use std::fs::File;

use self::page::{Leaf, Pager, Table, storage::readable::ReadableStorage};

const DATABASE: &str = "test.db";

fn main() {
    let file = File::open(DATABASE).unwrap();

    let mut pager = Pager::new(ReadableStorage::new(file)).unwrap();

    let page = pager.get_page_header::<Table, Leaf>(0).unwrap();
    dbg!(page);
}

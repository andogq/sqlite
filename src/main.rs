mod header;
mod page;

use std::fs::File;

use page::{Interior, Leaf, Table};

use self::page::{Pager, storage::readable::ReadableStorage};

const DATABASE: &str = "test.db";

fn main() {
    let file = File::open(DATABASE).unwrap();

    let mut pager = Pager::new(ReadableStorage::new(file)).unwrap();

    let page = pager.get_page_header::<Table, Leaf>(0).unwrap();
    dbg!(page);
}

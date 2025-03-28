mod header;
mod page;

use std::fs::File;

use self::{
    header::RawDbHeader,
    page::{Pager, storage::readable::ReadableStorage},
};

const DATABASE: &str = "test.db";

fn main() {
    let file = File::open(DATABASE).unwrap();

    let mut pager = Pager::new(ReadableStorage::new(file)).unwrap();

    let page = pager.get_page_header(0).unwrap();
    dbg!(page);
}

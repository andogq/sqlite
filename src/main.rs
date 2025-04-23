mod memory;
mod structures;

use std::fs::File;

use structures::btree::cell::Table;

use crate::{
    memory::pager::*,
    structures::btree::{BTree, BTreeWalker},
};

const DATABASE: &str = "test.db";

fn main() {
    let file = File::open(DATABASE).unwrap();

    let pager = Pager::bootstrap(file).unwrap();
    let header = pager.get_header().unwrap();

    let btree = BTree::<Table>::new(pager, PageId::FIRST);

    let walker = BTreeWalker::new(&btree);
    let cell = walker.get_cell().unwrap();

    let ctx = header.header(|header| header.into());
    let c = cell.get(&ctx);
}

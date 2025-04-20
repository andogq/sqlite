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
    let btree = BTree::<Table>::new(pager, PageId::FIRST);

    let walker = BTreeWalker::new(&btree);
    let cell = walker.get_cell().unwrap();
    dbg!(cell.get());
}

mod header;

use std::{fs::File, io::Read};

use header::RawDbHeader;
use zerocopy::FromBytes;

const DATABASE: &str = "test.db";

trait PageStorage {
    fn read_page(&self, page_id: usize);
}

struct Pager {
    storage: Box<dyn PageStorage>,

    header: RawDbHeader,
}

impl Pager {
    pub fn new() -> Self {
        todo!()
    }
}

fn read_header(reader: impl Read) -> RawDbHeader {
    RawDbHeader::read_from_io(reader).unwrap()
}

fn main() {
    println!("Hello, world!");

    let mut file = File::open(DATABASE).unwrap();
    let header = read_header(&mut file);

    dbg!(header);
}

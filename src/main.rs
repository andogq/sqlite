mod header;
mod page;

use std::{fs::File, io::Read};

use header::RawDbHeader;
use zerocopy::FromBytes;

const DATABASE: &str = "test.db";

fn read_header(reader: impl Read) -> RawDbHeader {
    RawDbHeader::read_from_io(reader).unwrap()
}

fn main() {
    println!("Hello, world!");

    let mut file = File::open(DATABASE).unwrap();
    let header = read_header(&mut file);

    dbg!(header);
}

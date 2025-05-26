use std::io::SeekFrom;

use pager::{Pager, Source};

use crate::disk::header::SqliteHeader;

pub mod pager;

#[derive(Clone, Debug)]
pub struct Ctx {
    pub header: SqliteHeader,
    pub pager: Pager,
}

impl Ctx {
    pub fn new(mut source: impl Source) -> Self {
        // Read the header from the source.
        let header = {
            let mut header_buf = [0; 100];
            source.seek(SeekFrom::Start(0)).unwrap();
            source.read_exact(&mut header_buf).unwrap();
            SqliteHeader::read_from_buffer(&header_buf).unwrap()
        };

        Self {
            pager: Pager::new(source, header.page_size() as usize),
            header: header.clone(),
        }
    }
}

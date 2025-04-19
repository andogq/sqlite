use std::{
    collections::{HashMap, hash_map::Entry},
    io::{Read, Seek, SeekFrom},
};

use crate::structures::header::SQLITE_HEADER_SIZE;

pub type Page = Box<[u8]>;

/// A page ID which maps into the file. Uses a provided page size to produce offsets, taking into
/// consideration additional space required for the header.
///
/// SQLite pages start from 1, however page 0 here is used for a smaller page which represents the
/// SQLite header.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PageId(usize);

impl PageId {
    /// Page ID which contains the SQLite header.
    ///
    /// This doesn't exist in 'standard' SQLite, so the otherwise unused page ID of 0 is selected
    /// for this purpose.
    pub const HEADER: Self = PageId(0);

    /// The first page which contains a [`BTree`].
    pub const FIRST: Self = PageId(1);

    /// Create a new page id with the given id.
    pub const fn new(page_id: usize) -> Self {
        Self(page_id)
    }

    /// Get the binary offset for the given page for a given page size, inclusive of the header
    /// bytes which are present in the first page.
    pub const fn get_offset(&self, page_size: usize) -> usize {
        if self.0 == Self::FIRST.0 {
            return SQLITE_HEADER_SIZE;
        }

        self.0 * page_size
    }
}

pub struct Pager<Source> {
    source: Source,
    page_size: usize,
    page_cache: HashMap<PageId, Page>,
}

impl<Source: Seek + Read> Pager<Source> {
    pub fn new(source: Source, page_size: usize, page_cache_size: usize) -> Self {
        Self {
            source,
            page_size,
            page_cache: HashMap::with_capacity(page_cache_size),
        }
    }

    /// Update the page size. This will empty the page cache.
    pub fn update_page_size(&mut self, page_size: usize) {
        self.page_size = page_size;
        self.page_cache.clear();
    }
}

impl<Source: Seek + Read> SomePager for Pager<Source> {
    /// Read a page from the source.
    fn get(&mut self, page_id: PageId) -> Result<Option<&[u8]>, std::io::Error> {
        Ok(Some(match self.page_cache.entry(page_id) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let page_offset = page_id.get_offset(self.page_size);
                self.source
                    .seek(SeekFrom::Start(page_offset as u64))
                    .unwrap();

                let mut buf = vec![0x00; self.page_size];
                match self.source.read_exact(&mut buf) {
                    Ok(()) => {}
                    Err(e) => {
                        return match e.kind() {
                            std::io::ErrorKind::UnexpectedEof => Ok(None),
                            _ => Err(e),
                        };
                    }
                };

                entry.insert(buf.into_boxed_slice())
            }
        }))
    }
}

pub trait SomePager {
    fn get(&mut self, page_id: PageId) -> Result<Option<&[u8]>, std::io::Error>;
}

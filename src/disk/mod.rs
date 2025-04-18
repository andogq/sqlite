use std::{
    collections::{HashMap, hash_map::Entry},
    io::{Read, Seek, SeekFrom},
};

pub type Page = Box<[u8]>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PageId(pub usize);

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
                let page_offset = page_id.0 * self.page_size;
                self.source.seek(SeekFrom::Start(page_offset as u64));

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

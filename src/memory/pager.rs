use std::{
    cell::RefCell,
    collections::{HashMap, hash_map::Entry},
    io::{Read, Seek, SeekFrom},
};

use crate::{
    memory::*,
    structures::header::{SQLITE_HEADER_SIZE, SqliteHeader},
};

/// Produce fixed sized [`MemoryPage`]s from a [`Source`].
pub struct Pager {
    /// Underlying source where memory will be read from.
    source: RefCell<Box<dyn Source>>,
    /// Page size to use when allocating and reading pages.
    page_size: usize,
    /// Page cache to store existing allocations.
    page_cache: RefCell<HashMap<PageId, MemoryPage>>,
}

impl Pager {
    /// Create a new pager with the provided [`Source`], using the specified page size.
    pub fn new(source: impl Source, page_size: usize, page_cache_size: usize) -> Self {
        Self {
            source: RefCell::new(Box::new(source)),
            page_size,
            page_cache: RefCell::new(HashMap::with_capacity(page_cache_size)),
        }
    }

    /// Update the page size. This will empty the page cache.
    pub fn update_page_size(&mut self, page_size: usize) {
        self.page_size = page_size;
        self.page_cache.get_mut().clear();
    }

    /// Read a page from the source.
    pub fn get(&self, page_id: PageId) -> Result<Option<MemoryPage>, std::io::Error> {
        Ok(Some(match self.page_cache.borrow_mut().entry(page_id) {
            Entry::Occupied(entry) => entry.into_mut().clone(),
            Entry::Vacant(entry) => {
                let page_offset = page_id.get_offset(self.page_size);

                let mut source = self.source.borrow_mut();
                source.seek(SeekFrom::Start(page_offset as u64)).unwrap();

                let mut buf = MemoryBuffer::allocate(self.page_size);
                match source.read_exact(&mut buf) {
                    Ok(()) => {}
                    Err(e) => {
                        return match e.kind() {
                            std::io::ErrorKind::UnexpectedEof => Ok(None),
                            _ => Err(e),
                        };
                    }
                };

                entry.insert(MemoryPage::new_with_buffer(buf)).clone()
            }
        }))
    }

    /// Create an instance from the provided source, by attempting to read the [`SqliteHeader`]
    /// from the beginning of the soruce and using it for configuration.
    pub fn bootstrap(mut source: impl Source) -> Result<Self, std::io::Error> {
        // Ensure source always begins from the start.
        source.rewind()?;

        let mut pager = Self::new(source, SQLITE_HEADER_SIZE, 16);

        // Read the header.
        let page = pager.get(PageId::HEADER)?.unwrap();
        let buf = &page.buffer();
        let header = SqliteHeader::read_from_buffer(buf).unwrap();

        // Update the configuration.
        pager.update_page_size(header.page_size() as usize);

        Ok(pager)
    }
}

/// Any source that is compatible with [`Pager`]
pub trait Source: 'static + Seek + Read {}
impl<S: 'static + Seek + Read> Source for S {}

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

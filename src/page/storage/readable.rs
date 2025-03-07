use std::{
    collections::{BTreeMap, btree_map::Entry},
    io::{Read, Seek, SeekFrom},
};

use super::{PageStorage, StorageError};

/// Helper trait for anything that implements [`Read`], and [`Seek`].
pub trait Readable: Read + Seek {}
impl<T> Readable for T where T: Read + Seek {}

/// Page storage for anything that implements [`Readable`].
pub struct ReadableStorage {
    /// Underlying [`Readable`] implementation.
    buf: Box<dyn Readable>,

    /// Configured page size.
    page_size: Option<usize>,

    /// Currently loaded pages.
    pages: BTreeMap<usize, Vec<u8>>,
}

impl ReadableStorage {
    /// Create a storage instance from something implementing [`Readable`].
    pub fn new(buf: impl 'static + Readable) -> Self {
        Self {
            buf: Box::new(buf),
            page_size: None,
            pages: BTreeMap::new(),
        }
    }

    fn get_page_size(&self) -> Result<usize, StorageError> {
        self.page_size.ok_or(StorageError::PageSizeNotConfigured)
    }
}

impl PageStorage for ReadableStorage {
    fn set_page_size(&mut self, page_size: usize) {
        self.page_size = Some(page_size);
    }

    fn read_start(&mut self, n: usize) -> Result<Vec<u8>, StorageError> {
        // Rewind the buffer.
        self.buf.rewind()?;

        // Create the buffer and read into it.
        let mut buf = vec![0; n];
        self.buf.read_exact(&mut buf)?;

        Ok(buf)
    }

    fn read_page(&mut self, page_id: usize) -> Result<&[u8], StorageError> {
        let page_size = self.get_page_size()?;

        if let Entry::Vacant(page_entry) = self.pages.entry(page_id) {
            // Seek to the page.
            self.buf
                .seek(SeekFrom::Start((page_id * page_size) as u64))?;

            // Read into a buffer.
            let mut buf = vec![0; page_size];
            self.buf.read_exact(&mut buf)?;

            // Save the buffer.
            page_entry.insert(buf);
        }

        // Read the page from the cache since it's guarenteed to exist.
        Ok(self
            .pages
            .get(&page_id)
            .expect("previously inserted page exists")
            .as_slice())
    }
}

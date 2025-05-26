use std::{
    cell::RefCell,
    io::{Read, Seek, SeekFrom},
    ops::Deref,
    rc::Rc,
};
use zerocopy::big_endian::*;

#[derive(Clone)]
pub struct Pager {
    source: Rc<RefCell<Box<dyn Source>>>,
    page_size: u16,
}

impl Pager {
    /// Create a new pager with the provided source. This will configure the pager to use the
    /// correct page size based on the header.
    pub fn new(source: impl Source) -> Self {
        let mut pager = Self {
            source: Rc::new(RefCell::new(Box::new(source))),
            page_size: 0,
        };

        pager.configure_from_source(super::disk::header::PAGE_SIZE_OFFSET);

        pager
    }

    /// Configure this pager using the page size located at `size_offset` in the source.
    fn configure_from_source(&mut self, size_offset: usize) {
        let mut source = self.source.borrow_mut();

        // Position the source in the correct location.
        source.seek(SeekFrom::Start(size_offset as u64)).unwrap();

        // Read two bytes into a buffer.
        let mut buf = [0; 2];
        source.read_exact(&mut buf).unwrap();

        // Deserialise as a u16, and set it as the page size.
        self.page_size = U16::from_bytes(buf).get();
    }

    /// Read the requested page, and write it to `buf`. It is expected that `buf` is large enough
    /// to hold the entire page, so it should be created with [`Self::new_page_buffer`].
    pub fn get_page(&self, page_id: u32, buf: &mut PageBuffer) {
        assert_eq!(
            buf.len(),
            self.page_size as usize,
            "buffer must match page size"
        );

        let mut source = self.source.borrow_mut();

        // Seek to the correct position.
        let offset = (self.page_size as u32 * page_id) as u64;
        source.seek(SeekFrom::Start(offset)).unwrap();

        // Fill the buffer.
        source.read_exact(&mut buf.buffer).unwrap();

        // Fix the buffer's size, if the offset means a full page won't be read (page 0).
        buf.offset = if page_id == 0 {
            super::disk::header::SQLITE_HEADER_SIZE
        } else {
            0
        };
    }

    /// Create a new buffer suitable for holding a page.
    pub fn new_page_buffer(&self) -> PageBuffer {
        PageBuffer::new(self.page_size as usize)
    }
}

pub trait Source: 'static + Read + Seek {}
impl<T> Source for T where T: 'static + Read + Seek {}

#[derive(Clone, Debug)]
pub struct PageBuffer {
    /// Additional offset to apply to every slice.
    offset: usize,

    /// Underlying data.
    buffer: Vec<u8>,
}

impl PageBuffer {
    fn new(size: usize) -> Self {
        Self {
            offset: 0,
            buffer: vec![0; size],
        }
    }

    /// Produce the full buffer, even if it has an offset applied to it.
    ///
    /// This is useful for processing offsets stored directly within the binary.
    pub fn raw(&self) -> &[u8] {
        &self.buffer
    }

    /// Update the provided pointer into this buffer, to take into account the offset already
    /// applied to this buffer.
    pub fn adjust_with_offset(&self, ptr: usize) -> usize {
        assert!(
            ptr >= self.offset,
            "original pointer must include buffer offset"
        );

        ptr - self.offset
    }
}

impl Deref for PageBuffer {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.buffer[self.offset..]
    }
}

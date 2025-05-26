use derive_more::{Deref, DerefMut};
use std::{
    cell::RefCell,
    collections::HashMap,
    io::{Read, Seek, SeekFrom},
    ops::Deref,
    rc::Rc,
};
use zerocopy::big_endian::*;

#[derive(Clone)]
pub struct Pager(Rc<PagerInner>);

struct PagerInner {
    /// Underlying source for this pager.
    source: RefCell<Box<dyn Source>>,

    /// Configured page size.
    page_size: u16,

    /// Loaded pages.
    pages: RefCell<HashMap<u32, PageBuffer>>,
}

impl Pager {
    /// Create a new pager with the provided source. This will configure the pager to use the
    /// correct page size based on the header.
    pub fn new(source: impl Source) -> Self {
        let mut pager = PagerInner {
            source: RefCell::new(Box::new(source)),
            page_size: 0,
            pages: RefCell::new(HashMap::new()),
        };
        pager.configure_from_source(super::disk::header::PAGE_SIZE_OFFSET);

        Self(Rc::new(pager))
    }

    /// Read the requested page, and write it to `buf`. It is expected that `buf` is large enough
    /// to hold the entire page, so it should be created with [`Self::new_page_buffer`].
    pub fn get_page(&self, page_id: u32) -> PageBuffer {
        self.0
            .pages
            .borrow_mut()
            .entry(page_id)
            .or_insert_with(|| {
                let mut buf = self.0.new_page_buffer();

                // Borrow the source to use it.
                let mut source = self.0.source.borrow_mut();

                // Seek to the correct position.
                let offset = (self.0.page_size as u32 * page_id) as u64;
                source.seek(SeekFrom::Start(offset)).unwrap();

                {
                    // Temporarily mutate the buffer whilst there's no other references.
                    let buf = Rc::get_mut(&mut buf.0).unwrap();

                    // Fill the buffer.
                    source.read_exact(&mut buf.buffer).unwrap();

                    // Fix the buffer's size, if the offset means a full page won't be read (page 0).
                    buf.offset = if page_id == 0 {
                        super::disk::header::SQLITE_HEADER_SIZE
                    } else {
                        0
                    };
                }

                buf
            })
            .clone()
    }
}

impl PagerInner {
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

    /// Create a new buffer suitable for holding a page.
    fn new_page_buffer(&self) -> PageBuffer {
        PageBuffer::new(self.page_size as usize)
    }
}

pub trait Source: 'static + Read + Seek {}
impl<T> Source for T where T: 'static + Read + Seek {}

#[derive(Clone, Debug, Deref, DerefMut)]
pub struct PageBuffer(Rc<PageBufferInner>);

#[derive(Debug)]
pub struct PageBufferInner {
    /// Additional offset to apply to every slice.
    offset: usize,

    /// Underlying data.
    buffer: Vec<u8>,
}

impl PageBuffer {
    fn new(size: usize) -> Self {
        Self(Rc::new(PageBufferInner {
            offset: 0,
            buffer: vec![0; size],
        }))
    }
}

impl PageBufferInner {
    /// Produce the full buffer, even if it has an offset applied to it.
    ///
    /// This is useful for processing offsets stored directly within the binary.
    pub fn raw(&self) -> &[u8] {
        &self.buffer
    }
}

impl Deref for PageBufferInner {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.buffer[self.offset..]
    }
}

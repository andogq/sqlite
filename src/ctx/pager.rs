use derive_more::{Deref, DerefMut};
use std::{
    cell::RefCell,
    collections::HashMap,
    fmt::Debug,
    io::{Read, Seek, SeekFrom},
    ops::Deref,
    rc::Rc,
};

#[derive(Clone, Debug)]
pub struct Pager(Rc<PagerInner>);

#[derive(Debug)]
struct PagerInner {
    /// Underlying source for this pager.
    source: RefCell<Box<dyn Source>>,

    /// Configured page size.
    page_size: usize,

    /// Loaded pages.
    pages: RefCell<HashMap<u32, PageBuffer>>,
}

impl Pager {
    /// Create a new pager with the provided source. This will configure the pager to use the
    /// correct page size based on the header.
    pub fn new(source: impl Source, page_size: usize) -> Self {
        Self(Rc::new(PagerInner {
            source: RefCell::new(Box::new(source)),
            page_size,
            pages: RefCell::new(HashMap::new()),
        }))
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
                let offset = (self.0.page_size as u32 * (page_id - 1)) as u64;
                source.seek(SeekFrom::Start(offset)).unwrap();

                {
                    // Temporarily mutate the buffer whilst there's no other references.
                    let buf = Rc::get_mut(&mut buf.0).unwrap();

                    // Fill the buffer.
                    source.read_exact(&mut buf.buffer).unwrap();

                    // Fix the buffer's size, if the offset means a full page won't be read (page 0).
                    buf.offset = if page_id == 1 {
                        crate::disk::header::SQLITE_HEADER_SIZE
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
    /// Create a new buffer suitable for holding a page.
    fn new_page_buffer(&self) -> PageBuffer {
        PageBuffer::new(self.page_size)
    }
}

pub trait Source: 'static + Read + Seek + Debug {}
impl<T> Source for T where T: 'static + Read + Seek + Debug {}

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

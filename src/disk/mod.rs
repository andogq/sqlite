use std::{
    cell::RefCell,
    collections::{HashMap, hash_map::Entry},
    io::{Read, Seek, SeekFrom},
    ops::{Bound, Index, Range, RangeBounds},
    rc::Rc,
};

use derive_more::{Deref, DerefMut};

use crate::structures::header::SQLITE_HEADER_SIZE;

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
    source: RefCell<Source>,
    page_size: usize,
    page_cache: RefCell<HashMap<PageId, Page>>,
}

impl<Source: Seek + Read> Pager<Source> {
    pub fn new(source: Source, page_size: usize, page_cache_size: usize) -> Self {
        Self {
            source: RefCell::new(source),
            page_size,
            page_cache: RefCell::new(HashMap::with_capacity(page_cache_size)),
        }
    }

    /// Update the page size. This will empty the page cache.
    pub fn update_page_size(&mut self, page_size: usize) {
        self.page_size = page_size;
        self.page_cache.get_mut().clear();
    }
}

impl<Source: Seek + Read> SomePager for Pager<Source> {
    /// Read a page from the source.
    fn get(&self, page_id: PageId) -> Result<Option<Page>, std::io::Error> {
        Ok(Some(match self.page_cache.borrow_mut().entry(page_id) {
            Entry::Occupied(entry) => entry.into_mut().clone(),
            Entry::Vacant(entry) => {
                let page_offset = page_id.get_offset(self.page_size);

                let mut source = self.source.borrow_mut();
                source.seek(SeekFrom::Start(page_offset as u64)).unwrap();

                let mut buf = Buffer::allocate(self.page_size);
                match source.read_exact(&mut buf) {
                    Ok(()) => {}
                    Err(e) => {
                        return match e.kind() {
                            std::io::ErrorKind::UnexpectedEof => Ok(None),
                            _ => Err(e),
                        };
                    }
                };

                entry.insert(Page::new(buf)).clone()
            }
        }))
    }
}

pub trait SomePager {
    fn get(&self, page_id: PageId) -> Result<Option<Page>, std::io::Error>;
}

use std::{cell::Ref, ops::Deref};

/// Buffer containing arbitrary data.
#[derive(Deref, DerefMut)]
pub struct Buffer(Box<[u8]>);

impl Buffer {
    pub fn allocate(size: usize) -> Self {
        Self(vec![0x00; size].into_boxed_slice())
    }
}

/// A page held in memory. Internally holds a shared reference to the buffer.
#[derive(Clone)]
pub struct Page {
    buffer: Rc<RefCell<Buffer>>,
}

impl Page {
    pub fn new(buffer: Buffer) -> Self {
        Self {
            buffer: Rc::new(RefCell::new(buffer)),
        }
    }

    pub fn buffer(&self) -> PageBuffer<'_> {
        PageBuffer(self.buffer.borrow())
    }

    pub fn slice<R: RangeBounds<usize>>(&self, bounds: R) -> PageSlice {
        PageSlice {
            page: self.clone(),
            bounds: (bounds.start_bound().cloned(), bounds.end_bound().cloned()),
        }
    }
}

/// A reference to the buffer of a page.
pub struct PageBuffer<'p>(Ref<'p, Buffer>);
impl Deref for PageBuffer<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0.0
    }
}

#[derive(Clone)]
pub struct PageSlice {
    page: Page,
    bounds: (Bound<usize>, Bound<usize>),
}

impl PageSlice {
    pub fn buffer(&self) -> PageSliceBuffer<'_> {
        PageSliceBuffer {
            buffer: self.page.buffer().0,
            bounds: self.bounds,
        }
    }
}

pub struct PageSliceBuffer<'p> {
    buffer: Ref<'p, Buffer>,
    bounds: (Bound<usize>, Bound<usize>),
}
impl Deref for PageSliceBuffer<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.buffer.0[self.bounds]
    }
}

//! Memory pages are fixed allocations of memory.
//!
//! In order to abide by memory ownership rules, there are a few levels of indirection between
//! [`Page`]s read from a file, and the underlying `&[u8]` slices.
//!
//! - [`Buffer`] is the simplest primitive, and is effectively a wrapper over `Box<[u8]>`. It is
//!   intended to provide some utility methods (eg allocation), however it doesn't intrinsicly
//!   provide any special functionality.
//!
//! - [`PageRef`] internally contains a [`Ref`], acting as a borrow guard to llow borrows to be
//!   dynamically tracked at run time. The borrow guard contains the [`Buffer`], and so any use of
//!   the page's data must be done so through this structure. This 'introduces' the lifetime of the
//!   underlying data, tying it back to the source of the [`PageRef`].
//!
//! - [`Page`] acts as an owned value (no internal references or lifetimes), however it contains
//!   the underlying [`Buffer`] wrapped in [`Rc<RefCell>`]. This allows handling of the page
//!   without having to satisfy compile-time lifetime rules, however interacting with the
//!   [`Buffer`] requires useof the [`Page::buffer`] method, which returns [`PageRef`]. This
//!   method ties the lifetime of the buffer to the instance of [`Page`], ensuring that the
//!   reference count will not be dropped whilst in use, and preventing dangling references to the
//!   original bytes from hanging around.
//!
//! This explanation has been slightly simplified, as [`PageRef`] and [`Page`] have additional
//! functionality to allow for producing slices to their data within. This mostly results in them
//! containing an 'inner' enum (a `Buffer` and `Slice` variation) wrapped in an `Rc` to allow
//! slices to store their parents.

use std::{
    cell::{Ref, RefCell},
    ops::{Bound, Deref, RangeBounds},
    rc::Rc,
};

use derive_more::{Deref, DerefMut};

/// Buffer containing arbitrary data.
#[derive(Clone, Deref, DerefMut)]
pub struct MemoryBuffer(Box<[u8]>);

impl MemoryBuffer {
    /// Allocate a new buffer of the provided size.
    pub fn allocate(size: usize) -> Self {
        Self(vec![0x00; size].into_boxed_slice())
    }
}

/// A page of memory, which is tracked at runtime.
#[derive(Clone)]
pub struct MemoryPage(Rc<MemoryPageInner>);

/// Inner implementation of [`MemoryPage`]. It distinguishes between the direct view of a buffer,
/// or a slice of a buffer.
#[derive(Clone)]
enum MemoryPageInner {
    /// Page containing a full buffer.
    Buffer {
        /// Buffer containing the data of the page.
        buffer: Rc<RefCell<MemoryBuffer>>,
    },
    /// Page containing a slice of another page. Since the underlying [`MemoryBuffer`] is only
    /// borrowed for short periods of time, the requested bounds must be retained in order to
    /// reconstruct the slice as required.
    Slice {
        /// Parent which this page will slice into.
        parent: MemoryPage,
        /// Requested bounds.
        bounds: (Bound<usize>, Bound<usize>),
    },
}

impl MemoryPage {
    /// Create a new instance with the provided [`MemoryPageInner`].
    fn new(inner: MemoryPageInner) -> Self {
        Self(Rc::new(inner))
    }

    /// Create a new page with the provided buffer.
    pub fn new_with_buffer(buffer: MemoryBuffer) -> MemoryPage {
        Self::new(MemoryPageInner::Buffer {
            buffer: Rc::new(RefCell::new(buffer)),
        })
    }

    /// Access the underlying [`MemoryPageRef`] for this page.
    pub fn buffer(&self) -> MemoryPageRef<'_> {
        MemoryPageRef::new(match &*self.0 {
            MemoryPageInner::Buffer { buffer } => MemoryPageRefInner::Buffer {
                buffer: buffer.borrow(),
                page: self,
            },
            MemoryPageInner::Slice { parent, bounds } => MemoryPageRefInner::Slice {
                parent: parent.buffer(),
                bounds: *bounds,
                page: self,
            },
        })
    }

    /// Produce a slice of this page.
    pub fn slice<R: RangeBounds<usize>>(&self, bounds: R) -> Self {
        Self::new(MemoryPageInner::Slice {
            parent: self.clone(),
            bounds: (bounds.start_bound().cloned(), bounds.end_bound().cloned()),
        })
    }
}

/// A reference to the [`MemoryBuffer`] of a [`MemoryPage`].
#[derive(Clone)]
pub struct MemoryPageRef<'p>(Rc<MemoryPageRefInner<'p>>);

/// Inner implementation to [`MemoryPageRef`].
///
/// See [`MemoryPageInner`] detailed explanation.
pub enum MemoryPageRefInner<'p> {
    /// Full buffer.
    Buffer {
        /// Underlying buffer borrow.
        buffer: Ref<'p, MemoryBuffer>,
        /// Original page that this ref originates from.
        page: &'p MemoryPage,
    },
    /// Slice into another [`MemoryPageRef`].
    Slice {
        /// Parent containing original buffer.
        parent: MemoryPageRef<'p>,
        /// Requested bounds.
        bounds: (Bound<usize>, Bound<usize>),
        /// Original page that this ref originates from.
        page: &'p MemoryPage,
    },
}

impl<'p> MemoryPageRef<'p> {
    /// Create a new instance with the provided inner.
    fn new(inner: MemoryPageRefInner<'p>) -> Self {
        Self(Rc::new(inner))
    }

    /// Produce a copy of the original page that this ref is from.
    pub fn page(&self) -> MemoryPage {
        match *self.0 {
            MemoryPageRefInner::Buffer { page, .. } => page.clone(),
            MemoryPageRefInner::Slice { page, .. } => page.clone(),
        }
    }

    /// Produce a slice of this page.
    pub fn slice<R: RangeBounds<usize>>(&self, bounds: R) -> Self {
        Self::new(MemoryPageRefInner::Slice {
            parent: self.clone(),
            bounds: (bounds.start_bound().cloned(), bounds.end_bound().cloned()),
            page: match *self.0 {
                MemoryPageRefInner::Buffer { page, .. } => page,
                MemoryPageRefInner::Slice { page, .. } => page,
            },
        })
    }
}

impl Deref for MemoryPageRef<'_> {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        match &*self.0 {
            // Produce the inner buffer.
            MemoryPageRefInner::Buffer { buffer, .. } => &buffer.0,
            // Dereference the parent, and apply the bounds to the result.
            MemoryPageRefInner::Slice { parent, bounds, .. } => &parent[*bounds],
        }
    }
}

//! This module contains utilities for deriving data from [`MemoryPage`]s. Accessing the underlying
//! bytes of a [`MemoryPage`] requires a call to [`MemoryPage::buffer`], which will perform a
//! runtime borrow of the buffer, ensuring it's active as long as the returned [`MemoryPageRef`] is
//! in scope.
//!
//! This requires two components in order to derive data from the borrowed buffer:
//!
//! 1. Something to produce the [`MemoryPageRef`], and hold it as long as required.
//!
//! 2. Something to process the borrowed [`MemoryPageRef`] (and the underlying bytes), which can
//!    produce data referencing the borrowed data (to limit the required copies).
//!
//! Anything implementing [`Process`] fulfills `1`, as the trait requires it to produce a
//! [`MemoryPageRef`] From [`Process::get_page_ref`]. The automatically implemented
//! [`Process::process`] method utilises this reference, and passes a copy of it to `2`.
//!
//! Anything that implements [`FromMemoryPageRef`] fulfills `2`. Implementors of this trait are
//! able to freely use the provided [`MemoryPageRef`] to extract byte slices, and hold references
//! to them. It is up to anything that creates an instance of [`FromMemoryPageRef`] to manage the
//! lifetimes, not the instance itself.
//!
//! These two components are combined in [`Process::process`], where the borrowed [`MemoryPageRef`]
//! is provided to [`FromMemoryPageRef::from_ref`] so it can create a new instance of itself. This
//! new instance cannot be created without violating the borrow requirements (as [`MemoryPageRef`]
//! will be dropped at the end of the function), so the instance is instead passed to a closure.
//! The closure is able to extract information as requierd, and must return owned values for later
//! usage.

use super::page::*;

/// Helper trait to convert some state (`self`) and a [`MemoryPage`] into some useful data.
///
/// This solves lifetime related problems by borrowing the [`MemoryPage`] within a closure, only
/// providing the reference for a limited time. This allows the user to return any derived data
/// from the borrow, cloning required data as necessary to ensure that no references are held for
/// too long.
pub trait Process: Sized {
    /// The kind of data that will be provided during processing.
    ///
    /// `'r` is the lifetime of the page reference.
    type Data<'r>: FromMemoryPageRef<'r, &'r Self>
    where
        Self: 'r;

    /// Fetch a reference to the memory page.
    ///
    /// This is used to automatically implement [`Process::process`].
    fn get_page_ref(&self) -> MemoryPageRef<'_>;

    /// Call the provided callback with the data derived from the page. The result of the callback
    /// will be returned, however it must outlive the reference to the data provided to it.
    fn process<U>(&self, f: impl FnOnce(Self::Data<'_>) -> U) -> U {
        let buffer = self.get_page_ref();
        let data = Self::Data::from_ref(self, &buffer);
        f(data)
    }
}

/// Utility trait to provide an instance using the provided state and [`MemoryPageRef`].
///
/// `'r` is the lifetime of the page reference.
pub trait FromMemoryPageRef<'r, S> {
    /// Use the proivided state and [`MemoryPageRef`] to produce a new instance.
    ///
    /// `&MemoryPageRef` is required (as opposed to `MemoryPageRef` without the reference) as it
    /// holds the active borrow to the underlying data. Passing an owned version would cause it to
    /// be dropped at the end of the function, invalidating the borrow. By passing it in as a
    /// reference, the function caller is required to keep it alive as long as it uses the result
    /// of this method.
    ///
    /// `'c` is the lifetime of the reference to [`MemoryPageRef`] provided by the caller. The
    /// caller must ensure that `'c` is valid whilst there is data referencing `'r`.
    fn from_ref<'c: 'r>(state: S, page_ref: &'c MemoryPageRef<'r>) -> Self;
}

impl<'r> FromMemoryPageRef<'r, ()> for &'r [u8] {
    fn from_ref<'c: 'r>(_state: (), page_ref: &'c MemoryPageRef<'r>) -> Self {
        page_ref
    }
}

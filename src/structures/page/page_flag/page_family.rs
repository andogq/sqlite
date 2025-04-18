use std::fmt::Debug;

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned, big_endian::U32};

use crate::structures::page::cell::InteriorCellData;

/// Marker trait for different 'families' of pages. The family indicates the relation to other
/// [`Page`]s in the B-Tree, such as [`Leaf`] if it has no descendants, or [`Interior`] if it
/// does.
pub trait PageFamily {
    /// Data required for [`Page`]s of this family.
    type PageData: Debug;

    /// Data contained in [`Cell`]s originating from [`Page`]s of this family.
    type CellData: Debug;
}

/// A leaf [`Page`] has no pointers to other pages, however it's [`Cell`]s hold keys and/or content
/// for [`Table`]s and [`Index`]es.
#[derive(Debug, Clone, Copy)]
pub enum Leaf {}
impl PageFamily for Leaf {
    type PageData = ();
    type CellData = ();
}

/// An interior [`Page`] contains keys, and pointers to child [`Page`]s.
#[derive(Debug, Clone, Copy)]
pub struct Interior {}
impl PageFamily for Interior {
    type PageData = InteriorPageData;
    type CellData = InteriorCellData;
}

#[derive(
    Clone, Copy, Debug, IntoBytes, FromBytes, Immutable, Unaligned, PartialEq, Eq, KnownLayout,
)]
#[repr(C)]
pub struct InteriorPageData {
    right_pointer: U32,
}

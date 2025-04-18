use std::fmt::Debug;

use crate::structures::page::cell::TableCellData;

/// Marker trait for different 'types' of pages. Type corresponds to the purpose of the page,
/// such as [`Table`] or [`Index`].
pub trait PageType {
    /// Data required for [`Page`]s of this type.
    type PageData: Debug;

    /// Data contained in [`Cell`]s originating from [`Page`]s of this type.
    type CellData: Debug;
}

/// Each entry in a table has a 64 bit key, and arbitrary data.
#[derive(Debug, Clone, Copy)]
pub enum Table {}
impl PageType for Table {
    type PageData = ();
    type CellData = TableCellData;
}

/// Each entry contains an arbitrarily long key.
#[derive(Debug, Clone, Copy)]
pub enum Index {}
impl PageType for Index {
    type PageData = ();
    type CellData = ();
}

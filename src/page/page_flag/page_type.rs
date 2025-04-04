use cuisiner::Cuisiner;

use crate::page::cell::TableCellData;

/// Marker trait for different 'types' of pages. Type corresponds to the purpose of the page,
/// such as [`Table`] or [`Index`].
pub trait PageType {
    /// Data required for [`Page`]s of this type.
    type PageData: Cuisiner;

    /// Data contained in [`Cell`]s originating from [`Page`]s of this type.
    type CellData: Cuisiner;
}

/// Each entry in a table has a 64 bit key, and arbitrary data.
#[derive(Debug)]
pub enum Table {}
impl PageType for Table {
    type PageData = ();
    type CellData = TableCellData;
}

/// Each entry contains an arbitrarily long key.
#[derive(Debug)]
pub enum Index {}
impl PageType for Index {
    type PageData = ();
    type CellData = ();
}

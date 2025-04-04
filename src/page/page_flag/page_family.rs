use cuisiner::Cuisiner;

use crate::page::cell::InteriorCellData;

/// Marker trait for different 'families' of pages. The family indicates the relation to other
/// [`Page`]s in the B-Tree, such as [`Leaf`] if it has no descendants, or [`Interior`] if it
/// does.
pub trait PageFamily {
    /// Data required for [`Page`]s of this family.
    type PageData: Cuisiner;

    /// Data contained in [`Cell`]s originating from [`Page`]s of this family.
    type CellData: Cuisiner;
}

/// A leaf [`Page`] has no pointers to other pages, however it's [`Cell`]s hold keys and/or content
/// for [`Table`]s and [`Index`]es.
#[derive(Debug)]
pub enum Leaf {}
impl PageFamily for Leaf {
    type PageData = ();
    type CellData = ();
}

/// An interior [`Page`] contains keys, and pointers to child [`Page`]s.
#[derive(Debug)]
pub struct Interior {}
impl PageFamily for Interior {
    type PageData = InteriorPageData;
    type CellData = InteriorCellData;
}

#[derive(Cuisiner, Debug)]
pub struct InteriorPageData {
    right_pointer: u32,
}

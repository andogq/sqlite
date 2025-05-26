//! The type of a page corresponds to the function of the page, either a table page or an index
//! page.

mod index;
mod table;

pub use self::{index::Index, table::Table};

/// Marker trait for page types.
pub trait PageType: 'static + Clone {
    const FLAG: u8;

    fn is_table() -> bool {
        false
    }

    fn is_index() -> bool {
        false
    }
}

#[derive(Clone, Debug)]
pub enum PageTypeFlag {
    Table,
    Index,
}

impl PageTypeFlag {
    const MASK: u8 = 0b0111;

    pub const fn new(flag: u8) -> Option<Self> {
        match flag & Self::MASK {
            0b101 => Some(Self::Table),
            0b010 => Some(Self::Index),
            _ => None,
        }
    }

    pub fn is<T: PageType>(&self) -> bool {
        match self {
            PageTypeFlag::Table if T::is_table() => true,
            PageTypeFlag::Index if T::is_index() => true,
            _ => false,
        }
    }
}

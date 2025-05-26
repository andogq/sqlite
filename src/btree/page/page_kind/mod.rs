mod interior;
mod leaf;

pub use self::{interior::InteriorPage, leaf::LeafPage};

#[derive(Clone, Debug)]
pub enum PageKindFlag {
    Leaf,
    Interior,
}

impl PageKindFlag {
    const MASK: u8 = 0b1000;

    pub const fn new(flag: u8) -> Option<Self> {
        match flag & Self::MASK {
            0b1000 => Some(Self::Leaf),
            0b0000 => Some(Self::Interior),
            _ => None,
        }
    }
}

mod page_family;
mod page_type;

use std::fmt::Debug;

pub use self::{page_family::*, page_type::*};
use crate::structures::util::ConstU8;

/// Trait to contain the page flag value for a given [`PageType`] and [`PageFamily`]
/// combination.
pub trait PageFlag {
    type Value: Clone + Copy + Debug + PartialEq + Eq;
}

impl PageFlag for (Index, Interior) {
    type Value = ConstU8<0x02>;
}
impl PageFlag for (Index, Leaf) {
    type Value = ConstU8<0x05>;
}
impl PageFlag for (Table, Interior) {
    type Value = ConstU8<0x0a>;
}
impl PageFlag for (Table, Leaf) {
    type Value = ConstU8<0x0d>;
}

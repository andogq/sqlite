use super::PageType;

#[derive(Clone, Debug)]
pub enum Index {}

impl PageType for Index {
    const FLAG: u8 = 0x02;

    fn is_index() -> bool {
        true
    }
}

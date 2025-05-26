use super::PageType;

#[derive(Clone, Debug)]
pub enum Table {}

impl PageType for Table {
    const FLAG: u8 = 0x05;

    fn is_table() -> bool {
        true
    }
}

pub mod index;
pub mod table;

use crate::structures::VarInt;

use super::PageType;

pub use self::{
    index::{Index, IndexCell},
    table::{Table, TableCell},
};

pub trait PageCell<'p>: Sized {
    fn from_buffer(buf: &'p [u8], page_type: PageType) -> (Self, &'p [u8]);
    fn get_debug(&self) -> usize;
}

pub struct Payload<'p> {
    length: VarInt,
    payload: &'p [u8],
    overflow_page: Option<usize>,
}

impl<'p> Payload<'p> {
    fn from_buf_with_length(buf: &'p [u8], length: VarInt) -> (Self, &'p [u8]) {
        // TODO: Calculate payload length

        (
            Self {
                length,
                payload: &buf[0..0],
                overflow_page: None,
            },
            buf,
        )
    }

    fn from_buf(buf: &'p [u8]) -> (Self, &'p [u8]) {
        let (length, buf) = VarInt::from_buffer(buf);

        Self::from_buf_with_length(buf, length)
    }
}

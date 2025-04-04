use cuisiner::Cuisiner;

use super::{Index, Interior, Leaf, PageFamily, PageType, Table};

/// A piece of data within a [`Page`]. Although there are two attributes, the cell's 'payload'
/// is dependent on the combination of the two attributes. See [`Payload`].
struct Cell<T: PageType, F: PageFamily>
where
    (T, F): Payload,
{
    type_data: T::CellData,
    family_data: F::CellData,
    payload_data: <(T, F) as Payload>::Data,
}

#[derive(Cuisiner)]
pub struct TableCellData {
    // TODO: make varint
    rowid: u32,
}

impl<F: PageFamily> Cell<Table, F>
where
    (Table, F): Payload,
{
    pub fn get_row_id(&self) -> u32 {
        self.type_data.rowid
    }
}

#[derive(Cuisiner)]
pub struct InteriorCellData {
    left_child: u32,
}

impl<T: PageType> Cell<T, Interior>
where
    (T, Interior): Payload,
{
    pub fn get_left_child(&self) -> u32 {
        self.family_data.left_child
    }
}

pub struct PayloadCellData {
    // TODO: varint
    length: u32,
    bytes: Vec<u8>,
    overflow: u32,
}

/// The type of the payload differs depending on [`PageType`] and [`PageFamily`]. This
/// trait is to be implemented for any combination of the two attributes.
pub trait Payload {
    /// The type of the data for an attribute combination.
    type Data;
}
impl Payload for (Table, Leaf) {
    type Data = PayloadCellData;
}
impl Payload for (Table, Interior) {
    type Data = ();
}
impl Payload for (Index, Leaf) {
    type Data = PayloadCellData;
}
impl Payload for (Index, Interior) {
    type Data = PayloadCellData;
}

impl<T: PageType, F: PageFamily> Cell<T, F>
where
    (T, F): Payload<Data = PayloadCellData>,
{
    pub fn get_length(&self) -> u32 {
        self.payload_data.length
    }

    pub fn get_bytes(&self) -> &[u8] {
        &self.payload_data.bytes
    }

    pub fn get_overflow(&self) -> u32 {
        self.payload_data.overflow
    }
}

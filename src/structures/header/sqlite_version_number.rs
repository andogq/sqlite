use zerocopy::{FromBytes, Immutable, IntoBytes, Unaligned, big_endian::*};

#[derive(Clone, Debug, FromBytes, IntoBytes, Immutable, Unaligned)]
#[repr(transparent)]
pub struct SqliteVersionNumber(U32);
impl SqliteVersionNumber {
    pub fn major(&self) -> u16 {
        (self.0.get() / 1_000_000) as u16
    }

    pub fn minor(&self) -> u16 {
        (self.0.get() % 1_000_000 / 1_000) as u16
    }

    pub fn patch(&self) -> u16 {
        (self.0.get() % 1_000) as u16
    }

    pub fn get(&self) -> (u16, u16, u16) {
        (self.major(), self.minor(), self.patch())
    }

    pub fn set(&mut self, major: u16, minor: u16, patch: u16) {
        self.0 = U32::new(major as u32 * 1_000_000 + minor as u32 * 1_000 + patch as u32);
    }
}

use std::error::Error;

pub mod header;
pub mod page;
pub mod util;

mod sealed {
    pub(super) trait Sealed {}
}

#[allow(private_bounds)]
pub trait ValidityMarker: sealed::Sealed {}

#[derive(Clone, Copy, Debug)]
pub enum Valid {}
impl sealed::Sealed for Valid {}
impl ValidityMarker for Valid {}

#[derive(Clone, Copy, Debug)]
pub enum Invalid {}
impl sealed::Sealed for Invalid {}
impl ValidityMarker for Invalid {}

pub trait Validate<T: ValidityMarker> {
    type Valid;
    type Error: Error;

    fn try_get(&self) -> Result<Self::Valid, Self::Error>;
}

pub trait GetValid: Validate<Valid> {
    fn get(&self) -> Self::Valid {
        self.try_get().expect("validation has been performed")
    }
}

impl<T: Validate<Valid>> GetValid for T {}

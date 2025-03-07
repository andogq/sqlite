mod storage;

use thiserror::Error;
use zerocopy::FromBytes;

use self::storage::{PageStorage, StorageError};
use crate::{
    RawDbHeader,
    header::{DbHeader, DbHeaderError, RAW_HEADER_SIZE},
};

#[derive(Debug, Error)]
enum PagerError {
    #[error(transparent)]
    Storage(#[from] StorageError),

    #[error(transparent)]
    DbHeader(#[from] DbHeaderError),
}

struct Pager {
    storage: Box<dyn PageStorage>,

    header: DbHeader,
}

impl Pager {
    pub fn new(mut storage: impl 'static + PageStorage) -> Result<Self, PagerError> {
        let header_bytes = storage.read_start(RAW_HEADER_SIZE)?;
        let header = RawDbHeader::read_from_bytes(&header_bytes)
            .expect("header_bytes correct size for RawDbHeader")
            .try_into()?;

        Ok(Self {
            storage: Box::new(storage),
            header,
        })
    }
}

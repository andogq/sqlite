pub mod readable;

use thiserror::Error;

/// Basic functionality required to read pages to and from some storage location. The implementor
/// is free to implement any caching or other optimises as required.
pub trait PageStorage {
    /// Read the start `n` bytes from the storage, and return them. There is no expectation to
    /// cache the returnd memory.
    fn read_start(&mut self, n: usize) -> Result<Vec<u8>, StorageError>;

    /// Configure storage to use the provided page size.
    fn set_page_size(&mut self, page_size: usize);

    /// Read the given page from the storage.
    fn read_page(&mut self, page_id: usize) -> Result<&[u8], StorageError>;
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("storage page size has not been configured")]
    PageSizeNotConfigured,

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

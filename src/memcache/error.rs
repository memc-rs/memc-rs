
extern crate failure;


#[derive(Debug, Failr)]
pub enum StorageError {
    #[fail(display = "Item expired")]
    ItemExpired,
    #[fail(display = "Key not found")]
    NotFound
}
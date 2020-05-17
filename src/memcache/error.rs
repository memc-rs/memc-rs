extern crate failure;

#[derive(Debug, Fail, PartialEq)]
pub enum StorageError {
    #[fail(display = "Key not found")]
    NotFound = 0x01,
    #[fail(display = "Key exists")]
    KeyExists = 0x02,
    #[fail(display = "Value too large")]
    ValueTooLarge = 0x03,
    #[fail(display = "Invalid arguments")]
    InvalidArguments = 0x04,
    #[fail(display = "Item not stored")]
    ItemNotStored = 0x05,
    #[fail(display = "Incr/Decr on non numeric value")]
    ArithOnNonNumeric = 0x06,
    #[fail(display = "Out of memory")]
    OutOfMemory = 0x82,
    #[fail(display = "Not supported")]
    NotSupported = 0x83,
    #[fail(display = "Internal error")]
    InternalError = 0x84,
    #[fail(display = "Busy")]
    Busy = 0x85,
    #[fail(display = "Temporary failure")]
    TemporaryFailure = 0x86,
}

pub type StorageResult<T> = std::result::Result<T, StorageError>;

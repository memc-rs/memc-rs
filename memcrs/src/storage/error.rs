extern crate failure;

#[derive(Debug, PartialEq)]
pub enum StorageError {
    NotFound = 0x01,
    KeyExists = 0x02,
    ValueTooLarge = 0x03,
    InvalidArguments = 0x04,
    ItemNotStored = 0x05,
    ArithOnNonNumeric = 0x06,
    UnkownCommand = 0x81,
    OutOfMemory = 0x82,
    NotSupported = 0x83,
    InternalError = 0x84,
    Busy = 0x85,
    TemporaryFailure = 0x86,
}

impl StorageError {
    pub fn to_static_string(&self) -> String {
        match self {
            StorageError::NotFound => String::from("Key not found"),
            StorageError::KeyExists => String::from("Key exists"),
            StorageError::ValueTooLarge => String::from("Value too large"),
            StorageError::InvalidArguments => String::from("Invalid arguments"),
            StorageError::ItemNotStored => String::from("Item not stored"),
            StorageError::ArithOnNonNumeric => String::from("Incr/Decr on non numeric value"),
            StorageError::UnkownCommand => String::from("Invalid command"),
            StorageError::OutOfMemory => String::from("Out of memory"),
            StorageError::NotSupported => String::from("Not supported"),
            StorageError::InternalError => String::from("Internal error"),
            StorageError::Busy => String::from("Busy"),
            StorageError::TemporaryFailure => String::from("Temporary failure"),
        }
    }
}

pub type StorageResult<T> = std::result::Result<T, StorageError>;

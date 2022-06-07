

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
    pub fn to_static_string(&self) -> &'static str {
        static NOT_FOUND: &str = "Not found";
        static KEY_EXISTS: &str = "Key exists";

        match self {
            StorageError::NotFound => NOT_FOUND,
            StorageError::KeyExists => KEY_EXISTS,
            StorageError::ValueTooLarge => "Value too big",
            StorageError::InvalidArguments => "Invalid arguments",
            StorageError::ItemNotStored => "Item not stored",
            StorageError::ArithOnNonNumeric => "Incr/Decr on non numeric value",
            StorageError::UnkownCommand => "Invalid command",
            StorageError::OutOfMemory => "Out of memory",
            StorageError::NotSupported => "Not supported",
            StorageError::InternalError => "Internal error",
            StorageError::Busy => "Busy",
            StorageError::TemporaryFailure => "Temporary failure",
        }
    }
}

pub type StorageResult<T> = std::result::Result<T, StorageError>;

#[derive(Debug, PartialEq, Eq)]
pub enum CacheError {
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

impl CacheError {
    pub fn to_static_string(&self) -> &'static str {
        static NOT_FOUND: &str = "Not found";
        static KEY_EXISTS: &str = "Key exists";

        match self {
            CacheError::NotFound => NOT_FOUND,
            CacheError::KeyExists => KEY_EXISTS,
            CacheError::ValueTooLarge => "Value too big",
            CacheError::InvalidArguments => "Invalid arguments",
            CacheError::ItemNotStored => "Item not stored",
            CacheError::ArithOnNonNumeric => "Incr/Decr on non numeric value",
            CacheError::UnkownCommand => "Invalid command",
            CacheError::OutOfMemory => "Out of memory",
            CacheError::NotSupported => "Not supported",
            CacheError::InternalError => "Internal error",
            CacheError::Busy => "Busy",
            CacheError::TemporaryFailure => "Temporary failure",
        }
    }
}

pub type Result<T> = std::result::Result<T, CacheError>;

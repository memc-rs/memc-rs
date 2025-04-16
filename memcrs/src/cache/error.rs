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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_error_to_static_string() {
        assert_eq!(CacheError::NotFound.to_static_string(), "Not found");
        assert_eq!(CacheError::KeyExists.to_static_string(), "Key exists");
        assert_eq!(
            CacheError::ValueTooLarge.to_static_string(),
            "Value too big"
        );
        assert_eq!(
            CacheError::InvalidArguments.to_static_string(),
            "Invalid arguments"
        );
        assert_eq!(
            CacheError::ItemNotStored.to_static_string(),
            "Item not stored"
        );
        assert_eq!(
            CacheError::ArithOnNonNumeric.to_static_string(),
            "Incr/Decr on non numeric value"
        );
        assert_eq!(
            CacheError::UnkownCommand.to_static_string(),
            "Invalid command"
        );
        assert_eq!(CacheError::OutOfMemory.to_static_string(), "Out of memory");
        assert_eq!(CacheError::NotSupported.to_static_string(), "Not supported");
        assert_eq!(
            CacheError::InternalError.to_static_string(),
            "Internal error"
        );
        assert_eq!(CacheError::Busy.to_static_string(), "Busy");
        assert_eq!(
            CacheError::TemporaryFailure.to_static_string(),
            "Temporary failure"
        );
    }
}

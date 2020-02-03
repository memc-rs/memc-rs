

pub enum Magic {
    Request = 0x80,
    Response = 0x81
}

pub enum ResponseStatus {
    Success = 0x00,
    KeyNotExists = 0x01,
    KeyExists = 0x02,
    TooBig = 0x03,
    InvalidArguments = 0x04,
    NotStored = 0x05,
    NonNumericValue = 0x06,
    AuthenticationError = 0x20,
    AuthenticationContinue = 0x21,
    UnkownError = 0x81,
    NotEnoughMemory = 0x82
}

pub enum DataTypes {
    RawBytes = 0x00,
}

pub enum Command {
    Get = 0x00,
    Set = 0x01,
    Add = 0x02,
    Replace = 0x03,
    Delete = 0x04,
    Increment = 0x05,
    Decrement = 0x06,
    Quit = 0x07,
    Flush = 0x08,
    GetQuiet = 0x09,
    Noop = 0x0a,
    Version = 0x0b,
    GetKey = 0x0c,
    GetKeyQuiet = 0x0d,
    Append = 0x0e,
    Prepend = 0x0f,
    Stat = 0x10,
    SetQuiet = 0x11,
    AddQuiet = 0x12,
    ReplaceQuiet = 0x13,
    DeleteQuiet = 0x14,
    IncrementQuiet = 0x15,
    DecrementQuiet = 0x16,
    QuitQuiet = 0x17,
    FlushQuiet = 0x18,
    AppendQuiet = 0x19,
    PrependQuiet = 0x1a,
    Touch = 0x1c,
    GetAndTouch = 0x1d,
    GetAndTouchQuiet = 0x1e,
    GetAndTouchKey = 0x23,
    GetAndTouchKeyQuiet = 0x24,

    SaslListMechs = 0x20,
    SaslAuth = 0x21,
    SaslStep = 0x22,
}


[#derive(Debug, Default)]
pub struct RequestHeader {
    magic: u8,
    opcode: u8,
    keyLength: u16,
    extrasLength: u8,
    dataType: u8,
    reserved: u16,
    bodyLength: u32,
    opaque: u32,
    cas: u64
}

[#derive(Debug, Default)]
pub struct ResponseHeader {
    magic: u8,
    opcode: u8,
    keyLength: u16,
    extrasLength: u8,
    dataType: u8,
    status: u16,
    bodyLength: u32,
    opaque: u32,
    cas: u64
}

[#derive(Debug, Default)]
pub struct FlushRequestBody {
    expiration: u32
}

[#derive(Debug, Default)]
pub struct SetRequest {
    flags: u32,
    expiration: u32
}
type AddRequest = SetRequest;
type ReplaceRequest = SetRequest;

[#derive(Debug, Default)]
pub struct IncrementRequest {
    delta: u64,
    initial: u64,
    expiration: u32
}

type DecrementRequest = IncrementRequest;

[#derive(Debug, Default)]
pub struct IncrementResponse {
    value: u64;
}

type DecrementResponse = IncrementResponse;

[#derive(Debug, Default)]
pub struct TouchRequest {
    expiration: u32,
}



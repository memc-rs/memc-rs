use serde_derive::{Deserialize, Serialize};

pub enum Magic {
    Request = 0x80,
    Response = 0x81,
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
    NotEnoughMemory = 0x82,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct RequestHeader {
    pub magic: u8,
    pub opcode: u8,
    pub key_length: u16,
    pub extras_length: u8,
    pub data_type: u8,
    pub reserved: u16,
    pub body_length: u32,
    pub opaque: u32,
    pub cas: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseHeader {
    magic: u8,
    opcode: u8,
    key_length: u16,
    extras_length: u8,
    data_type: u8,
    status: u16,
    body_length: u32,
    opaque: u32,
    cas: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    header: ResponseHeader,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Response {
    header: ResponseHeader,
}

pub type NoopRequest = Request;
pub type NoopResponse = Response;

#[derive(Serialize, Deserialize, Debug)]
pub struct GetRequest {
    header: RequestHeader,
    key: Vec<u8>,
}

pub type GetQuietRequest = GetRequest;
pub type GetKeyRequest = GetRequest;
pub type GetKeyQuietRequest = GetRequest;

#[derive(Serialize, Deserialize, Debug)]
pub struct GetResponse {
    header: ResponseHeader,
    flags: u32,
    key: Vec<u8>,
    value: Vec<u8>,
}

pub type DeleteRequest = GetRequest;
pub type DeleteResponse = Response;

pub type GetQuietlyResponse = GetResponse;
pub type GetKeyResponse = GetResponse;
pub type GetKeyQuietlyResponse = GetResponse;

#[derive(Serialize, Deserialize, Debug)]
pub struct SetRequest {
    header: RequestHeader,
    flags: u32,
    expiration: u32,
    key: Vec<u8>,
    value: Vec<u8>,
}
pub type AddRequest = SetRequest;
pub type ReplaceRequest = SetRequest;

pub type SetResponse = Response;
pub type AddResponse = Response;
pub type ReplaceResponse = Response;

#[derive(Serialize, Deserialize, Debug)]
pub struct IncrementRequest {
    header: RequestHeader,
    delta: u64,
    initial: u64,
    expiration: u32,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct IncrementResponse {
    value: u64,
}

pub type DecrementRequest = IncrementRequest;
pub type DecrementResponse = IncrementResponse;

#[derive(Serialize, Deserialize, Debug)]
pub struct TouchRequest {
    expiration: u32,
}

pub type TouchResponse = Response;

#[derive(Serialize, Deserialize, Debug)]
pub struct FlushRequest {
    header: RequestHeader,
    expiration: u32,
}
pub type FlushResponse = Response;

/* TODO Get And Touch (GAT) */

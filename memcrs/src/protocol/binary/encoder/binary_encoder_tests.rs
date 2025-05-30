#[allow(unused)]
use super::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::error;
    use crate::mock::value::from_string;

    fn create_response_header(
        cmd: network::Command,
        opaque: u32,
        cas: u64,
    ) -> network::ResponseHeader {
        let mut response_header = network::ResponseHeader::new(cmd as u8, opaque);
        response_header.cas = cas;
        response_header
    }

    fn encode_packet(src: BinaryResponse) -> ResponseMessage {
        let encoder = MemcacheBinaryEncoder::new();
        encoder.encode_message(&src)
    }

    fn test_encode(expected_result: &[u8], response: BinaryResponse) {
        let encode_result = encode_packet(response);
        assert_eq!(encode_result.data, expected_result);
    }

    #[test]
    fn encode_set_response() {
        let header = create_response_header(network::Command::Set, 0xDEAD_BEEF, 0x4FE6C1);
        let response = BinaryResponse::Set(network::SetResponse { header });
        let expected_result: [u8; 24] = [
            0x81, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xDE, 0xAD,
            0xBE, 0xEF, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4f, 0xe6, 0xc1,
        ];

        test_encode(&expected_result, response);
    }

    #[test]
    fn encode_replace_response() {
        let header = create_response_header(network::Command::Replace, 0, 4);
        let response = BinaryResponse::Replace(network::ReplaceResponse { header });
        let expected_result: [u8; 24] = [
            0x81, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04,
        ];
        test_encode(&expected_result, response);
    }

    #[test]
    fn encode_add_response() {
        let header = create_response_header(network::Command::Add, 0, 4);
        let response = BinaryResponse::Add(network::AddResponse { header });
        let expected_result: [u8; 24] = [
            0x81, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04,
        ];
        test_encode(&expected_result, response);
    }
    #[test]
    fn encode_append_response() {
        let header = create_response_header(network::Command::Append, 0, 2);
        let response = BinaryResponse::Append(network::AppendResponse { header });
        let expected_result: [u8; 24] = [
            0x81, 0x0e, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
        ];
        test_encode(&expected_result, response);
    }

    #[test]
    fn encode_prepend_response() {
        let header = create_response_header(network::Command::Prepend, 0, 3);
        let response = BinaryResponse::Prepend(network::PrependResponse { header });
        let expected_result: [u8; 24] = [
            0x81, 0x0f, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03,
        ];
        test_encode(&expected_result, response);
    }

    #[test]
    fn encode_get_key_quiet_response() {
        let mut expected_result = BytesMut::with_capacity(512);
        expected_result.put_slice(&[
            0x81, 0x0d, 0x00, 0x03, // key len
            0x04, // extras len
            0x00, 0x00, 0x00, // status
            0x00, 0x00, 0x00, 0x0b, // total body: 11
            0x00, 0x00, 0x00, 0x00, // opaque: 0
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, // cas: 1
            0x00, 0x00, 0x00, 0x00, // flags:
            0x66, 0x6f, 0x6f, // key: foo
            0x74, 0x65, 0x73, 0x74, // value: test
        ]);
        let mut header = create_response_header(network::Command::GetKeyQuiet, 0, 1);
        header.key_length = "foo".len() as u16;
        header.extras_length = 4;
        header.body_length = "foo".len() as u32 + "test".len() as u32 + header.extras_length as u32;
        let response = BinaryResponse::GetKeyQuietly(network::GetKeyQuietlyResponse {
            header,
            flags: 0,
            key: Bytes::from("foo"),
            value: from_string("test"),
        });
        let encode_result = encode_packet(response);
        assert_eq!(encode_result.data, expected_result);
    }

    #[test]
    fn encode_get_response() {
        let mut expected_result = BytesMut::with_capacity(512);
        expected_result.put_slice(&[
            0x81, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0d, 0x00, 0x00, 0x00, 0x00,
            0x33, 0x30, 0x35, 0x30,
        ]);
        let mut header = create_response_header(network::Command::Get, 0, 13);
        header.key_length = 0;
        header.extras_length = 4;
        header.body_length = "3050".len() as u32 + header.extras_length as u32;
        let response = BinaryResponse::Get(network::GetResponse {
            header,
            flags: 0,
            key: Bytes::new(),
            value: from_string("3050"),
        });
        let encode_result = encode_packet(response);
        assert_eq!(encode_result.data, expected_result);
    }

    #[test]
    fn encode_noop_response() {
        let header = create_response_header(network::Command::Noop, 0, 0);
        let expected_result: [u8; 24] = [
            0x81, 0x0a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let response = BinaryResponse::Noop(network::NoopResponse { header });
        test_encode(&expected_result, response);
    }

    #[test]
    fn encode_delete_response() {
        let header = create_response_header(network::Command::Delete, 0, 0);
        let expected_result: [u8; 24] = [
            0x81, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let response = BinaryResponse::Delete(network::DeleteResponse { header });
        test_encode(&expected_result, response);
    }

    #[test]
    fn encode_flush_response() {
        let expected_result: [u8; 24] = [
            0x81, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];

        let header = create_response_header(network::Command::Flush, 0, 0);
        let response = BinaryResponse::Flush(network::FlushResponse { header });
        test_encode(&expected_result, response);
    }

    #[test]
    fn encode_increment_response() {
        let expected_result: [u8; 32] = [
            0x81, 0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, // cas: 5
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0c, 0x1c, // value: 3100
        ];
        let mut header = create_response_header(network::Command::Increment, 0, 5);
        header.body_length = 8;
        let response = BinaryResponse::Increment(network::IncrementResponse {
            header,
            value: 3100,
        });
        test_encode(&expected_result, response);
    }

    #[test]
    fn encode_decrement_response() {
        let expected_result: [u8; 32] = [
            0x81, 0x06, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x0b, 0xea,
        ];
        let mut header = create_response_header(network::Command::Decrement, 0, 6);
        header.body_length = 8;
        let response = BinaryResponse::Decrement(network::DecrementResponse {
            header,
            value: 3050,
        });
        test_encode(&expected_result, response);
    }

    #[test]
    fn encode_version_response() {
        let expected_result: [u8; 29] = [
            0x81, 0x0b, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x31, 0x2e, 0x36, 0x2e,
            0x32,
        ];
        let mut header = create_response_header(network::Command::Version, 0, 0);
        header.body_length = "1.6.2".len() as u32;
        let response = BinaryResponse::Version(network::VersionResponse {
            header,
            version: String::from("1.6.2"),
        });
        test_encode(&expected_result, response);
    }

    #[test]
    fn encode_error_response() {
        let expected_result: [u8; 33] = [
            0x81, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x4e, 0x6f, 0x74, 0x20,
            0x66, 0x6f, 0x75, 0x6e, 0x64,
        ];
        let mut header = create_response_header(network::Command::Get, 0, 0);
        header.body_length = "Not found".len() as u32;
        let err = error::CacheError::NotFound;
        header.status = error::CacheError::NotFound as u16;
        let response = BinaryResponse::Error(network::ErrorResponse {
            header,
            error: err.to_static_string(),
        });
        test_encode(&expected_result, response);
    }
}

use super::*;

#[cfg(test)]
mod tests {

    use super::*;

    fn decode_packet(src: &[u8]) -> Result<Option<BinaryRequest>, io::Error> {
        let mut decoder = MemcacheBinaryCodec::new(1024);
        let mut buf = BytesMut::with_capacity(src.len());
        buf.put_slice(&src);
        decoder.decode(&mut buf)
    }

    #[test]
    fn decode_set_request() {
        let set_request_packet: [u8; 39] = [
            0x80, // magic
            0x01, // opcode
            0x00, 0x03, // key length
            0x08, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x0f, // total body length
            0xDE, 0xAD, 0xBE, 0xEF, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x01, // cas
            0xAB, 0xAD, 0xCA, 0xFE, // flags
            0x00, 0x00, 0x00, 0x32, // expiration
            0x66, 0x6f, 0x6f, // key 'foo'
            0x74, 0x65, 0x73, 0x74, // value 'test'
        ];
        let decode_result = decode_packet(&set_request_packet);
        match decode_result {
            Ok(set_request) => {
                assert!(set_request.is_some());
                if let Some(request) = set_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, binary::Command::Set as u8);
                    assert_eq!(header.key_length, 0x03);
                    assert_eq!(header.extras_length, 0x08);
                    assert_eq!(header.data_type, binary::DataTypes::RawBytes as u8);
                    assert_eq!(header.vbucket_id, 0x00);
                    assert_eq!(header.body_length, 0x0f);
                    assert_eq!(header.opaque, 0xDEADBEEF);
                    assert_eq!(header.cas, 0x01);
                    //
                    match request {
                        BinaryRequest::Set(req) => {
                            assert_eq!(req.flags, 0xabadcafe);
                            assert_eq!(req.expiration, 0x32);
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                            assert_eq!(req.value[..], [b't', b'e', b's', b't']);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_replace_request() {
        let set_request_packet: [u8; 39] = [
            0x80, // magic
            0x03, // opcode
            0x00, 0x03, // key length
            0x08, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x0f, // total body length
            0xDE, 0xAD, 0xBE, 0xEF, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x01, // cas
            0xAB, 0xAD, 0xCA, 0xFE, // flags
            0x00, 0x00, 0x00, 0x32, // expiration
            0x66, 0x6f, 0x6f, // key 'foo'
            0x74, 0x65, 0x73, 0x74, // value 'test'
        ];
        let decode_result = decode_packet(&set_request_packet);
        match decode_result {
            Ok(set_request) => {
                assert!(set_request.is_some());
                if let Some(request) = set_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, binary::Command::Replace as u8);

                    match request {
                        BinaryRequest::Replace(req) => {
                            assert_eq!(req.flags, 0xabadcafe);
                            assert_eq!(req.expiration, 0x32);
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                            assert_eq!(req.value[..], [b't', b'e', b's', b't']);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_add_request() {
        let set_request_packet: [u8; 38] = [
            0x80, 0x02, 0x00, 0x03, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0e, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x64, 0x66, 0x6f, 0x6f, 0x62, 0x61, 0x72,
        ];
        let decode_result = decode_packet(&set_request_packet);
        match decode_result {
            Ok(set_request) => {
                assert!(set_request.is_some());
                if let Some(request) = set_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, binary::Command::Add as u8);

                    match request {
                        BinaryRequest::Add(req) => {
                            assert_eq!(req.flags, 0);
                            assert_eq!(req.expiration, 100);
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                            assert_eq!(req.value[..], [b'b', b'a', b'r']);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_get_request() {
        let get_request_packet: [u8; 27] = [
            0x80, // magic
            0x00, // opcode
            0x00, 0x03, // key len
            0x00, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x03, // total body length
            0x00, 0x00, 0x00, 0x00, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, // cas
            0x66, 0x6f, 0x6f, // key 'foo'
        ];

        let decode_result = decode_packet(&get_request_packet);
        match decode_result {
            Ok(set_request) => {
                assert!(set_request.is_some());
                if let Some(request) = set_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, binary::Command::Get as u8);
                    assert_eq!(header.key_length, 0x03);
                    assert_eq!(header.extras_length, 0x00);
                    assert_eq!(header.data_type, binary::DataTypes::RawBytes as u8);
                    assert_eq!(header.vbucket_id, 0x00);
                    assert_eq!(header.body_length, 0x03);
                    assert_eq!(header.opaque, 0x00000000);
                    assert_eq!(header.cas, 0x00);
                    //
                    match request {
                        BinaryRequest::Get(req) => {
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }
    #[test]
    fn decode_get_quiet_request() {
        let get_request_packet: [u8; 27] = [
            0x80, // magic
            0x09, // opcode
            0x00, 0x03, // key len
            0x00, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x03, // total body length
            0x00, 0x00, 0x00, 0x00, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, // cas
            0x66, 0x6f, 0x6f, // key 'foo'
        ];

        let decode_result = decode_packet(&get_request_packet);
        match decode_result {
            Ok(set_request) => {
                assert!(set_request.is_some());
                if let Some(request) = set_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, binary::Command::GetQuiet as u8);
                    assert_eq!(header.key_length, 0x03);
                    assert_eq!(header.extras_length, 0x00);
                    assert_eq!(header.data_type, binary::DataTypes::RawBytes as u8);
                    assert_eq!(header.vbucket_id, 0x00);
                    assert_eq!(header.body_length, 0x03);
                    assert_eq!(header.opaque, 0x00000000);
                    assert_eq!(header.cas, 0x00);
                    //
                    match request {
                        BinaryRequest::GetQuietly(req) => {
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_get_key_request() {
        let get_request_packet: [u8; 27] = [
            0x80, // magic
            0x0c, // opcode
            0x00, 0x03, // key len
            0x00, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x03, // total body length
            0x00, 0x00, 0x00, 0x00, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, // cas
            0x66, 0x6f, 0x6f, // key 'foo'
        ];

        let decode_result = decode_packet(&get_request_packet);
        match decode_result {
            Ok(set_request) => {
                assert!(set_request.is_some());
                if let Some(request) = set_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, binary::Command::GetKey as u8);
                    //
                    match request {
                        BinaryRequest::GetKey(req) => {
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_get_key_quiet_request() {
        let get_request_packet: [u8; 27] = [
            0x80, // magic
            0x0D, // opcode
            0x00, 0x03, // key len
            0x00, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x03, // total body length
            0x00, 0x00, 0x00, 0x00, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, // cas
            0x66, 0x6f, 0x6f, // key 'foo'
        ];

        let decode_result = decode_packet(&get_request_packet);
        match decode_result {
            Ok(set_request) => {
                assert!(set_request.is_some());
                if let Some(request) = set_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, binary::Command::GetKeyQuiet as u8);
                    //
                    match request {
                        BinaryRequest::GetKeyQuietly(req) => {
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_if_buffer_doesnt_contain_full_header_none_should_be_returned() {
        let set_request_packet: [u8; 4] = [
            0x80, // magic
            0x01, // opcode
            0x00, 0x03, // key length
        ];
        let decode_result = decode_packet(&set_request_packet);
        match decode_result {
            Ok(set_request) => {
                assert!(set_request.is_none());
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_if_buffer_doesnt_contain_full_packet_none_should_be_returned() {
        let set_request_packet: [u8; 24] = [
            0x80, // magic
            0x01, // opcode
            0x00, 0x03, // key length
            0x08, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x0f, // total body length
            0xDE, 0xAD, 0xBE, 0xEF, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x01, // cas
        ];
        let decode_result = decode_packet(&set_request_packet);
        match decode_result {
            Ok(set_request) => {
                assert!(set_request.is_none());
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_check_if_error_on_incorrect_magic() {
        let set_request_packet: [u8; 24] = [
            0x81, // magic
            0x01, // opcode
            0x00, 0x03, // key length
            0x08, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x0f, // total body length
            0xDE, 0xAD, 0xBE, 0xEF, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x01, // cas
        ];
        let decode_result = decode_packet(&set_request_packet);
        match decode_result {
            Ok(_) => unreachable!(),
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::InvalidData);
            }
        }
    }

    #[test]
    fn decode_if_key_length_too_large_error_should_be_returned() {
        let get_request_packet: [u8; 27] = [
            0x80, // magic
            0x00, // opcode
            0xff, 0xff, // key len
            0x00, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x03, // total body length
            0x00, 0x00, 0x00, 0x00, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, // cas
            0x66, 0x6f, 0x6f, // key 'foo'
        ];
        let decode_result = decode_packet(&get_request_packet);
        match decode_result {
            Ok(_) => unreachable!(),
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::InvalidData);
            }
        }
    }

    #[test]
    fn decode_if_extras_length_too_large_error_should_be_returned() {
        let set_request_packet: [u8; 39] = [
            0x80, // magic
            0x01, // opcode
            0x00, 0x03, // key length
            0x15, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x0f, // total body length
            0xDE, 0xAD, 0xBE, 0xEF, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x01, // cas
            0xAB, 0xAD, 0xCA, 0xFE, // flags
            0x00, 0x00, 0x00, 0x32, // expiration
            0x66, 0x6f, 0x6f, // key 'foo'
            0x74, 0x65, 0x73, 0x74, // value 'test'
        ];

        let decode_result = decode_packet(&set_request_packet);
        match decode_result {
            Ok(_) => unreachable!(),
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::InvalidData);
            }
        }
    }

    #[test]
    fn decode_body_length_should_be_greater_than_key_len_and_extras_len() {
        let set_request_packet: [u8; 39] = [
            0x80, // magic
            0x01, // opcode
            0x00, 0x03, // key length
            0x08, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x0A, // total body length
            0xDE, 0xAD, 0xBE, 0xEF, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x01, // cas
            0xAB, 0xAD, 0xCA, 0xFE, // flags
            0x00, 0x00, 0x00, 0x32, // expiration
            0x66, 0x6f, 0x6f, // key 'foo'
            0x74, 0x65, 0x73, 0x74, // value 'test'
        ];

        let decode_result = decode_packet(&set_request_packet);
        match decode_result {
            Ok(_) => unreachable!(),
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::InvalidData);
            }
        }
    }

    #[test]
    fn decode_if_opcode_is_greater_than_opcode_max_error_should_be_returned() {
        let set_request_packet: [u8; 39] = [
            0x80, // magic
            0x25, // opcode
            0x00, 0x03, // key length
            0x08, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x0A, // total body length
            0xDE, 0xAD, 0xBE, 0xEF, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x01, // cas
            0xAB, 0xAD, 0xCA, 0xFE, // flags
            0x00, 0x00, 0x00, 0x32, // expiration
            0x66, 0x6f, 0x6f, // key 'foo'
            0x74, 0x65, 0x73, 0x74, // value 'test'
        ];

        let decode_result = decode_packet(&set_request_packet);
        match decode_result {
            Ok(_) => unreachable!(),
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::InvalidData);
            }
        }
    }

    #[test]
    fn decode_data_type_should_be_0() {
        let set_request_packet: [u8; 39] = [
            0x80, // magic
            0x01, // opcode
            0x00, 0x03, // key length
            0x08, // extras length
            0xff, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x0A, // total body length
            0xDE, 0xAD, 0xBE, 0xEF, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x01, // cas
            0xAB, 0xAD, 0xCA, 0xFE, // flags
            0x00, 0x00, 0x00, 0x32, // expiration
            0x66, 0x6f, 0x6f, // key 'foo'
            0x74, 0x65, 0x73, 0x74, // value 'test'
        ];

        let decode_result = decode_packet(&set_request_packet);
        match decode_result {
            Ok(_) => unreachable!(),
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::InvalidData);
            }
        }
    }

    #[test]
    fn decode_append_request() {
        let append_request_packet: [u8; 30] = [
            0x80, // magic
            0x0e, // opcode
            0x00, 0x03, // key length
            0x00, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x06, // total body length
            0x00, 0x00, 0x00, 0x00, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, // cas
            0x66, 0x6f, 0x6f, // key 'foo'
            0x62, 0x61, 0x73, // value 'bas'
        ];

        let decode_result = decode_packet(&append_request_packet);
        match decode_result {
            Ok(append_request) => {
                assert!(append_request.is_some());
                if let Some(request) = append_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, binary::Command::Append as u8);
                    assert_eq!(header.key_length, 0x03);
                    assert_eq!(header.extras_length, 0x00);
                    assert_eq!(header.data_type, binary::DataTypes::RawBytes as u8);
                    assert_eq!(header.vbucket_id, 0x00);
                    assert_eq!(header.body_length, 0x06);
                    assert_eq!(header.opaque, 0x00000000);
                    assert_eq!(header.cas, 0x00);
                    //
                    match request {
                        BinaryRequest::Append(req) => {
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                            assert_eq!(req.value[..], [b'b', b'a', b's']);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_prepend_request() {
        let prepend_request_packet: [u8; 30] = [
            0x80, // magic
            0x0f, // opcode
            0x00, 0x03, // key length
            0x00, // extras length
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x06, // total body length
            0x00, 0x00, 0x00, 0x00, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, // cas
            0x66, 0x6f, 0x6f, // key 'foo'
            0x62, 0x69, 0x73, // value 'bis'
        ];

        let decode_result = decode_packet(&prepend_request_packet);
        match decode_result {
            Ok(prepend_request) => {
                assert!(prepend_request.is_some());
                if let Some(request) = prepend_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, binary::Command::Prepend as u8);
                    assert_eq!(header.key_length, 0x03);
                    assert_eq!(header.extras_length, 0x00);
                    assert_eq!(header.data_type, binary::DataTypes::RawBytes as u8);
                    assert_eq!(header.vbucket_id, 0x00);
                    assert_eq!(header.body_length, 0x06);
                    assert_eq!(header.opaque, 0x00000000);
                    assert_eq!(header.cas, 0x00);
                    //
                    match request {
                        BinaryRequest::Prepend(req) => {
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                            assert_eq!(req.value[..], [b'b', b'i', b's']);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_delete_request() {
        let prepend_request_packet: [u8; 27] = [
            0x80, // magic
            0x04, // opcode
            0x00, 0x03, // key len
            0x00, // extras len
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x03, // total body len
            0x00, 0x00, 0x00, 0x00, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, // cas
            0x66, 0x6f, 0x6f, // key 'foo'
        ];

        let decode_result = decode_packet(&prepend_request_packet);
        match decode_result {
            Ok(delete_request) => {
                assert!(delete_request.is_some());
                if let Some(request) = delete_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, binary::Command::Delete as u8);
                    assert_eq!(header.key_length, 0x03);
                    assert_eq!(header.extras_length, 0x00);
                    assert_eq!(header.data_type, binary::DataTypes::RawBytes as u8);
                    assert_eq!(header.vbucket_id, 0x00);
                    assert_eq!(header.body_length, 0x03);
                    assert_eq!(header.opaque, 0x00000000);
                    assert_eq!(header.cas, 0x00);
                    //
                    match request {
                        BinaryRequest::Delete(req) => {
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_delete_quiet_request() {
        let delete_request_packet: [u8; 27] = [
            0x80, // magic
            0x14, // opcode
            0x00, 0x03, // key len
            0x00, // extras len
            0x00, // data type
            0x00, 0x00, // vbucket id
            0x00, 0x00, 0x00, 0x03, // total body len
            0x00, 0x00, 0x00, 0x00, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, // cas
            0x66, 0x6f, 0x6f, // key 'foo'
        ];

        let decode_result = decode_packet(&delete_request_packet);
        match decode_result {
            Ok(delete_request) => {
                assert!(delete_request.is_some());
                if let Some(request) = delete_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, binary::Command::DeleteQuiet as u8);
                    assert_eq!(header.key_length, 0x03);
                    assert_eq!(header.extras_length, 0x00);
                    assert_eq!(header.data_type, binary::DataTypes::RawBytes as u8);
                    assert_eq!(header.vbucket_id, 0x00);
                    assert_eq!(header.body_length, 0x03);
                    assert_eq!(header.opaque, 0x00000000);
                    assert_eq!(header.cas, 0x00);
                    //
                    match request {
                        BinaryRequest::DeleteQuiet(req) => {
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    fn inc_dec_request_check_header(
        decode_result: &Result<Option<BinaryRequest>, io::Error>,
        opcode: binary::Command,
    ) {
        match decode_result {
            Ok(incr_request) => {
                assert!(incr_request.is_some());
                if let Some(request) = incr_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, opcode as u8);
                    assert_eq!(header.key_length, 0x03);
                    assert_eq!(header.extras_length, 0x14);
                    assert_eq!(header.data_type, binary::DataTypes::RawBytes as u8);
                    assert_eq!(header.vbucket_id, 0x00);
                    assert_eq!(header.body_length, 0x17);
                    assert_eq!(header.opaque, 0x00000000);
                    assert_eq!(header.cas, 0x00);
                    //
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_increment_request() {
        let increment_request_packet: [u8; 47] = [
            0x80, // magic
            0x05, // opcode
            0x00, 0x03, //key len
            0x14, // extras len
            0x00, // data type
            0x00, 0x00, //vbucket id
            0x00, 0x00, 0x00, 0x17, // total body len
            0x00, 0x00, 0x00, 0x00, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64, // amount to add
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Initial value
            0x00, 0x00, 0x00, 0x00, // Expiration
            0x66, 0x6f, 0x6f, // key 'foo'
        ];

        let decode_result = decode_packet(&increment_request_packet);
        inc_dec_request_check_header(&decode_result, binary::Command::Increment);
        match decode_result {
            Ok(incr_request) => {
                assert!(incr_request.is_some());
                if let Some(request) = incr_request {
                    //
                    match request {
                        BinaryRequest::Increment(req) => {
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                            assert_eq!(req.delta, 100);
                            assert_eq!(req.initial, 0);
                            assert_eq!(req.expiration, 0);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_increment_quiet_request() {
        let increment_request_packet: [u8; 47] = [
            0x80, // magic
            0x15, // opcode
            0x00, 0x03, //key len
            0x14, // extras len
            0x00, // data type
            0x00, 0x00, //vbucket id
            0x00, 0x00, 0x00, 0x17, // total body len
            0x00, 0x00, 0x00, 0x00, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x65, // amount to add
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, // Initial value
            0x00, 0xff, 0x00, 0x00, // Expiration
            0x66, 0x6f, 0x6f, // key 'foo'
        ];

        let decode_result = decode_packet(&increment_request_packet);
        inc_dec_request_check_header(&decode_result, binary::Command::IncrementQuiet);
        match decode_result {
            Ok(incr_request) => {
                assert!(incr_request.is_some());
                if let Some(request) = incr_request {
                    //
                    match request {
                        BinaryRequest::IncrementQuiet(req) => {
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                            assert_eq!(req.delta, 101);
                            assert_eq!(req.initial, 1);
                            assert_eq!(req.expiration, 0x00ff0000);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_decrement_request() {
        let decrement_request_packet: [u8; 47] = [
            0x80, // magic
            0x06, // opcode
            0x00, 0x03, //key len
            0x14, // extras len
            0x00, // data type
            0x00, 0x00, //vbucket id
            0x00, 0x00, 0x00, 0x17, // total body len
            0x00, 0x00, 0x00, 0x00, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x64, // amount to add
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // Initial value
            0x00, 0x00, 0x00, 0x00, // Expiration
            0x66, 0x6f, 0x6f, // key 'foo'
        ];

        let decode_result = decode_packet(&decrement_request_packet);
        inc_dec_request_check_header(&decode_result, binary::Command::Decrement);
        match decode_result {
            Ok(decr_request) => {
                assert!(decr_request.is_some());
                if let Some(request) = decr_request {
                    //
                    match request {
                        BinaryRequest::Decrement(req) => {
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                            assert_eq!(req.delta, 100);
                            assert_eq!(req.initial, 0);
                            assert_eq!(req.expiration, 0);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_decrement_quiet_request() {
        let decrement_request_packet: [u8; 47] = [
            0x80, // magic
            0x16, // opcode
            0x00, 0x03, //key len
            0x14, // extras len
            0x00, // data type
            0x00, 0x00, //vbucket id
            0x00, 0x00, 0x00, 0x17, // total body len
            0x00, 0x00, 0x00, 0x00, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x66, // amount to add
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10, // Initial value
            0xDE, 0xAD, 0xBE, 0xEF, // Expiration
            0x66, 0x6f, 0x6f, // key 'foo'
        ];

        let decode_result = decode_packet(&decrement_request_packet);
        inc_dec_request_check_header(&decode_result, binary::Command::DecrementQuiet);
        match decode_result {
            Ok(decr_request) => {
                assert!(decr_request.is_some());
                if let Some(request) = decr_request {
                    //
                    match request {
                        BinaryRequest::DecrementQuiet(req) => {
                            assert_eq!(req.key, [b'f', b'o', b'o']);
                            assert_eq!(req.delta, 102);
                            assert_eq!(req.initial, 16);
                            assert_eq!(req.expiration, 0xDEADBEEF);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_noop_request() {
        decode_header_only_request(binary::Command::Noop);
    }

    #[test]
    fn decode_version_request() {
        decode_header_only_request(binary::Command::Version);
    }

    fn decode_header_only_request(opcode: binary::Command) {
        let noop_request_packet: [u8; 24] = [
            0x80,         // magic
            opcode as u8, // opcode
            0x00,
            0x00, //key len
            0x00, // extras len
            0x00, // data type
            0x00,
            0x00, //vbucket id
            0x00,
            0x00,
            0x00,
            0x00, // total body len
            0x00,
            0x00,
            0x00,
            0x00, // opaque
            0x00,
            0x00,
            0x00,
            0x00, // cas
            0x00,
            0x00,
            0x00,
            0x00, // cas
        ];

        let decode_result = decode_packet(&noop_request_packet);
        match decode_result {
            Ok(noop_request) => {
                assert!(noop_request.is_some());
                if let Some(request) = noop_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, opcode as u8);
                    assert_eq!(header.key_length, 0x00);
                    assert_eq!(header.extras_length, 0x00);
                    assert_eq!(header.data_type, binary::DataTypes::RawBytes as u8);
                    assert_eq!(header.vbucket_id, 0x00);
                    assert_eq!(header.body_length, 0x00);
                    assert_eq!(header.opaque, 0x00000000);
                    assert_eq!(header.cas, 0x00);
                    //
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_flush_with_expiration_request() {
        let flush_request_packet: [u8; 28] = [
            0x80, // magic
            0x08, // opcode
            0x00, 0x00, //key len
            0x04, // extras len
            0x00, // data type
            0x00, 0x00, //vbucket id
            0x00, 0x00, 0x00, 0x04, // total body len
            0x00, 0x00, 0x00, 0x00, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x64, // expiration 100
        ];

        let decode_result = decode_packet(&flush_request_packet);
        match decode_result {
            Ok(flush_request) => {
                assert!(flush_request.is_some());
                if let Some(request) = flush_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, binary::Command::Flush as u8);
                    assert_eq!(header.key_length, 0x00);
                    assert_eq!(header.extras_length, 0x04);
                    assert_eq!(header.data_type, binary::DataTypes::RawBytes as u8);
                    assert_eq!(header.vbucket_id, 0x00);
                    assert_eq!(header.body_length, 0x04);
                    assert_eq!(header.opaque, 0x00000000);
                    assert_eq!(header.cas, 0x00);

                    match request {
                        BinaryRequest::Flush(req) => {
                            assert_eq!(req.expiration, 100);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }

    #[test]
    fn decode_flush_request() {
        let flush_request_packet: [u8; 24] = [
            0x80, // magic
            0x08, // opcode
            0x00, 0x00, //key len
            0x00, // extras len
            0x00, // data type
            0x00, 0x00, //vbucket id
            0x00, 0x00, 0x00, 0x00, // total body len
            0x00, 0x00, 0x00, 0x00, // opaque
            0x00, 0x00, 0x00, 0x00, // cas
            0x00, 0x00, 0x00, 0x00, // cas
        ];

        let decode_result = decode_packet(&flush_request_packet);
        match decode_result {
            Ok(flush_request) => {
                assert!(flush_request.is_some());
                if let Some(request) = flush_request {
                    let header = request.get_header();
                    assert_eq!(header.magic, binary::Magic::Request as u8);
                    assert_eq!(header.opcode, binary::Command::Flush as u8);
                    assert_eq!(header.key_length, 0x00);
                    assert_eq!(header.extras_length, 0x00);
                    assert_eq!(header.data_type, binary::DataTypes::RawBytes as u8);
                    assert_eq!(header.vbucket_id, 0x00);
                    assert_eq!(header.body_length, 0x00);
                    assert_eq!(header.opaque, 0x00000000);
                    assert_eq!(header.cas, 0x00);

                    match request {
                        BinaryRequest::Flush(req) => {
                            assert_eq!(req.expiration, 0);
                        }
                        _ => unreachable!(),
                    }
                }
            }
            Err(_) => unreachable!(),
        }
    }
    #[test]
    fn decode_fuzz_crash1_request() {
        let crash_request_packet: [u8; 29] = [
            128, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 255, 255, 0, 255, 126, 39, 0, 0, 2, 239,
            191, 191, 210, 27,
        ];
        let decode_result = decode_packet(&crash_request_packet);
        match decode_result {
            Ok(_) => unreachable!(),
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::InvalidData);
            }
        }
    }
    #[test]
    fn decode_fuzz_crash2_request() {
        let crash_request_packet: [u8; 25] = [
            128, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 96, 255, 255, 254, 63, 255, 4, 93, 64,
            27,
        ];
        let decode_result = decode_packet(&crash_request_packet);
        match decode_result {
            Ok(_) => unreachable!(),
            Err(err) => {
                assert_eq!(err.kind(), io::ErrorKind::InvalidData);
            }
        }
    }
}

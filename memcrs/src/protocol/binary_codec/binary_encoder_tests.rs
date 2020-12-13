use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    fn create_response_header(cmd: binary::Command, opaque: u32, cas: u64) -> binary::ResponseHeader {
        let mut response_header =
        binary::ResponseHeader::new(binary::Command::Set as u8, 0xDEAD_BEEF);
        response_header.cas = cas;
        response_header
    }

    fn encode_packet(src: BinaryResponse) -> Result<BytesMut, io::Error> {
        let mut encoder = MemcacheBinaryCodec::new();
        let mut buf = BytesMut::with_capacity(128);        
        match encoder.encode(src, &mut buf) {
            Ok(_) => Ok(buf),
            Err(err) => Err(err)
        }        
    }

    #[test]
    fn encode_set_response() {
        let header = create_response_header(binary::Command::Set, 0xDEAD_BEEF, 0x4fE6C1);
        let response = BinaryResponse::Set(binary::SetResponse {
            header,
        });               
        let expected_result = [0x81, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 
            0xDE, 0xAD, 0xBE, 0xEF, 
            0x00, 0x00, 0x00, 0x00, 0x00, 0x4f, 0xe6, 0xc1];
        
        let encode_result = encode_packet(response);
        match encode_result {
            Ok(buf) => {
                assert_eq!(&buf[..], expected_result);
            },
            Err(_) => unreachable!()
        }
        
    }
}


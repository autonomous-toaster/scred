use scred_http::h2::h2_hpack_rfc7541::HpackEncoder;

#[test]
fn test_encode_status_200() {
    let headers = vec![];
    let result = HpackEncoder::encode_response_headers(200, &headers);
    match result {
        Ok(payload) => {
            println!("SUCCESS: payload len={}, bytes={:02x?}", payload.len(), payload);
            assert_eq!(payload.len(), 1);
            assert_eq!(payload[0], 0x88);
        }
        Err(e) => {
            panic!("FAILED: {}", e);
        }
    }
}

#[test] 
fn test_encode_status_200_with_header() {
    let headers = vec![("content-type".to_string(), "application/json".to_string())];
    let result = HpackEncoder::encode_response_headers(200, &headers);
    match result {
        Ok(payload) => {
            println!("SUCCESS with header: payload len={}, bytes={:02x?}", payload.len(), &payload[..payload.len().min(20)]);
        }
        Err(e) => {
            panic!("FAILED with header: {}", e);
        }
    }
}

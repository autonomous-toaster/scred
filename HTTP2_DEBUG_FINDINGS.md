# HTTP/2 Framing Error - Debug Findings

## Summary
Successfully ran actual curl test against SCRED MITM HTTP/2 server.
**Root cause identified**: HPACK decoder bug, NOT encoder.

## Test Execution
```bash
# Server running on 127.0.0.1:8080
http_proxy="" https_proxy="" RUST_LOG=debug cargo run --bin scred-mitm

# Client request
curl -vk -x http://127.0.0.1:8080 https://httpbin.org/anything
```

## What Works ✓
1. MITM server listens on port 8080
2. Client connects and performs CONNECT tunnel
3. TLS handshake succeeds (self-signed cert via rcgen)
4. ALPN negotiation succeeds (both offer h2)
5. HTTP/2 preface exchange succeeds
6. SETTINGS frame exchange succeeds  
7. HEADERS frame received correctly (36 bytes)

## What Fails ✗
```
HPACK decode error: Value truncated
→ H2Request creation fails
→ Handler never called
→ No response sent
→ Client receives GOAWAY from curl timeout
→ curl error: (16) Error in the HTTP2 framing layer
```

## Detailed Trace
```
Log: END_HEADERS received on stream 1
Log: Complete HPACK block retrieved, 36 bytes
Log: HPACK decode error: Value truncated
Log: Failed to create H2Request or END_STREAM not set
```

## Root Cause
**File**: `crates/scred-http/src/h2/h2_hpack_integration.rs`  
**Function**: `decode_literal()`  
**Issue**: Incorrect byte offset calculation or missing RFC 7541 integer encoding

### RFC 7541 Integer Encoding (Section 5.1)
The standard uses variable-length integer encoding for field sizes:
- Values 0-126 fit in 1 byte
- Values >= 127 require multiple bytes with continuation flag

Our decoder likely fails when:
1. Name length > 127 bytes
2. Value length > 127 bytes
3. Static table index requires multi-byte encoding

### Current Implementation Problem
```rust
let len = data[0] as usize;  // Assumes 1-byte length
if data.len() < 1 + len {
    return Err(anyhow!("Name truncated"));
}
```

This doesn't handle RFC 7541 Section 5.1 integer encoding!

## Fix Required
Implement RFC 7541 Section 5.1 integer decoding:

```rust
fn decode_integer(data: &[u8], prefix_bits: u8) -> Result<(u64, usize)> {
    let prefix_mask = (1u8 << prefix_bits) - 1;
    let first = data[0] & prefix_mask;
    
    if (first as u64) < 127u64 {
        return Ok((first as u64, 1));
    }
    
    let mut value = 127u64;
    let mut multiplier = 1u64;
    let mut index = 1;
    
    loop {
        if index >= data.len() {
            return Err(anyhow!("Integer truncated"));
        }
        let byte = data[index] as u64;
        value = value.checked_add(((byte & 0x7F) * multiplier))
            .ok_or_else(|| anyhow!("Integer overflow"))?;
        index += 1;
        
        if (byte & 0x80) == 0 {
            break;
        }
        multiplier = multiplier.checked_mul(128)
            .ok_or_else(|| anyhow!("Integer overflow"))?;
    }
    
    Ok((value, index))
}
```

## Next Steps
1. Implement RFC 7541 integer decoding
2. Update `decode_literal()` to use it for:
   - Name length (when parsing new names)
   - Value length (for values)
3. Update `decode_header_at()` if needed for multi-byte indices
4. Add test case with large headers (> 127 bytes)
5. Re-test with curl

## Files Impacted
- `crates/scred-http/src/h2/h2_hpack_integration.rs`

## Commits So Far
- c5c2785: RFC 7540 compliance fixes (stream ID, response encoding)
- aa3a33e: H2 handler integration
- 027ade9: Diagnostic logging (revealed the true issue)

## Architecture: Before and After

### Before (Broken)
```
┌──────────────────────────────────────────────────────────┐
│  tls_mitm.rs (1,500 LOC)                                 │
│  ┌────────────────────────────────────────────────────┐  │
│  │ Custom HTTP/2 Implementation (DELETED)             │  │
│  │ • Frame::parse() - Manual frame parsing            │  │
│  │ • HPACK decoder - Custom header compression        │  │
│  │ • Stream state machine - Manual stream management  │  │
│  │ • Frame forwarding - Manual frame writing          │  │
│  └────────────────────────────────────────────────────┘  │
│  ↓ References modules that don't exist                   │
│  ❌ COMPILATION FAILED                                   │
└──────────────────────────────────────────────────────────┘
```

### After (Using `h2` Crate)
```
┌──────────────────────────────────────────────────────────┐
│  tls_mitm.rs (~800 LOC)                                  │
│  ┌────────────────────────────────────────────────────┐  │
│  │ h2::server Integration                             │  │
│  │ • Accept incoming H2 connections                   │  │
│  │ • Receive requests (already parsed)                │  │
│  │ • Send responses (auto-framed)                     │  │
│  └────────────────────────────────────────────────────┘  │
│  ↓                                                        │
│  ┌────────────────────────────────────────────────────┐  │
│  │ Redaction Layer (Our Code)                         │  │
│  │ • Intercept requests                               │  │
│  │ • Apply redaction engine                           │  │
│  │ • Apply policies                                   │  │
│  └────────────────────────────────────────────────────┘  │
│  ↓                                                        │
│  ┌────────────────────────────────────────────────────┐  │
│  │ h2::client Integration                             │  │
│  │ • Connect to upstream                              │  │
│  │ • Send requests                                    │  │
│  │ • Receive responses                                │  │
│  └────────────────────────────────────────────────────┘  │
│  ✅ COMPILATION SUCCESS                                  │
└──────────────────────────────────────────────────────────┘
```

## Migration Details

### 1. Client-Side H2 (MITM receiving from client)

**Current (Broken):**
```rust
use scred_http::h2::frame::{Frame, FrameType};

// Read frame header manually
let mut header = [0u8; 9];
conn.read_exact(&mut header).await?;

// Parse frame
let frame = Frame::parse(&header)?;

// Dispatch on frame type
match frame.frame_type {
    FrameType::Headers => {
        // Decode HPACK manually
        let headers = hpack_decoder.decode(&frame.payload)?;
        // ...
    }
    FrameType::Data => {
        // Handle data frame
        // ...
    }
}
```

**New (Using `h2`):**
```rust
use h2::server;

// Handshake with client
let mut h2_conn = server::handshake(conn).await?;

// Accept requests (frames handled automatically)
while let Some(result) = h2_conn.accept().await {
    let (request, send_response) = result?;
    
    // Request is already parsed, headers decoded
    let method = request.method();
    let uri = request.uri();
    let headers = request.headers();
    
    // Apply redaction
    let redacted_request = apply_redaction(request)?;
    
    // Send response
    let response = handle_request(redacted_request)?;
    send_response.send_response(response, true)?;
}
```

### 2. Server-Side H2 (MITM connecting to upstream)

**Current (Broken):**
```rust
use scred_http::h2::h2_upstream_client::H2UpstreamClient;

let client = H2UpstreamClient::new();
let response = client.send_request(request).await?;
```

**New (Using `h2`):**
```rust
use h2::client;
use http::Request;

// Handshake with upstream
let (mut h2_client, h2_connection) = client::handshake(upstream_io).await?;

// Spawn connection driver
tokio::spawn(async move {
    h2_connection.await.unwrap();
});

// Send request
let request = Request::builder()
    .method("GET")
    .uri("https://example.com")
    .body(())
    .unwrap();

let (response, _) = h2_client.send_request(request)?;
let response = response.await?;
```

### 3. HPACK Handling

**Current (Broken):**
```rust
use scred_http::h2::hpack::HpackDecoder;

let mut decoder = HpackDecoder::new();
let headers = decoder.decode(&payload)?;
```

**New (Using `h2`):**
```rust
// HPACK is handled automatically by h2 crate
// Headers are already decoded when you receive the request
let headers = request.headers();  // Already decoded!
```

### 4. Frame Forwarding

**Current (Broken):**
```rust
use scred_http::h2::frame_forwarder::{forward_h2_frames, FrameForwarderConfig};

let config = FrameForwarderConfig::default();
forward_h2_frames(client_conn, upstream_conn, config).await?;
```

**New (Using `h2`):**
```rust
// No manual frame forwarding needed
// h2 crate manages streams and frames automatically
// We work at the request/response level

let (request, send_response) = h2_conn.accept().await?;
let modified_request = modify_request(request);
let upstream_response = forward_to_upstream(modified_request).await?;
send_response.send_response(upstream_response, true)?;
```

## Data Flow

### Request Path
```
Client TLS Connection
    ↓
h2::server::handshake()
    ↓ (automatic frame parsing, HPACK decoding)
Request Object (http::Request)
    ↓
[REDACTION HOOK] ← Our code intercepts here
    ↓
Policy Application
    ↓
h2::client::send_request()
    ↓ (automatic frame writing, HPACK encoding)
Upstream TLS Connection
```

### Response Path
```
Upstream TLS Connection
    ↓
h2::client::recv_response()
    ↓ (automatic frame parsing, HPACK decoding)
Response Object (http::Response)
    ↓
[REDACTION HOOK] ← Our code intercepts here
    ↓
Policy Application
    ↓
h2::server::send_response()
    ↓ (automatic frame writing, HPACK encoding)
Client TLS Connection
```

## Implementation Phases

### Phase 1: Remove Broken Code (Day 1)
- Remove imports for non-existent modules
- Delete backup files
- Comment out broken functions temporarily
- Verify what actually compiles

### Phase 2: Basic H2 Integration (Days 2-3)
- Add `h2` crate to dependencies (if not already there)
- Create minimal `h2::server` handshake in `tls_mitm.rs`
- Create minimal `h2::client` handshake for upstream
- Verify basic connectivity

### Phase 3: Request/Response Interception (Days 4-5)
- Integrate redaction engine at request level
- Integrate policy application
- Test with simple requests

### Phase 4: Full Feature Parity (Days 6-10)
- Implement all redaction modes
- Implement all policy types
- Handle edge cases (chunked encoding, trailers, etc.)
- Error handling

### Phase 5: Cleanup and Testing (Days 11-12)
- Remove old code paths
- Write integration tests
- Verify no regressions
- Run `just ci`

## Testing Strategy

### Unit Tests (New, Easy)
```rust
#[test]
fn test_redact_credit_card_in_request() {
    let request = create_request_with_body("card: 1234-5678-9012-3456");
    let redacted = apply_redaction(request);
    assert!(redacted.body.contains("card: ****-****-****-3456"));
}
```

### Integration Tests (Critical)
```rust
#[tokio::test]
async fn test_h2_mitm_basic_request() {
    // Set up mock H2 server (upstream)
    // Connect through MITM
    // Send H2 request
    // Verify request received by upstream
    // Verify redaction applied
}
```

### Manual Testing (Required)
- Test with real browsers (Chrome, Firefox)
- Test with curl --http2
- Test with various upstream servers
- Verify TLS certificates work correctly

## Dependencies

**Required:**
```toml
[dependencies]
h2 = "0.4"  # Already present?
http = "1.0"  # Already present?
tokio = { version = "1", features = ["full"] }
```

**Already in `scred-http/Cargo.toml`:**
- `h2` crate (check version)
- `http` crate
- `tokio`

**No new dependencies needed** - just use what's already there correctly.

## Migration Checklist

- [ ] Remove old import statements
- [ ] Delete backup files
- [ ] Add `h2` imports
- [ ] Implement `h2::server` handshake
- [ ] Implement `h2::client` handshake
- [ ] Integrate redaction at request level
- [ ] Integrate redaction at response level
- [ ] Handle all HTTP methods
- [ ] Handle chunked encoding
- [ ] Handle trailers
- [ ] Handle errors gracefully
- [ ] Write integration tests
- [ ] Verify with real browsers
- [ ] Run `cargo check --workspace`
- [ ] Run `just ci`

## Success Metrics

**Compilation:**
- ✅ Zero compilation errors
- ✅ Zero warnings (or minimal, acceptable ones)
- ✅ `cargo build --workspace` succeeds

**Functionality:**
- ✅ MITM works with H2 clients
- ✅ Proxy works with H2 upstreams
- ✅ Redaction applied correctly
- ✅ Policies enforced correctly

**Code Quality:**
- ✅ Smaller functions (easier to test)
- ✅ Clear separation of concerns
- ✅ Less code to maintain
- ✅ Foundation for CRAP score improvements

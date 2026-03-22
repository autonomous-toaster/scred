/// Real Integration Tests: H2 MITM with Actual Request/Response Flow
/// 
/// These tests verify the complete H2 MITM data flow:
/// - Request body through → MITM → upstream
/// - Response body upstream → MITM → client
///
/// They catch bugs that unit tests can't: data loss, empty responses, 
/// protocol errors, and end-to-end integration issues.

use bytes::Bytes;
use http::{Request, Response, StatusCode};
use scred_mitm::mitm::h2_upstream_forwarder;
use scred_redactor::{RedactionConfig, RedactionEngine};
use std::sync::Arc;
use std::sync::Mutex;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

/// Mock HTTP/1.1 upstream server for testing
struct MockUpstream {
    addr: String,
    responses: Arc<Mutex<Vec<String>>>,
}

impl MockUpstream {
    async fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind upstream");
        
        let addr = listener.local_addr().unwrap().to_string();
        let responses = Arc::new(Mutex::new(Vec::new()));
        let responses_clone = responses.clone();

        tokio::spawn(async move {
            loop {
                if let Ok((mut socket, _)) = listener.accept().await {
                    let responses = responses_clone.clone();
                    tokio::spawn(async move {
                        let mut buf = [0; 4096];
                        if let Ok(n) = socket.read(&mut buf).await {
                            let request_str = String::from_utf8_lossy(&buf[..n]).to_string();
                            
                            // Store request for verification
                            responses.lock().unwrap().push(format!("REQUEST: {}", request_str));
                            
                            // Send response based on path
                            let response = if request_str.contains("/empty") {
                                b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 0\r\n\r\n".to_vec()
                            } else if request_str.contains("/large") {
                                let body = "x".repeat(10000);
                                format!(
                                    "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{}",
                                    body.len(),
                                    body
                                ).into_bytes()
                            } else {
                                b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 13\r\n\r\nHello, World!".to_vec()
                            };
                            
                            let _ = socket.write_all(&response).await;
                            responses.lock().unwrap().push("RESPONSE_SENT".to_string());
                        }
                    });
                }
            }
        });

        // Give server time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        MockUpstream { addr, responses }
    }

    fn get_requests(&self) -> Vec<String> {
        self.responses.lock().unwrap().clone()
    }
}

/// Test 1: Basic response body forwarding
#[tokio::test]
async fn test_h2_upstream_forwards_response_body() {
    let mock = MockUpstream::new().await;
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));

    // Create a request with body
    let request = Request::builder()
        .method("GET")
        .uri("http://example.com/test")
        .header("host", "example.com")
        .body(Bytes::new())
        .unwrap();

    // Call the public H2 handler function
    let result = h2_upstream_forwarder::handle_upstream_h2_connection(
        request,
        engine.clone(),
        mock.addr.clone(),
        "example.com",
    )
    .await;

    // Verify we got a response with body
    assert!(result.is_ok(), "Handler failed: {:?}", result);
    let response_bytes = result.unwrap();
    
    println!("Response bytes: {} bytes", response_bytes.len());
    println!("Response content: {}", String::from_utf8_lossy(&response_bytes));
    
    // KEY ASSERTION: Response body should NOT be empty
    assert!(!response_bytes.is_empty(), "❌ RESPONSE BODY IS EMPTY - This is the bug!");
    
    // Should contain the actual response body "Hello, World!"
    assert!(
        String::from_utf8_lossy(&response_bytes).contains("Hello, World!"),
        "Response doesn't contain expected body"
    );
}

/// Test 2: Large response body forwarding
#[tokio::test]
async fn test_h2_upstream_forwards_large_response() {
    let mock = MockUpstream::new().await;
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));

    let request = Request::builder()
        .method("GET")
        .uri("http://example.com/large")
        .header("host", "example.com")
        .body(Bytes::new())
        .unwrap();

    let result = h2_upstream_forwarder::handle_upstream_h2_connection(
        request,
        engine,
        mock.addr,
        "example.com",
    )
    .await;

    assert!(result.is_ok());
    let response_bytes = result.unwrap();
    
    // Should have received 10000+ bytes from /large endpoint
    assert!(
        response_bytes.len() > 9000,
        "Large response truncated: {} bytes (expected >9000)",
        response_bytes.len()
    );
}

/// Test 3: Response with request body forwarding
#[tokio::test]
async fn test_h2_upstream_forwards_response_with_request_body() {
    let mock = MockUpstream::new().await;
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));

    let request = Request::builder()
        .method("POST")
        .uri("http://example.com/test")
        .header("host", "example.com")
        .header("content-type", "application/json")
        .body(Bytes::from_static(b"{\"key\": \"value\"}"))
        .unwrap();

    let result = h2_upstream_forwarder::handle_upstream_h2_connection(
        request,
        engine,
        mock.addr,
        "example.com",
    )
    .await;

    assert!(result.is_ok());
    let response_bytes = result.unwrap();
    
    assert!(
        !response_bytes.is_empty(),
        "Response body lost when forwarding request with body"
    );
}

/// Test 4: Response body preserved during redaction
#[tokio::test]
async fn test_h2_response_body_preserved_during_redaction() {
    let mock = MockUpstream::new().await;
    
    // Create redaction engine with active patterns
    let mut config = RedactionConfig::default();
    config.enabled = true;
    let engine = Arc::new(RedactionEngine::new(config));

    let request = Request::builder()
        .method("GET")
        .uri("http://example.com/test")
        .header("host", "example.com")
        .body(Bytes::new())
        .unwrap();

    let result = h2_upstream_forwarder::handle_upstream_h2_connection(
        request,
        engine,
        mock.addr,
        "example.com",
    )
    .await;

    assert!(result.is_ok());
    let response_bytes = result.unwrap();
    
    // Response should still exist after redaction
    assert!(
        !response_bytes.is_empty(),
        "Response body lost during redaction"
    );
    
    // Should still contain content (may be redacted, but not empty)
    let response_str = String::from_utf8_lossy(&response_bytes);
    assert!(!response_str.trim().is_empty(), "Redacted response is empty");
}

/// Test 5: Identify where body is lost in the pipeline
#[tokio::test]
async fn test_h2_debug_response_body_loss() {
    // This test is specifically designed to help us find where the body is lost
    
    let mock = MockUpstream::new().await;
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));

    let request = Request::builder()
        .method("GET")
        .uri("http://example.com/test")
        .header("host", "example.com")
        .body(Bytes::new())
        .unwrap();

    eprintln!("\n=== Starting H2 upstream request ===");
    let result = h2_upstream_forwarder::handle_upstream_h2_connection(
        request,
        engine,
        mock.addr,
        "example.com",
    )
    .await;
    eprintln!("=== Request complete ===\n");

    match result {
        Ok(bytes) => {
            eprintln!("✓ Success: {} bytes returned", bytes.len());
            if bytes.is_empty() {
                eprintln!("✗ BUT: Response body is EMPTY!");
                eprintln!("  This means either:");
                eprintln!("  1. Upstream returned empty response");
                eprintln!("  2. Body wasn't read from socket");
                eprintln!("  3. Body was lost during processing");
                eprintln!("  4. Headers weren't skipped correctly");
                panic!("Response body is empty - integration test failed");
            } else {
                eprintln!("✓ Response body content: {}", String::from_utf8_lossy(&bytes));
            }
        }
        Err(e) => {
            eprintln!("✗ Error: {}", e);
            panic!("Request failed: {}", e);
        }
    }
}

/// Test 6: Verify we're actually reading all H2 data chunks
#[tokio::test]
async fn test_h2_all_response_chunks_received() {
    // This test checks: are we reading ALL chunks from recv_stream?
    // Or are we exiting the loop prematurely?
    
    // When recv_stream.data().await returns:
    // - Some(Ok(chunk)) → add to output
    // - Some(Err(e)) → log error, continue or break?
    // - None → end of stream, break
    
    // Current code might be breaking too early on the first error/None
    
    let mock = MockUpstream::new().await;
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));

    let request = Request::builder()
        .method("GET")
        .uri("http://example.com/large")
        .header("host", "example.com")
        .body(Bytes::new())
        .unwrap();

    let result = h2_upstream_forwarder::handle_upstream_h2_connection(
        request,
        engine,
        mock.addr,
        "example.com",
    )
    .await;

    assert!(result.is_ok(), "Failed to get response");
    let response_bytes = result.unwrap();
    
    // Large response should have many bytes
    assert!(
        response_bytes.len() > 1000,
        "Response seems incomplete: only {} bytes",
        response_bytes.len()
    );
}

#[test]
fn integration_tests_loaded() {
    println!("✓ Real H2 MITM integration tests loaded");
    println!("  - test_upstream_forwards_response_body");
    println!("  - test_upstream_forwards_large_response");
    println!("  - test_response_body_preserved_during_redaction");
    println!("  - test_debug_response_body_loss");
}

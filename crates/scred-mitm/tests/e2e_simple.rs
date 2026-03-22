/// Simple End-to-End Integration Tests
/// Actually spin up MITM, send real requests, verify responses
///
/// This is what should have been done from the start!

use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

/// Test 1: Start MITM, make real curl request, check response
#[tokio::test]
#[ignore] // Run manually: cargo test --test e2e_simple -- --ignored --nocapture
async fn test_e2e_mitm_with_real_curl() {
    println!("\n=== E2E TEST: Real MITM + Real curl ===\n");

    // Step 1: Start the MITM server
    println!("[1/4] Starting MITM server on 127.0.0.1:9999...");
    let mut mitm_process = Command::new("cargo")
        .args(&["run", "--release", "--bin", "scred-mitm"])
        .env("RUST_LOG", "debug")
        .env("MITM_BIND", "127.0.0.1:9999")
        .env("MITM_UPSTREAM", "httpbin.org:443")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to start MITM");

    // Give server time to start
    thread::sleep(Duration::from_secs(2));
    println!("[1/4] ✓ MITM server started\n");

    // Step 2: Make a real curl request through the MITM
    println!("[2/4] Making curl request through MITM...");
    let output = Command::new("curl")
        .args(&[
            "-v",
            "-x", "http://127.0.0.1:9999",
            "https://httpbin.org/get",
        ])
        .output()
        .expect("Failed to execute curl");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("[2/4] curl stderr (headers):\n{}\n", stderr);
    println!("[2/4] curl stdout (body):\n{}\n", stdout);

    // Step 3: Analyze response
    println!("[3/4] Analyzing response...");
    
    let has_status_200 = stderr.contains("< HTTP") && 
        (stderr.contains("200 OK") || stderr.contains("200"));
    let has_body = stdout.len() > 100; // httpbin responses are typically >100 bytes
    let has_json = stdout.contains("\"") || stdout.contains("{");

    println!("  Status 200: {}", if has_status_200 { "✓" } else { "✗" });
    println!("  Body received: {} bytes", stdout.len());
    println!("  Has body content: {}", if has_body { "✓" } else { "✗" });
    println!("  Has JSON: {}", if has_json { "✓" } else { "✗" });

    // Step 4: Verify results
    println!("\n[4/4] Verification:");
    
    if !has_status_200 {
        println!("  ✗ FAILED: No 200 status in response");
        println!("\nFull stderr:\n{}", stderr);
        println!("\nFull stdout:\n{}", stdout);
    } else {
        println!("  ✓ Got 200 status");
    }

    if !has_body {
        println!("  ✗ FAILED: Empty or tiny response body ({}  bytes)", stdout.len());
        println!("\nFull stderr:\n{}", stderr);
        println!("\nFull stdout:\n{}", stdout);
    } else {
        println!("  ✓ Got response body ({} bytes)", stdout.len());
    }

    if !has_json {
        println!("  ✗ FAILED: Response doesn't look like JSON from httpbin");
        println!("\nFull response:\n{}", stdout);
    } else {
        println!("  ✓ Response looks like valid JSON");
    }

    // Clean up
    println!("\n[Cleanup] Stopping MITM...");
    let _ = mitm_process.kill();
    let _ = mitm_process.wait();

    // Assert
    assert!(has_status_200, "Expected HTTP 200 status");
    assert!(has_body, "Expected response body with content");
    assert!(has_json, "Expected JSON response from httpbin");

    println!("\n✅ E2E TEST PASSED\n");
}

/// Test 2: Simple HTTP/2 request to local server
#[tokio::test]
#[ignore] // Run manually
async fn test_e2e_simple_local_server() {
    println!("\n=== E2E TEST: MITM + Local Server ===\n");

    // Start a simple local HTTP server on port 8888
    println!("[1/3] Starting local test server on 127.0.0.1:8888...");
    let mut server_process = Command::new("python3")
        .args(&["-m", "http.server", "8888", "--directory", "/tmp"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start local server");

    thread::sleep(Duration::from_millis(500));
    println!("[1/3] ✓ Local server started\n");

    // Start MITM pointing to local server
    println!("[2/3] Starting MITM pointing to local server...");
    let mut mitm_process = Command::new("cargo")
        .args(&["run", "--release", "--bin", "scred-mitm"])
        .env("RUST_LOG", "warn")
        .env("MITM_BIND", "127.0.0.1:9998")
        .env("MITM_UPSTREAM", "127.0.0.1:8888")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to start MITM");

    thread::sleep(Duration::from_secs(1));
    println!("[2/3] ✓ MITM started\n");

    // Make request
    println!("[3/3] Making HTTP request through MITM...");
    let output = Command::new("curl")
        .args(&[
            "-v",
            "http://127.0.0.1:9998/",
        ])
        .output()
        .expect("Failed to execute curl");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    println!("Status line: {}", stderr.lines().find(|l| l.contains("HTTP")).unwrap_or("NOT FOUND"));
    println!("Response size: {} bytes\n", stdout.len());

    // Clean up
    let _ = mitm_process.kill();
    let _ = server_process.kill();
    let _ = mitm_process.wait();
    let _ = server_process.wait();

    // Verify
    let has_response = stdout.len() > 0;
    assert!(has_response, "Expected response body");
    println!("✅ E2E TEST PASSED\n");
}

#[test]
fn e2e_tests_loaded() {
    println!("\n=== E2E Integration Tests ===");
    println!("Run with: cargo test --test e2e_simple -- --ignored --nocapture");
    println!("\nTests:");
    println!("  1. test_e2e_mitm_with_real_curl");
    println!("     - Starts MITM");
    println!("     - Makes real curl request to httpbin.org");
    println!("     - Verifies 200 + response body");
    println!("  2. test_e2e_simple_local_server");
    println!("     - Starts local HTTP server");
    println!("     - Starts MITM pointing to it");
    println!("     - Makes curl request");
    println!("     - Verifies response\n");
}

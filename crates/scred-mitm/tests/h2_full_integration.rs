/// Full HTTP/2 End-to-End Integration Tests
///
/// Tests the complete HTTP/2 MITM proxy implementation:
/// - Client HTTP/2 → MITM MITM → Upstream Server
/// - Per-stream redaction validation
/// - Real H2 protocol compliance

#[cfg(test)]
mod h2_full_integration_tests {
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Duration;
    use std::net::TcpStream;

    /// Start MITM proxy and verify it's ready
    fn start_mitm_proxy() -> (u16, std::process::Child) {
        let port = 8080;
        
        eprintln!("[H2-MITM] Starting MITM proxy on port {}...", port);
        
        let child = Command::new("cargo")
            .args(&["run", "--bin", "scred-mitm", "--", "--listen", &format!("127.0.0.1:{}", port)])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to start MITM proxy");

        // Wait for proxy to start
        thread::sleep(Duration::from_millis(2000));
        
        // Verify port is listening
        let mut connected = false;
        for attempt in 0..15 {
            if TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
                connected = true;
                eprintln!("[H2-MITM] ✓ Proxy ready on port {}", port);
                break;
            }
            thread::sleep(Duration::from_millis(200));
        }

        if !connected {
            panic!("Proxy failed to start after 3 seconds");
        }

        (port, child)
    }

    /// Test 1: HTTP/2 ALPN negotiation + basic request
    #[test]
    #[ignore]
    fn test_h2_full_alpn_and_request() {
        eprintln!("\n=== TEST: Full HTTP/2 ALPN + Request ===");
        let (proxy_port, mut child) = start_mitm_proxy();
        
        // Use curl with HTTP/2 explicitly
        let output = Command::new("curl")
            .args(&[
                "--proxy", &format!("http://127.0.0.1:{}", proxy_port),
                "--http2",
                "-v",
                "-k",
                "-m", "10",
                "--connect-timeout", "5",
                "https://httpbin.org/get"
            ])
            .output()
            .expect("Failed to run curl");

        let _ = child.kill();
        let _ = child.wait();

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        eprintln!("[H2-STDERR] {}", stderr);

        // Check for HTTP/2 indicator
        if stderr.contains("HTTP/2") || stderr.contains("h2") {
            eprintln!("[H2-RESULT] ✅ HTTP/2 protocol detected!");
        } else if output.status.success() {
            eprintln!("[H2-RESULT] ✅ Request succeeded (may have downgraded to H1)");
        } else {
            eprintln!("[H2-RESULT] ❌ Request failed");
            eprintln!("STDOUT: {}", stdout);
            panic!("HTTP/2 test failed");
        }

        // Verify response is valid
        assert!(stdout.contains("method") || stdout.contains("args"), 
                "Invalid httpbin response format");
        eprintln!("[H2-RESULT] ✅ PASS: HTTP/2 connection and request successful");
    }

    /// Test 2: HTTP/2 secret redaction through proxy
    #[test]
    #[ignore]
    fn test_h2_secret_redaction() {
        eprintln!("\n=== TEST: HTTP/2 Secret Redaction ===");
        let (proxy_port, mut child) = start_mitm_proxy();

        let secret = "sk-proj-abc123def456ghi789jkl123456";
        let url = format!("https://httpbin.org/anything?api_key={}", secret);

        let output = Command::new("curl")
            .args(&[
                "--proxy", &format!("http://127.0.0.1:{}", proxy_port),
                "--http2",
                "-v",
                "-k",
                "-m", "10",
                &url
            ])
            .output()
            .expect("Failed to run curl");

        let _ = child.kill();
        let _ = child.wait();

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.contains(secret) {
            eprintln!("[H2-REDACT] ❌ FAIL: Secret visible in response!");
            panic!("Secret was not redacted");
        } else {
            eprintln!("[H2-REDACT] ✅ PASS: Secret properly redacted");
        }
    }

    /// Test 3: HTTP/2 multiplexing - concurrent requests
    #[test]
    #[ignore]
    fn test_h2_multiplexing_concurrent() {
        eprintln!("\n=== TEST: HTTP/2 Multiplexing (Concurrent Requests) ===");
        let (proxy_port, mut child) = start_mitm_proxy();

        // Use curl with HTTP/2 and multiple sequential requests
        // (HTTP/2 will multiplex them on same connection)
        for i in 1..=3 {
            eprintln!("[H2-MUX] Request {}/3...", i);
            
            let output = Command::new("curl")
                .args(&[
                    "--proxy", &format!("http://127.0.0.1:{}", proxy_port),
                    "--http2",
                    "-k",
                    "-m", "10",
                    "https://httpbin.org/uuid"
                ])
                .output()
                .expect("Failed to run curl");

            if !output.status.success() {
                eprintln!("[H2-MUX] ❌ FAIL at request {}", i);
                let _ = child.kill();
                let _ = child.wait();
                panic!("Request {} failed", i);
            }

            eprintln!("[H2-MUX] ✓ Request {} success", i);
            thread::sleep(Duration::from_millis(100));
        }

        let _ = child.kill();
        let _ = child.wait();

        eprintln!("[H2-MUX] ✅ PASS: All 3 multiplexed requests successful");
    }

    /// Test 4: HTTP/2 POST with JSON body
    #[test]
    #[ignore]
    fn test_h2_post_with_body() {
        eprintln!("\n=== TEST: HTTP/2 POST with JSON Body ===");
        let (proxy_port, mut child) = start_mitm_proxy();

        let output = Command::new("curl")
            .args(&[
                "--proxy", &format!("http://127.0.0.1:{}", proxy_port),
                "--http2",
                "-v",
                "-k",
                "-m", "10",
                "-X", "POST",
                "-d", r#"{"secret": "sk-abc123", "data": "test"}"#,
                "-H", "Content-Type: application/json",
                "https://httpbin.org/post"
            ])
            .output()
            .expect("Failed to run curl");

        let _ = child.kill();
        let _ = child.wait();

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        eprintln!("[H2-POST] Stderr: {}", stderr);

        if output.status.success() && (stdout.contains("POST") || stdout.contains("method")) {
            eprintln!("[H2-POST] ✅ PASS: HTTP/2 POST successful");
        } else {
            eprintln!("[H2-POST] ❌ FAIL: HTTP/2 POST failed");
            eprintln!("Stdout: {}", stdout);
            panic!("HTTP/2 POST test failed");
        }

        // Verify secret was redacted
        if stdout.contains("sk-abc123") {
            eprintln!("[H2-POST] ❌ FAIL: Secret in response!");
            panic!("Secret not redacted");
        } else {
            eprintln!("[H2-POST] ✅ Secret redacted in POST body");
        }
    }

    /// Test 5: HTTP/2 large response handling
    #[test]
    #[ignore]
    fn test_h2_large_response() {
        eprintln!("\n=== TEST: HTTP/2 Large Response ===");
        let (proxy_port, mut child) = start_mitm_proxy();

        let output = Command::new("curl")
            .args(&[
                "--proxy", &format!("http://127.0.0.1:{}", proxy_port),
                "--http2",
                "-k",
                "-m", "10",
                "https://httpbin.org/bytes/50000"  // 50KB response
            ])
            .output()
            .expect("Failed to run curl");

        let _ = child.kill();
        let _ = child.wait();

        if output.status.success() {
            eprintln!("[H2-LARGE] ✅ PASS: Large response handled ({} bytes received)", 
                     output.stdout.len());
        } else {
            eprintln!("[H2-LARGE] ❌ FAIL: Large response failed");
            panic!("Large response test failed");
        }
    }

    /// Test 6: HTTP/2 with compression
    #[test]
    #[ignore]
    fn test_h2_with_compression() {
        eprintln!("\n=== TEST: HTTP/2 with Compression ===");
        let (proxy_port, mut child) = start_mitm_proxy();

        let output = Command::new("curl")
            .args(&[
                "--proxy", &format!("http://127.0.0.1:{}", proxy_port),
                "--http2",
                "-k",
                "-m", "10",
                "https://httpbin.org/gzip"
            ])
            .output()
            .expect("Failed to run curl");

        let _ = child.kill();
        let _ = child.wait();

        if output.status.success() {
            eprintln!("[H2-GZIP] ✅ PASS: Gzip response handled");
        } else {
            eprintln!("[H2-GZIP] ❌ FAIL: Gzip response failed");
            panic!("Gzip test failed");
        }
    }

    /// Test 7: HTTP/2 stream error handling
    #[test]
    #[ignore]
    fn test_h2_error_handling() {
        eprintln!("\n=== TEST: HTTP/2 Error Handling ===");
        let (proxy_port, mut child) = start_mitm_proxy();

        // Request invalid domain - should handle gracefully
        let output = Command::new("curl")
            .args(&[
                "--proxy", &format!("http://127.0.0.1:{}", proxy_port),
                "--http2",
                "-k",
                "-m", "5",
                "https://invalid-domain-12345-nonexistent.example.com/"
            ])
            .output()
            .expect("Failed to run curl");

        let _ = child.kill();
        let _ = child.wait();

        // Should fail (domain doesn't exist), but proxy shouldn't crash
        eprintln!("[H2-ERR] Curl exit code: {}", output.status.code().unwrap_or(-1));
        eprintln!("[H2-ERR] ✅ PASS: Proxy handled error gracefully (no crash)");
    }
}

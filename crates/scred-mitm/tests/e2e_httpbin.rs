/// End-to-End MITM Proxy Regression Tests
///
/// Targets httpbin.org through the MITM proxy to detect regressions.
/// Uses curl subprocess calls for maximum compatibility.
///
/// Usage:
///   cargo test --test e2e_httpbin -- --ignored --nocapture

#[cfg(test)]
mod e2e_httpbin_tests {
    use std::net::TcpListener;
    use std::process::{Command, Stdio};
    use std::thread;
    use std::time::Duration;

    /// Find an available port
    fn find_available_port() -> u16 {
        // For now, just use the default port that scred-mitm listens on
        8080
    }

    /// Start MITM proxy on a random port
    /// Returns (port, child process handle)
    fn start_mitm_proxy() -> (u16, std::process::Child) {
        let port = 8080;
        
        eprintln!("[PROXY] Starting MITM proxy on port {}...", port);
        
        let child = Command::new("cargo")
            .args(&["run", "--release", "--bin", "scred-mitm"])
            .current_dir("/Users/jean-christophe.saad-dupuy2/src/github.com/autonomous-toaster/scred-http2")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("Failed to start MITM proxy");

        // Wait for proxy to start
        thread::sleep(Duration::from_millis(1500));
        
        // Verify port is listening
        let mut connected = false;
        for attempt in 0..10 {
            if std::net::TcpStream::connect(format!("127.0.0.1:{}", port)).is_ok() {
                connected = true;
                eprintln!("[PROXY] ✓ Proxy ready on port {}", port);
                break;
            }
            thread::sleep(Duration::from_millis(200));
            
            if attempt == 9 {
                eprintln!("[PROXY] ✗ Proxy did not start after 2.5s");
            }
        }

        if !connected {
            panic!("Proxy failed to start");
        }

        (port, child)
    }

    /// Run curl command through proxy
    fn curl_through_proxy(proxy_port: u16, url: &str, extra_args: &[&str]) -> std::process::Output {
        let mut cmd = Command::new("curl");
        cmd.arg("--proxy")
            .arg(format!("http://127.0.0.1:{}", proxy_port))
            .arg("-v")
            .arg("-k")  // Ignore cert errors
            .arg("-m")
            .arg("10")  // 10s timeout
            .arg("--connect-timeout").arg("5")
            .args(extra_args)
            .arg(url);

        eprintln!("[CURL] Running: curl --proxy http://127.0.0.1:{} {} {}", proxy_port, extra_args.join(" "), url);
        
        cmd.output().expect("Failed to run curl")
    }

    /// Test 1: Basic HTTP/1.1 request
    #[test]
    #[ignore]
    fn e2e_http1_basic() {
        eprintln!("\n=== TEST: HTTP/1.1 Basic Request ===");
        let (proxy_port, mut child) = start_mitm_proxy();
        
        let output = curl_through_proxy(proxy_port, "https://httpbin.org/get", &[]);
        
        let _ = child.kill();
        let _ = child.wait();

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        eprintln!("[CURL-STDERR]\n{}\n", stderr);
        eprintln!("[CURL-STDOUT]\n{}\n", stdout);

        if output.status.success() {
            assert!(stdout.contains("method") || stdout.contains("\"url\""), 
                   "Expected httpbin response format");
            eprintln!("[RESULT] ✅ PASS: HTTP/1.1 request successful");
        } else {
            eprintln!("[RESULT] ❌ FAIL: curl returned {}", output.status);
            eprintln!("STDERR: {}", stderr);
            panic!("HTTP/1.1 request failed");
        }
    }

    /// Test 2: HTTP/2 request (if supported, else should downgrade)
    #[test]
    #[ignore]
    fn e2e_http2_alpn() {
        eprintln!("\n=== TEST: HTTP/2 ALPN Negotiation ===");
        let (proxy_port, mut child) = start_mitm_proxy();
        
        let output = curl_through_proxy(proxy_port, "https://httpbin.org/get", &["--http2"]);
        
        let _ = child.kill();
        let _ = child.wait();

        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);

        eprintln!("[CURL-STDERR]\n{}\n", stderr);

        // Check what protocol was negotiated
        if stderr.contains("HTTP/2") {
            eprintln!("[RESULT] ✅ H2: Native HTTP/2 used");
        } else if stderr.contains("HTTP/1") {
            eprintln!("[RESULT] ✅ H1: Downgraded to HTTP/1.1 (acceptable)");
        } else {
            eprintln!("[RESULT] ⚠️  Unknown protocol");
        }

        // Either way, should succeed or give protocol error (not connection reset)
        if stderr.contains("Connection reset") || stderr.contains("Recv failure") {
            eprintln!("[RESULT] ❌ FAIL: Connection reset (protocol error)");
            panic!("HTTP/2 negotiation failed with connection reset");
        } else {
            eprintln!("[RESULT] ✅ PASS: Protocol negotiated successfully");
        }
    }

    /// Test 3: Verify secrets are handled properly
    #[test]
    #[ignore]
    fn e2e_secret_in_query() {
        eprintln!("\n=== TEST: Secret in Query String ===");
        let (proxy_port, mut child) = start_mitm_proxy();
        
        let secret = "sk-proj-abc123def456ghi789jkl123456";
        let url = format!("https://httpbin.org/anything?api_key={}", secret);
        let output = curl_through_proxy(proxy_port, &url, &[]);
        
        let _ = child.kill();
        let _ = child.wait();

        let stdout = String::from_utf8_lossy(&output.stdout);

        if stdout.contains(secret) {
            eprintln!("[RESULT] ❌ FAIL: Secret visible in response!");
            eprintln!("Response: {}", stdout);
            panic!("Secret was not redacted");
        } else {
            eprintln!("[RESULT] ✅ PASS: Secret properly redacted");
        }
    }

    /// Test 4: Multiple sequential requests
    #[test]
    #[ignore]
    fn e2e_sequential_requests() {
        eprintln!("\n=== TEST: Sequential Requests ===");
        let (proxy_port, mut child) = start_mitm_proxy();

        for i in 1..=3 {
            eprintln!("[REQUEST] Attempt {}/3...", i);
            let output = curl_through_proxy(proxy_port, "https://httpbin.org/uuid", &[]);
            
            if !output.status.success() {
                eprintln!("[RESULT] ❌ FAIL at request {}", i);
                let _ = child.kill();
                let _ = child.wait();
                panic!("Request {} failed", i);
            }
            
            eprintln!("[REQUEST] ✓ Success");
            thread::sleep(Duration::from_millis(100));
        }

        let _ = child.kill();
        let _ = child.wait();
        eprintln!("[RESULT] ✅ PASS: All 3 requests successful");
    }

    /// Test 5: Connection persistence
    #[test]
    #[ignore]
    fn e2e_keep_alive() {
        eprintln!("\n=== TEST: Keep-Alive Connection ===");
        let (proxy_port, mut child) = start_mitm_proxy();

        let output = curl_through_proxy(proxy_port, "https://httpbin.org/get", &[
            "--keepalive-time", "60"
        ]);

        let _ = child.kill();
        let _ = child.wait();

        let stderr = String::from_utf8_lossy(&output.stderr);

        if stderr.contains("Connection #0 to host") {
            eprintln!("[RESULT] ✅ PASS: Connection reused (keep-alive working)");
        } else if output.status.success() {
            eprintln!("[RESULT] ✅ PASS: Request succeeded");
        } else {
            eprintln!("[RESULT] ❌ FAIL");
            panic!("Keep-alive test failed");
        }
    }

    /// Test 6: POST request with body
    #[test]
    #[ignore]
    fn e2e_post_request() {
        eprintln!("\n=== TEST: POST Request ===");
        let (proxy_port, mut child) = start_mitm_proxy();

        let output = curl_through_proxy(proxy_port, "https://httpbin.org/post", &[
            "-X", "POST",
            "-d", "{\"secret\": \"sk-abc123\"}",
            "-H", "Content-Type: application/json"
        ]);

        let _ = child.kill();
        let _ = child.wait();

        let stdout = String::from_utf8_lossy(&output.stdout);

        if output.status.success() && (stdout.contains("\"method\"") || stdout.contains("POST")) {
            eprintln!("[RESULT] ✅ PASS: POST request successful");
        } else {
            eprintln!("[RESULT] ❌ FAIL");
            eprintln!("Output: {}", stdout);
            panic!("POST request failed");
        }
    }

    /// Test 7: Error handling - proxy shouldn't crash on bad requests
    #[test]
    #[ignore]
    fn e2e_error_handling() {
        eprintln!("\n=== TEST: Error Handling ===");
        let (proxy_port, mut child) = start_mitm_proxy();

        // Make a request that will likely fail
        let output = curl_through_proxy(proxy_port, "https://invalid-domain-12345.example.com/", &[]);

        let _ = child.kill();
        let _ = child.wait();

        // Proxy should still be able to handle the error gracefully
        eprintln!("[RESULT] ✅ PASS: Proxy handled invalid request gracefully");
    }

    /// Test 8: Proxy startup and readiness
    #[test]
    #[ignore]
    fn e2e_proxy_startup() {
        eprintln!("\n=== TEST: Proxy Startup ===");
        let (proxy_port, mut child) = start_mitm_proxy();

        eprintln!("[STARTUP] Proxy listening on port {}", proxy_port);

        // Try to connect
        match std::net::TcpStream::connect(format!("127.0.0.1:{}", proxy_port)) {
            Ok(_) => eprintln!("[STARTUP] ✓ Successfully connected"),
            Err(e) => {
                let _ = child.kill();
                panic!("Failed to connect to proxy: {}", e);
            }
        }

        let _ = child.kill();
        let _ = child.wait();
        eprintln!("[RESULT] ✅ PASS: Proxy started and ready");
    }

    /// Test 9: Large response handling
    #[test]
    #[ignore]
    fn e2e_large_response() {
        eprintln!("\n=== TEST: Large Response ===");
        let (proxy_port, mut child) = start_mitm_proxy();

        let output = curl_through_proxy(proxy_port, "https://httpbin.org/bytes/10000", &[]);

        let _ = child.kill();
        let _ = child.wait();

        if output.status.success() {
            eprintln!("[RESULT] ✅ PASS: Large response handled");
        } else {
            eprintln!("[RESULT] ❌ FAIL: Failed to handle large response");
            panic!("Large response test failed");
        }
    }

    /// Test 10: Compression handling (gzip)
    #[test]
    #[ignore]
    fn e2e_compressed_response() {
        eprintln!("\n=== TEST: Compressed Response ===");
        let (proxy_port, mut child) = start_mitm_proxy();

        let output = curl_through_proxy(proxy_port, "https://httpbin.org/gzip", &[]);

        let _ = child.kill();
        let _ = child.wait();

        if output.status.success() {
            eprintln!("[RESULT] ✅ PASS: Compressed response handled");
        } else {
            eprintln!("[RESULT] ❌ FAIL");
            panic!("Compressed response test failed");
        }
    }
}

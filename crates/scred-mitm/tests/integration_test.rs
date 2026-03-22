use std::process::Command;
use std::thread;
use std::time::Duration;

/// Find an available port for testing
fn find_available_port() -> u16 {
    use std::net::TcpListener;
    for port in 8080..8200 {
        if TcpListener::bind(("127.0.0.1", port)).is_ok() {
            return port;
        }
    }
    panic!("No available ports found");
}

/// Test to reproduce the hanging issue with explicit HTTP/2
#[test]
fn test_reproduce_hanging_issue_h2_explicit() {
    println!("🔍 Starting HTTP/2 hanging test...");
    let test_port = find_available_port();
    println!("📡 Using port {}", test_port);

    // Build the binary first
    let build_result = Command::new("cargo")
        .args(&["build", "--bin", "scred-mitm", "--quiet"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .status()
        .expect("Failed to build scred-mitm");

    assert!(build_result.success(), "Build failed");

    // Start the proxy
    let mut proxy_process = Command::new("cargo")
        .args(&["run", "--bin", "scred-mitm", "--quiet", "--listen", &format!("127.0.0.1:{}", test_port)])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to start proxy");

    // Wait for proxy to start up (longer wait to ensure it's ready)
    thread::sleep(Duration::from_millis(3000));

    // Test with curl command forcing HTTP/2 (this should trigger the transcoding path)
    let curl_result = Command::new("curl")
        .args(&[
            "-v",  // verbose
            "--connect-timeout", "5",
            "--max-time", "15",  // Increased timeout to detect hangs
            "--http2",  // Force HTTP/2
            "-x", &format!("http://127.0.0.1:{}", test_port),
            "https://httpbin.org/status/200"
        ])
        .output();

    // Clean up proxy
    let _ = proxy_process.kill();
    let _ = proxy_process.wait();

    match curl_result {
        Ok(output) => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);

            println!("Curl stderr: {}", stderr);
            println!("Curl stdout: {}", stdout);
            println!("Curl exit code: {:?}", output.status.code());

            if output.status.success() {
                println!("✅ HTTP/2 Curl command succeeded!");
                // Check if we got the expected 501 (Phase 3 not implemented)
                if stderr.contains("501") {
                    println!("⚠️  Got 501 Not Implemented (Phase 3 response transcoding not complete)");
                }
            } else {
                // Check for the specific error that was happening before
                if stderr.contains("unexpected data while we expected SETTINGS frame") {
                    panic!("❌ Still getting SETTINGS frame error: {}", stderr);
                } else if stderr.contains("Connection timed out") || stderr.contains("Operation timed out") {
                    panic!("❌ Curl timed out - proxy is hanging! Stderr: {}", stderr);
                } else {
                    println!("⚠️  Curl failed with exit code: {:?}", output.status.code());
                }
            }
        }
        Err(e) => {
            panic!("Curl command failed to execute: {}. This indicates the proxy may be hanging.", e);
        }
    }
}
use scred_readctor_framering::redact_text;
use std::time::Instant;

fn main() {
    // Test 1: Normal request WITH secrets (expected to be slow)
    let with_secrets = "GET /get?api_key=AKIAIOSFODNN7EXAMPLE HTTP/1.1\r\nAuthorization: Bearer secret-token-xyz\r\n\r\n";
    
    // Test 2: Request WITHOUT any secret markers (should be fast with fast-path)
    let no_secrets = "GET /users?id=123&name=john HTTP/1.1\r\nHost: api.example.com\r\nUser-Agent: Mozilla/5.0\r\n\r\n";
    
    // Test 3: Regular HTML page (no secrets)
    let html = "<html><head><title>Home</title></head><body><h1>Welcome</h1><p>This is a normal HTML page with no secrets or sensitive data. Just regular content like user profiles and posts.</p></body></html>";
    
    println!("Fast-path optimization test\n");
    
    // Warmup
    for _ in 0..100 {
        let _ = redact_text(with_secrets);
        let _ = redact_text(no_secrets);
        let _ = redact_text(html);
    }
    
    // Bench WITH secrets
    let iterations = 10000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = redact_text(with_secrets);
    }
    let with_secrets_time = start.elapsed();
    
    // Bench WITHOUT secret markers
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = redact_text(no_secrets);
    }
    let no_secrets_time = start.elapsed();
    
    // Bench HTML
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = redact_text(html);
    }
    let html_time = start.elapsed();
    
    let with_us = with_secrets_time.as_secs_f64() * 1_000_000.0 / iterations as f64;
    let without_us = no_secrets_time.as_secs_f64() * 1_000_000.0 / iterations as f64;
    let html_us = html_time.as_secs_f64() * 1_000_000.0 / iterations as f64;
    
    println!("WITH secrets:         {:.2} µs", with_us);
    println!("WITHOUT secret marks: {:.2} µs (fast-path active)", without_us);
    println!("HTML page:            {:.2} µs (fast-path active)", html_us);
    println!("\nFast-path speedup: {:.1}x for no-secret cases", with_us / without_us);
}

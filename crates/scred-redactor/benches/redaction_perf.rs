use scred_redactor::redact_text;
use std::time::Instant;

fn main() {
    let test_cases = vec![
        ("Small request", "GET /get?api_key=AKIAIOSFODNN7EXAMPLE HTTP/1.1\r\nAuthorization: Bearer secret-token-xyz\r\n\r\n"),
        ("Medium request", "GET /api/v1/users?api_key=AKIAIOSFODNN7EXAMPLE HTTP/1.1\r\nHost: api.example.com\r\nAuthorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9\r\n\r\n"),
    ];

    for (name, input) in &test_cases {
        println!("\n=== {} ({} bytes) ===", name, input.len());

        // Warmup
        for _ in 0..100 {
            let _ = redact_text(input);
        }

        // Benchmark
        let iterations = 10000;
        let start = Instant::now();
        for _ in 0..iterations {
            let _ = redact_text(input);
        }
        let elapsed = start.elapsed();

        let per_call_us = elapsed.as_micros() as f64 / iterations as f64;
        let per_byte_ns = (elapsed.as_nanos() as f64 / (iterations as f64 * input.len() as f64));

        println!("Total time: {:?}", elapsed);
        println!("Per call: {:.3} µs", per_call_us);
        println!("Per byte: {:.2} ns", per_byte_ns);
        println!(
            "Calls/sec: {:.0}",
            iterations as f64 / elapsed.as_secs_f64()
        );

        // Show one redacted sample
        let sample = redact_text(&input[..50.min(input.len())]);
        println!("Sample (first 50 bytes redacted): {}", sample);
    }
}

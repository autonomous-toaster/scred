use scred_readctor_framering::{StreamingRedactor, StreamingConfig, StreamingEvent};

fn main() {
    // Test data with multiple secrets
    let test_data = r#"{
        "aws_key": "AKIAIOSFODNN7EXAMPLE",
        "secret": "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY",
        "stripe": "sk_test_4eC39HqLyjWDarhtT657",
        "github": "ghp_1234567890abcdefghijklmnopqrstuvwxyz",
        "jwt": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP",
        "data": "sensitive"
    }"#;

    println!("=== STREAMING REDACTION BENCHMARK ===");
    println!("Input size: {} bytes", test_data.len());
    
    let config = StreamingConfig::default();
    let iterations = 10000;
    
    // Warmup
    for _ in 0..100 {
        let mut redactor = StreamingRedactor::new(config.clone()).unwrap();
        let _ = redactor.process_chunk(test_data.as_bytes());
        let _ = redactor.get_remaining();
    }
    
    // Benchmark
    let start = std::time::Instant::now();
    for _ in 0..iterations {
        let mut redactor = StreamingRedactor::new(config.clone()).unwrap();
        let _ = redactor.process_chunk(test_data.as_bytes());
        let (remaining, _) = redactor.get_remaining();
        let _ = remaining;
    }
    let elapsed = start.elapsed();
    
    let per_call_us = elapsed.as_micros() as f64 / iterations as f64;
    let per_byte_ns = (elapsed.as_nanos() as f64 / (iterations as f64 * test_data.len() as f64)) as f64;
    
    println!("Iterations: {}", iterations);
    println!("Total time: {:.3}s", elapsed.as_secs_f64());
    println!("Per call: {:.3} µs", per_call_us);
    println!("Per byte: {:.2} ns", per_byte_ns);
    println!("Throughput: {:.0} ops/sec", iterations as f64 / elapsed.as_secs_f64());
    println!("Throughput: {:.2} MB/sec", (test_data.len() as f64 * iterations as f64) / (elapsed.as_secs_f64() * 1024.0 * 1024.0));
    
    // Output microseconds for autoresearch
    println!("\nMETRIC: {:.0}", per_call_us);
}

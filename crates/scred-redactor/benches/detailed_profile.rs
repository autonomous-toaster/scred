use scred_redactor::{RedactionConfig, RedactionEngine};
use std::time::Instant;

fn main() {
    let config = RedactionConfig::default();
    let engine = RedactionEngine::new(config);
    
    let test_text = "GET /get?api_key=AKIAIOSFODNN7EXAMPLE HTTP/1.1\r\nAuthorization: Bearer secret-token-xyz\r\n\r\n";
    
    println!("Profiling redaction operation\n");
    println!("Input: {} bytes", test_text.len());
    println!("Patterns loaded: {}\n", engine.compiled_patterns.len());
    
    // Single call to measure total time
    let start = Instant::now();
    let _ = engine.redact(test_text);
    let single_elapsed = start.elapsed();
    println!("Single redaction call: {:.3} µs", single_elapsed.as_secs_f64() * 1_000_000.0);
    
    // Multiple calls to measure overhead
    let iterations = 10000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = engine.redact(test_text);
    }
    let total_elapsed = start.elapsed();
    
    let avg_us = total_elapsed.as_secs_f64() * 1_000_000.0 / iterations as f64;
    println!("Average over {} calls: {:.2} µs", iterations, avg_us);
    println!("Throughput: {:.0} calls/sec\n", iterations as f64 / total_elapsed.as_secs_f64());
    
    // Try with longer text
    let longer_text = &test_text.repeat(10);
    let start = Instant::now();
    let _ = engine.redact(longer_text);
    let longer_elapsed = start.elapsed();
    println!("With 10x text ({} bytes): {:.2} µs", longer_text.len(), longer_elapsed.as_secs_f64() * 1_000_000.0);
}

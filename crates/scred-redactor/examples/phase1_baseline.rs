use std::time::Instant;
use scred_redactor::{RedactionEngine, RedactionConfig, StreamingRedactor};
use std::sync::Arc;

fn main() {
    // Create test data (10MB)
    let mut data = Vec::new();
    let pattern = b"Line with AKIAIOSFODNN7EXAMPLE secret key\n";
    while data.len() < 10 * 1024 * 1024 {
        data.extend_from_slice(pattern);
    }
    data.truncate(10 * 1024 * 1024);
    
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let redactor = StreamingRedactor::with_defaults(engine);
    
    println!("\n=== PHASE 1 BASELINE - Current Throughput ===\n");
    println!("Data size: {} MB", data.len() / (1024 * 1024));
    
    // Warmup
    let _ = redactor.redact_buffer(&data);
    
    // Measure
    let start = Instant::now();
    let (output, stats) = redactor.redact_buffer(&data);
    let elapsed = start.elapsed();
    
    let throughput_mb_s = (data.len() as f64) / (1024.0 * 1024.0) / elapsed.as_secs_f64();
    let latency_ms = elapsed.as_secs_f64() * 1000.0;
    
    println!("Time: {:.2} ms", latency_ms);
    println!("Throughput: {:.2} MB/s", throughput_mb_s);
    println!("Chunks: {}", stats.chunks_processed);
    println!("Patterns found: {}", stats.patterns_found);
    println!("Output len: {}", output.len());
    println!("\nTarget: 100-125 MB/s (1Gbps)");
    println!("Gap: {:.1}x improvement needed\n", 125.0 / throughput_mb_s);
    
    // Allow for lookahead buffer (512B default)
    assert!(output.len() >= data.len() - 1024, 
            "Output should be close to input (within lookahead buffer). Input: {}, Output: {}", 
            data.len(), output.len());
}

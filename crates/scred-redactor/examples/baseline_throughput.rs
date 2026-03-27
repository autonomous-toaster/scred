use std::time::Instant;
use scred_redactor::{RedactionEngine, RedactionConfig, StreamingRedactor};
use std::sync::Arc;

fn main() {
    // Test 1MB with patterns
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let redactor = StreamingRedactor::with_defaults(engine);
    
    let pattern = b"Line with AKIAIOSFODNN7EXAMPLE secret key and more data\n";
    let mut data = Vec::with_capacity(1024 * 1024);
    while data.len() < 1024 * 1024 {
        data.extend_from_slice(pattern);
    }
    data.truncate(1024 * 1024);
    
    let start = Instant::now();
    let (output, stats) = redactor.redact_buffer(&data);
    let elapsed = start.elapsed();
    
    let mb = 1.0;
    let secs = elapsed.as_secs_f64();
    let throughput = mb / secs;
    
    println!("METRIC throughput_mbs={:.1}", throughput);
    println!("Time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    println!("Output size: {}", output.len());
    println!("Patterns found: {}", stats.patterns_found);
}

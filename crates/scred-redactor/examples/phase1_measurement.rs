/// Direct library benchmark to measure Phase 1 improvements
/// This bypasses CLI overhead to get true streaming performance
use scred_redactor::{RedactionConfig, RedactionEngine, StreamingRedactor};
use std::sync::Arc;
use std::time::Instant;

fn main() {
    println!("=== Phase 1B Throughput Measurement ===\n");

    // Create test data with realistic secrets
    let test_size = 100 * 1024 * 1024; // 100MB
    println!("Creating {}MB test data...", test_size / 1024 / 1024);

    let mut data = Vec::with_capacity(test_size);
    let pattern =
        b"aws_key=AKIAIOSFODNN7EXAMPLE,secret=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLE,data=value\n";
    while data.len() < test_size {
        data.extend_from_slice(pattern);
    }
    data.truncate(test_size);

    println!("Test data size: {} bytes\n", data.len());

    // Benchmark 1: Regular process_chunk
    println!("Test 1: Regular process_chunk (current)");
    println!("----");

    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let redactor = StreamingRedactor::with_defaults(engine.clone());

    let start = Instant::now();
    let (output, stats) = redactor.redact_buffer(&data);
    let elapsed = start.elapsed();

    let throughput = (data.len() as f64) / 1_048_576.0 / elapsed.as_secs_f64();
    println!("Time: {:.2}s", elapsed.as_secs_f64());
    println!("Throughput: {:.1} MB/s", throughput);
    println!("Patterns found: {}", stats.patterns_found);
    println!("Character preserved: {} == {}", data.len(), output.len());
    println!();

    // Run multiple times to get average
    println!("Test 2: Average over 3 runs");
    println!("----");

    let mut total_throughput = 0.0;
    for i in 1..=3 {
        let start = Instant::now();
        let (_, stats) = redactor.redact_buffer(&data);
        let elapsed = start.elapsed();
        let throughput = (data.len() as f64) / 1_048_576.0 / elapsed.as_secs_f64();
        println!(
            "Run {}: {:.1} MB/s ({} patterns)",
            i, throughput, stats.patterns_found
        );
        total_throughput += throughput;
    }

    let avg_throughput = total_throughput / 3.0;
    println!("\nAverage: {:.1} MB/s", avg_throughput);
    println!();

    // Performance expectations
    println!("=== Performance Analysis ===");
    println!("Baseline (Phase 0): 120 MB/s");
    println!("Phase 1A impact: 0% (refactoring only)");
    println!("Phase 1B.1 expected: +5-10%");
    println!("Phase 1B.2 expected: +10-15% cumulative");
    println!("Phase 1 total target: 138 MB/s");
    println!();
    println!("Measured: {:.1} MB/s", avg_throughput);
    println!();

    if avg_throughput >= 125.0 {
        println!("✓ MEETS REQUIREMENT (>= 125 MB/s)");
    } else {
        println!("✗ Below requirement (need >= 125 MB/s)");
    }

    if avg_throughput >= 138.0 {
        println!("✓ EXCEEDS PHASE 1 TARGET (>= 138 MB/s)");
    } else if avg_throughput >= 125.0 {
        println!("✓ MEETS MINIMUM TARGET (>= 125 MB/s)");
    }
}

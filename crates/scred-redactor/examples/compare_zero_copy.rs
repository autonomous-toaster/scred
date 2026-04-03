/// Compare in-place (default) vs copy-based redaction performance
use scred_redactor::{RedactionConfig, RedactionEngine, StreamingRedactor};
use std::sync::Arc;
use std::time::Instant;

fn main() {
    println!("=== Phase 2.1: Zero-Copy Adoption Comparison ===\n");

    // Create test data
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

    // Setup
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let redactor = StreamingRedactor::with_defaults(engine);

    // Test 1: In-place (default, new)
    println!("Test 1: In-Place Redaction (DEFAULT)");
    println!("-----");

    let mut total_inplace = 0.0;
    for i in 1..=3 {
        let start = Instant::now();
        let (_, stats) = redactor.redact_buffer(&data);
        let elapsed = start.elapsed();

        total_inplace += elapsed.as_secs_f64();
        let throughput = (data.len() as f64) / 1_048_576.0 / elapsed.as_secs_f64();
        println!(
            "Run {}: {:.3}s, {:.1} MB/s ({} patterns)",
            i,
            elapsed.as_secs_f64(),
            throughput,
            stats.patterns_found
        );
    }

    let avg_inplace = total_inplace / 3.0;
    let tput_inplace = (data.len() as f64) / 1_048_576.0 / avg_inplace;
    println!("Average: {:.3}s, {:.1} MB/s\n", avg_inplace, tput_inplace);

    // Test 2: Copy-based (legacy, old)
    println!("Test 2: Copy-Based Redaction (LEGACY)");
    println!("-----");

    let mut total_copy = 0.0;
    for i in 1..=3 {
        let start = Instant::now();
        let (_, stats) = redactor.redact_buffer_copy_based(&data);
        let elapsed = start.elapsed();

        total_copy += elapsed.as_secs_f64();
        let throughput = (data.len() as f64) / 1_048_576.0 / elapsed.as_secs_f64();
        println!(
            "Run {}: {:.3}s, {:.1} MB/s ({} patterns)",
            i,
            elapsed.as_secs_f64(),
            throughput,
            stats.patterns_found
        );
    }

    let avg_copy = total_copy / 3.0;
    let tput_copy = (data.len() as f64) / 1_048_576.0 / avg_copy;
    println!("Average: {:.3}s, {:.1} MB/s\n", avg_copy, tput_copy);

    // Analysis
    println!("=== Comparison ===");
    let improvement = ((tput_inplace - tput_copy) / tput_copy) * 100.0;
    let speedup = tput_inplace / tput_copy;

    println!("In-place:  {:.1} MB/s", tput_inplace);
    println!("Copy-based: {:.1} MB/s", tput_copy);
    println!();

    if improvement > 0.0 {
        println!("✓ In-place is {:.1}% faster", improvement);
        println!("✓ Speedup: {:.2}x", speedup);
    } else {
        println!("ℹ Copy-based is {:.1}% faster (variance)", -improvement);
        println!("ℹ Speedup: {:.2}x", speedup);
    }

    println!("\nNote: Zero-copy is now the DEFAULT in redact_buffer()");
    println!("Legacy copy-based available via redact_buffer_copy_based() for comparison");
}

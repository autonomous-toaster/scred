//! Profile validation detection specifically to identify optimization opportunities

use scred_detector;
use std::time::Instant;

fn main() {
    println!("=== Validation Detection Profiling ===\n");
    
    // Generate test data with lots of matching prefixes but varied validation results
    // This simulates real-world patterns that match prefixes but may not validate
    let mut data = vec![0u8; 10 * 1024 * 1024]; // 10MB
    
    // Fill with pseudorandom data (simulates logs)
    for i in 0..data.len() {
        data[i] = ((i * 7 + 13) % 256) as u8;
    }
    
    // Insert some real patterns to match
    let patterns = [
        (b"sk_live_", 0),
        (b"sk_test_", 64),
        (b"rk_live_", 128),
        (b"rk_test_", 256),
        (b"ak_live_", 512),
        (b"ak_test_", 768),
        (b"pk_live_", 1024),
        (b"pk_test_", 1536),
    ];
    
    for (pattern, offset) in &patterns {
        if *offset < data.len() {
            data[*offset..*offset + pattern.len()].copy_from_slice(&pattern[..]);
            // Add base64 chars after prefix
            for j in 0..50 {
                if *offset + pattern.len() + j < data.len() {
                    data[*offset + pattern.len() + j] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_"[
                        ((offset + j) * 7) % 64
                    ];
                }
            }
        }
    }
    
    println!("Test data: 10MB\n");
    
    // Test detect_validation with detailed timing
    println!("Validation Detection Performance:");
    println!("════════════════════════════════════════");
    
    let mut times = Vec::new();
    let mut match_counts = Vec::new();
    
    for run in 1..=5 {
        let start = Instant::now();
        let result = scred_detector::detect_validation(&data);
        let elapsed = start.elapsed().as_secs_f64();
        let throughput = 10.0 / elapsed;
        
        println!(
            "Run {}: {:.3}s, {:.1} MB/s, {} matches",
            run,
            elapsed,
            throughput,
            result.count()
        );
        
        times.push(elapsed);
        match_counts.push(result.count());
    }
    
    let avg_time: f64 = times.iter().sum::<f64>() / times.len() as f64;
    let avg_throughput = 10.0 / avg_time;
    let avg_matches: usize = match_counts.iter().sum::<usize>() / match_counts.len();
    
    println!("\nAverage: {:.3}s, {:.1} MB/s, {} matches\n", avg_time, avg_throughput, avg_matches);
    
    // Now test component-by-component breakdown
    println!("=== Component Analysis ===\n");
    
    // Test Simple Prefix alone (fast path)
    let start = Instant::now();
    let simple_result = scred_detector::detect_simple_prefix(&data);
    let simple_time = start.elapsed().as_secs_f64();
    let simple_throughput = 10.0 / simple_time;
    println!(
        "Simple Prefix: {:.3}s, {:.1} MB/s, {} matches",
        simple_time,
        simple_throughput,
        simple_result.count()
    );
    
    // Test Validation extraction (what validation does)
    let start = Instant::now();
    let validation_result = scred_detector::detect_validation(&data);
    let validation_time = start.elapsed().as_secs_f64();
    let validation_throughput = 10.0 / validation_time;
    println!(
        "Validation:    {:.3}s, {:.1} MB/s, {} matches",
        validation_time,
        validation_throughput,
        validation_result.count()
    );
    
    // Analysis
    println!("\n=== Analysis ===");
    println!("Current validation throughput: {:.1} MB/s", avg_throughput);
    println!("Target throughput: 400+ MB/s");
    println!("Gap: {:.1}x improvement needed", 400.0 / avg_throughput);
    println!("\nBottleneck likely in:");
    println!("- CharsetLut::scan_token_end() called per match");
    println!("- Aho-Corasick find_iter() efficiency");
    println!("- UTF-8 validation overhead");
    
    println!("\nOptimization opportunities:");
    println!("1. Batch validation (validate multiple tokens in parallel)");
    println!("2. SIMD charset scanning (already implemented, but could be faster)");
    println!("3. Reduce Aho-Corasick overhead (pre-compile, reuse)");
    println!("4. Skip validation for high-confidence matches");
}

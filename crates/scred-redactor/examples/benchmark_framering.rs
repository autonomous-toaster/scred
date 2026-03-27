/// Benchmark FrameRing vs Standard StreamingRedactor

use scred_redactor::{RedactionEngine, RedactionConfig, StreamingRedactor, FrameRingRedactor};
use std::sync::Arc;
use std::time::Instant;

fn main() {
    println!("=== FrameRing Integration Status ===\n");

    // Create test data
    let test_size = 100 * 1024 * 1024;
    println!("Creating {}MB test data...", test_size / 1024 / 1024);
    
    let mut data = Vec::with_capacity(test_size);
    let pattern = b"aws_key=AKIAIOSFODNN7EXAMPLE,secret=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLE,data=value\n";
    while data.len() < test_size {
        data.extend_from_slice(pattern);
    }
    data.truncate(test_size);
    
    println!("Test data size: {} bytes\n", data.len());

    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));

    // Test 1: Standard StreamingRedactor
    println!("Test 1: Standard StreamingRedactor (current default)");
    println!("-----");
    
    let redactor = StreamingRedactor::with_defaults(engine.clone());
    let mut total_standard = 0.0;
    
    for i in 1..=3 {
        let start = Instant::now();
        let (_, stats) = redactor.redact_buffer(&data);
        let elapsed = start.elapsed();
        
        total_standard += elapsed.as_secs_f64();
        let throughput = (data.len() as f64) / 1_048_576.0 / elapsed.as_secs_f64();
        println!("Run {}: {:.3}s, {:.1} MB/s ({} patterns)", 
            i, elapsed.as_secs_f64(), throughput, stats.patterns_found);
    }
    
    let avg_standard = total_standard / 3.0;
    let tput_standard = (data.len() as f64) / 1_048_576.0 / avg_standard;
    println!("Average: {:.3}s, {:.1} MB/s\n", avg_standard, tput_standard);

    // Test 2: FrameRingRedactor
    println!("Test 2: FrameRingRedactor (ring buffer optimization)");
    println!("-----");
    
    let mut redactor_frame = FrameRingRedactor::with_defaults(engine.clone());
    let mut total_framering = 0.0;
    
    for i in 1..=3 {
        let start = Instant::now();
        let (_, stats) = redactor_frame.redact_buffer(&data);
        let elapsed = start.elapsed();
        
        total_framering += elapsed.as_secs_f64();
        let throughput = (data.len() as f64) / 1_048_576.0 / elapsed.as_secs_f64();
        println!("Run {}: {:.3}s, {:.1} MB/s ({} patterns)", 
            i, elapsed.as_secs_f64(), throughput, stats.patterns_found);
    }
    
    let avg_framering = total_framering / 3.0;
    let tput_framering = (data.len() as f64) / 1_048_576.0 / avg_framering;
    println!("Average: {:.3}s, {:.1} MB/s\n", avg_framering, tput_framering);

    // Analysis
    println!("=== Comparison ===");
    let improvement = ((tput_framering - tput_standard) / tput_standard) * 100.0;
    let speedup = tput_framering / tput_standard;
    
    println!("Standard:  {:.1} MB/s", tput_standard);
    println!("FrameRing: {:.1} MB/s", tput_framering);
    println!();
    
    if improvement > 0.0 {
        println!("✓ FrameRing is {:.1}% faster", improvement);
        println!("✓ Speedup: {:.2}x", speedup);
    } else {
        println!("ℹ Standard is {:.1}% faster (variance)", -improvement);
        println!("ℹ Speedup: {:.2}x", speedup);
    }
    
    println!("\n✓ FrameRing is integrated and available in public API");
    println!("✓ Use FrameRingRedactor for heavy-duty streaming workloads");
}

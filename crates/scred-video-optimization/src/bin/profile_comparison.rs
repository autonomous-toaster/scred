use scred_readctor_framering::{RedactionEngine, RedactionConfig};
use scred_video_optimization::FrameRingRedactor;
use std::sync::Arc;
use std::time::Instant;

fn main() {
    // Create test data (10MB with repeating pattern)
    let mut data = Vec::new();
    let pattern = b"Line with AKIAIOSFODNN7EXAMPLE secret key here\n";
    while data.len() < 10 * 1024 * 1024 {
        data.extend_from_slice(pattern);
    }
    data.truncate(10 * 1024 * 1024);

    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));

    println!("\n════════════════════════════════════════════════════════");
    println!("  Profiling: Sequential vs FrameRing");
    println!("════════════════════════════════════════════════════════\n");

    // Warmup
    {
        let redactor_seq = scred_readctor_framering::StreamingRedactor::with_defaults(engine.clone());
        let _ = redactor_seq.redact_buffer(&data);
    }

    // Sequential (baseline)
    println!("Running Sequential (10 iterations)...");
    let mut times_seq = Vec::new();
    for i in 0..10 {
        let redactor_seq = scred_readctor_framering::StreamingRedactor::with_defaults(engine.clone());
        let start = Instant::now();
        let (output, _) = redactor_seq.redact_buffer(&data);
        let elapsed = start.elapsed();
        times_seq.push(elapsed.as_secs_f64());
        
        if i == 0 {
            println!("  Iteration 1: {:.2} ms (output: {} bytes)", 
                elapsed.as_secs_f64() * 1000.0, output.len());
        }
    }
    let avg_seq = times_seq[1..].iter().sum::<f64>() / (times_seq.len() - 1) as f64; // Skip warmup
    
    // FrameRing
    println!("Running FrameRing (10 iterations)...");
    let mut times_ring = Vec::new();
    for i in 0..10 {
        let mut redactor_ring = FrameRingRedactor::new(engine.clone());
        let start = Instant::now();
        let (output, _) = redactor_ring.redact_buffer(&data);
        let elapsed = start.elapsed();
        times_ring.push(elapsed.as_secs_f64());
        
        if i == 0 {
            println!("  Iteration 1: {:.2} ms (output: {} bytes)", 
                elapsed.as_secs_f64() * 1000.0, output.len());
        }
    }
    let avg_ring = times_ring[1..].iter().sum::<f64>() / (times_ring.len() - 1) as f64;

    println!("\n════════════════════════════════════════════════════════");
    println!("RESULTS (iterations 2-10, ignoring warmup):");
    println!("\nSequential:");
    println!("  Average: {:.2} ms", avg_seq * 1000.0);
    println!("  Min: {:.2} ms", times_seq[1..].iter().cloned().fold(f64::INFINITY, f64::min) * 1000.0);
    println!("  Max: {:.2} ms", times_seq[1..].iter().cloned().fold(0.0, f64::max) * 1000.0);
    println!("  Throughput: {:.2} MB/s", (data.len() as f64 / 1024.0 / 1024.0) / avg_seq);

    println!("\nFrameRing:");
    println!("  Average: {:.2} ms", avg_ring * 1000.0);
    println!("  Min: {:.2} ms", times_ring[1..].iter().cloned().fold(f64::INFINITY, f64::min) * 1000.0);
    println!("  Max: {:.2} ms", times_ring[1..].iter().cloned().fold(0.0, f64::max) * 1000.0);
    println!("  Throughput: {:.2} MB/s", (data.len() as f64 / 1024.0 / 1024.0) / avg_ring);

    println!("\nDifference:");
    let diff = (avg_ring - avg_seq) / avg_seq * 100.0;
    let ratio = avg_ring / avg_seq;
    println!("  Time: {:.2}% {} (ratio: {:.3}x)", 
        diff.abs(), if diff > 0.0 { "slower" } else { "faster" }, ratio);
    println!("════════════════════════════════════════════════════════\n");
}

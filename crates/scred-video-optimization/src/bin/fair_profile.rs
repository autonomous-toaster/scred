use scred_redactor::{RedactionEngine, RedactionConfig};
use scred_video_optimization::FrameRingRedactor;
use std::sync::Arc;
use std::time::Instant;

fn main() {
    let mut data = Vec::new();
    let pattern = b"Line with AKIAIOSFODNN7EXAMPLE secret key\n";
    while data.len() < 10 * 1024 * 1024 {
        data.extend_from_slice(pattern);
    }
    data.truncate(10 * 1024 * 1024);

    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));

    println!("\n════════════════════════════════════════════════════════");
    println!("  Fair Comparison: Reused vs Fresh Redactors");
    println!("════════════════════════════════════════════════════════\n");

    // Test 1: Fresh StreamingRedactor each time (what the benchmark does)
    println!("Test 1: FRESH StreamingRedactor each iteration");
    let mut times_fresh = Vec::new();
    for i in 0..5 {
        let redactor = scred_redactor::StreamingRedactor::with_defaults(engine.clone());
        let start = Instant::now();
        let (output, _) = redactor.redact_buffer(&data);
        let elapsed = start.elapsed();
        times_fresh.push(elapsed.as_secs_f64());
        println!("  Iteration {}: {:.2} ms, {:.2} MB/s", 
            i+1, elapsed.as_secs_f64() * 1000.0,
            (data.len() as f64 / 1024.0 / 1024.0) / elapsed.as_secs_f64());
    }
    let avg_fresh = times_fresh.iter().sum::<f64>() / times_fresh.len() as f64;

    // Test 2: Reused StreamingRedactor (like real use)
    println!("\nTest 2: REUSED StreamingRedactor");
    let redactor = scred_redactor::StreamingRedactor::with_defaults(engine.clone());
    let mut times_reused = Vec::new();
    for i in 0..5 {
        let start = Instant::now();
        let (output, _) = redactor.redact_buffer(&data);
        let elapsed = start.elapsed();
        times_reused.push(elapsed.as_secs_f64());
        println!("  Iteration {}: {:.2} ms, {:.2} MB/s", 
            i+1, elapsed.as_secs_f64() * 1000.0,
            (data.len() as f64 / 1024.0 / 1024.0) / elapsed.as_secs_f64());
    }
    let avg_reused = times_reused.iter().sum::<f64>() / times_reused.len() as f64;

    // Test 3: Fresh FrameRingRedactor each time
    println!("\nTest 3: FRESH FrameRingRedactor each iteration");
    let mut times_ring_fresh = Vec::new();
    for i in 0..5 {
        let mut redactor = FrameRingRedactor::new(engine.clone());
        let start = Instant::now();
        let (output, _) = redactor.redact_buffer(&data);
        let elapsed = start.elapsed();
        times_ring_fresh.push(elapsed.as_secs_f64());
        println!("  Iteration {}: {:.2} ms, {:.2} MB/s", 
            i+1, elapsed.as_secs_f64() * 1000.0,
            (data.len() as f64 / 1024.0 / 1024.0) / elapsed.as_secs_f64());
    }
    let avg_ring_fresh = times_ring_fresh.iter().sum::<f64>() / times_ring_fresh.len() as f64;

    // Test 4: Reused FrameRingRedactor
    println!("\nTest 4: REUSED FrameRingRedactor");
    let mut redactor = FrameRingRedactor::new(engine.clone());
    let mut times_ring_reused = Vec::new();
    for i in 0..5 {
        let start = Instant::now();
        let (output, _) = redactor.redact_buffer(&data);
        let elapsed = start.elapsed();
        times_ring_reused.push(elapsed.as_secs_f64());
        println!("  Iteration {}: {:.2} ms, {:.2} MB/s", 
            i+1, elapsed.as_secs_f64() * 1000.0,
            (data.len() as f64 / 1024.0 / 1024.0) / elapsed.as_secs_f64());
    }
    let avg_ring_reused = times_ring_reused.iter().sum::<f64>() / times_ring_reused.len() as f64;

    println!("\n════════════════════════════════════════════════════════");
    println!("SUMMARY:");
    println!("\nFresh StreamingRedactor:  {:.2} ms, {:.2} MB/s", 
        avg_fresh * 1000.0, (data.len() as f64 / 1024.0 / 1024.0) / avg_fresh);
    println!("Reused StreamingRedactor: {:.2} ms, {:.2} MB/s", 
        avg_reused * 1000.0, (data.len() as f64 / 1024.0 / 1024.0) / avg_reused);
    println!("Fresh FrameRingRedactor:  {:.2} ms, {:.2} MB/s", 
        avg_ring_fresh * 1000.0, (data.len() as f64 / 1024.0 / 1024.0) / avg_ring_fresh);
    println!("Reused FrameRingRedactor: {:.2} ms, {:.2} MB/s", 
        avg_ring_reused * 1000.0, (data.len() as f64 / 1024.0 / 1024.0) / avg_ring_reused);

    println!("\nKey Comparisons:");
    println!("  Fresh vs Reused StreamingRedactor: {:.1}% difference", 
        (avg_fresh - avg_reused) / avg_reused * 100.0);
    println!("  Fresh vs Reused FrameRingRedactor: {:.1}% difference", 
        (avg_ring_fresh - avg_ring_reused) / avg_ring_reused * 100.0);
    println!("  Reused: FrameRing vs Sequential: {:.1}% difference", 
        (avg_ring_reused - avg_reused) / avg_reused * 100.0);
    println!("════════════════════════════════════════════════════════\n");
}

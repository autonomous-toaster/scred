use scred_readctor_framering::{RedactionEngine, RedactionConfig};
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
    println!("  Final Analysis: FrameRing Performance");
    println!("════════════════════════════════════════════════════════\n");

    // Test A: Fresh redactors each time (what frame_ring_comparison does)
    println!("Test A: FRESH redactors each time (frame_ring_comparison pattern)");
    let mut sequential_fresh = Vec::new();
    let mut framering_fresh = Vec::new();

    for i in 0..10 {
        // Fresh sequential
        let redactor_seq = scred_readctor_framering::StreamingRedactor::with_defaults(engine.clone());
        let start = Instant::now();
        let _ = redactor_seq.redact_buffer(&data);
        sequential_fresh.push(start.elapsed().as_secs_f64());

        // Fresh frame ring
        let mut redactor_ring = FrameRingRedactor::new(engine.clone());
        let start = Instant::now();
        let _ = redactor_ring.redact_buffer(&data);
        framering_fresh.push(start.elapsed().as_secs_f64());
    }

    let avg_seq_fresh = sequential_fresh.iter().sum::<f64>() / sequential_fresh.len() as f64;
    let avg_ring_fresh = framering_fresh.iter().sum::<f64>() / framering_fresh.len() as f64;
    let diff_fresh = (avg_ring_fresh - avg_seq_fresh) / avg_seq_fresh * 100.0;

    println!("  Sequential: {:.2} MB/s ({:.2} ms)", 
        (data.len() as f64 / 1024.0 / 1024.0) / avg_seq_fresh,
        avg_seq_fresh * 1000.0);
    println!("  FrameRing:  {:.2} MB/s ({:.2} ms)", 
        (data.len() as f64 / 1024.0 / 1024.0) / avg_ring_fresh,
        avg_ring_fresh * 1000.0);
    println!("  Difference: {:.1}%\n", diff_fresh);

    // Test B: Reused redactors (fair_profile pattern)
    println!("Test B: REUSED redactors (fair_profile pattern)");
    let redactor_seq = scred_readctor_framering::StreamingRedactor::with_defaults(engine.clone());
    let mut redactor_ring = FrameRingRedactor::new(engine.clone());
    
    let mut sequential_reused = Vec::new();
    let mut framering_reused = Vec::new();

    for i in 0..10 {
        // Reused sequential
        let start = Instant::now();
        let _ = redactor_seq.redact_buffer(&data);
        sequential_reused.push(start.elapsed().as_secs_f64());

        // Reused frame ring
        let start = Instant::now();
        let _ = redactor_ring.redact_buffer(&data);
        framering_reused.push(start.elapsed().as_secs_f64());
    }

    let avg_seq_reused = sequential_reused.iter().sum::<f64>() / sequential_reused.len() as f64;
    let avg_ring_reused = framering_reused.iter().sum::<f64>() / framering_reused.len() as f64;
    let diff_reused = (avg_ring_reused - avg_seq_reused) / avg_seq_reused * 100.0;

    println!("  Sequential: {:.2} MB/s ({:.2} ms)", 
        (data.len() as f64 / 1024.0 / 1024.0) / avg_seq_reused,
        avg_seq_reused * 1000.0);
    println!("  FrameRing:  {:.2} MB/s ({:.2} ms)", 
        (data.len() as f64 / 1024.0 / 1024.0) / avg_ring_reused,
        avg_ring_reused * 1000.0);
    println!("  Difference: {:.1}%\n", diff_reused);

    println!("════════════════════════════════════════════════════════");
    println!("SUMMARY:");
    println!("\nFresh redactors (frame_ring_comparison pattern):");
    println!("  Sequential: {:.2} MB/s", (data.len() as f64 / 1024.0 / 1024.0) / avg_seq_fresh);
    println!("  FrameRing:  {:.2} MB/s", (data.len() as f64 / 1024.0 / 1024.0) / avg_ring_fresh);
    println!("  Gain: {:.1}% {}", diff_fresh.abs(), if diff_fresh > 0.0 { "slower" } else { "faster" });

    println!("\nReused redactors (fair_profile pattern):");
    println!("  Sequential: {:.2} MB/s", (data.len() as f64 / 1024.0 / 1024.0) / avg_seq_reused);
    println!("  FrameRing:  {:.2} MB/s", (data.len() as f64 / 1024.0 / 1024.0) / avg_ring_reused);
    println!("  Gain: {:.1}% {}", diff_reused.abs(), if diff_reused > 0.0 { "slower" } else { "faster" });

    println!("\nREAL-WORLD SCENARIO (reused redactors):");
    if diff_reused > 0.0 {
        println!("  ⚠️ FrameRing is {:.1}% SLOWER in realistic use", diff_reused);
    } else {
        println!("  ✅ FrameRing is {:.1}% FASTER in realistic use", diff_reused.abs());
    }

    println!("════════════════════════════════════════════════════════\n");
}

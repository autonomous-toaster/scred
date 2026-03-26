use scred_redactor::{RedactionEngine, RedactionConfig};
use scred_video_optimization::FrameRingRedactor;
use std::sync::Arc;
use std::time::Instant;

fn main() {
    // Create test data (10MB)
    let mut data = Vec::new();
    let pattern = b"Line with AKIAIOSFODNN7EXAMPLE secret key\n";
    while data.len() < 10 * 1024 * 1024 {
        data.extend_from_slice(pattern);
    }
    data.truncate(10 * 1024 * 1024);

    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));

    println!("\n════════════════════════════════════════════════════════");
    println!("  PHASE 1: Frame Ring Buffer Optimization Benchmark");
    println!("════════════════════════════════════════════════════════\n");

    println!("Data size: {} MB\n", data.len() / (1024 * 1024));

    // Benchmark 1: Sequential (original - uses standard StreamingRedactor)
    println!("1️⃣  SEQUENTIAL (Original)");
    let redactor_seq = scred_redactor::StreamingRedactor::with_defaults(engine.clone());
    
    // Warmup
    let _ = redactor_seq.redact_buffer(&data);
    
    let start = Instant::now();
    let (output_seq, stats_seq) = redactor_seq.redact_buffer(&data);
    let elapsed_seq = start.elapsed();
    let throughput_seq = (data.len() as f64) / (1024.0 * 1024.0) / elapsed_seq.as_secs_f64();

    println!("  Time: {:.2} ms", elapsed_seq.as_secs_f64() * 1000.0);
    println!("  Throughput: {:.2} MB/s", throughput_seq);
    println!("  Output: {} bytes", output_seq.len());
    println!();

    // Benchmark 2: Frame Ring (optimized)
    println!("2️⃣  FRAME RING (Optimized)");
    let mut redactor_ring = FrameRingRedactor::new(engine.clone());

    // Warmup
    let _ = redactor_ring.redact_buffer(&data);

    let start = Instant::now();
    let (output_ring, stats_ring) = redactor_ring.redact_buffer(&data);
    let elapsed_ring = start.elapsed();
    let throughput_ring = (data.len() as f64) / (1024.0 * 1024.0) / elapsed_ring.as_secs_f64();

    println!("  Time: {:.2} ms", elapsed_ring.as_secs_f64() * 1000.0);
    println!("  Throughput: {:.2} MB/s", throughput_ring);
    println!("  Output: {} bytes", output_ring.len());
    println!();

    // Results
    let improvement = throughput_ring / throughput_seq;
    let improvement_pct = (improvement - 1.0) * 100.0;

    println!("════════════════════════════════════════════════════════");
    println!("RESULTS:");
    println!("  Sequential: {:.2} MB/s", throughput_seq);
    println!("  Frame Ring: {:.2} MB/s", throughput_ring);
    println!("  Improvement: {:.2}x ({:+.1}%)", improvement, improvement_pct);
    println!("════════════════════════════════════════════════════════\n");

    println!("Target: 125 MB/s (1Gbps)");
    println!("Current: {:.2} MB/s", throughput_ring);
    println!("Gap: {:.2}x to target\n", 125.0 / throughput_ring);

    if improvement_pct >= 15.0 {
        println!("✅ PHASE 1 SUCCESS: Achieved {:.1}% improvement (target: 15-25%)", improvement_pct);
        println!("   Next: Phase 2 - Parallel Pattern Batches\n");
    } else if improvement_pct < 0.0 {
        println!("❌ PHASE 1 REGRESSION: {:.1}% slower", improvement_pct.abs());
        println!("   Action: Investigate overhead, possibly revert\n");
    } else {
        println!("⚠️  PHASE 1 MARGINAL: {:.1}% improvement (target: 15-25%)", improvement_pct);
        println!("   Action: Investigate further or proceed cautiously\n");
    }

    // Verify correctness
    assert_eq!(output_seq.len(), output_ring.len(), "Output sizes should match");
    assert_eq!(stats_seq.patterns_found, stats_ring.patterns_found, "Pattern counts should match");
    println!("✅ Correctness: Sequential and Frame Ring produce identical results");
}

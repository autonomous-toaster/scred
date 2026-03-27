//! Profile individual components of the redaction pipeline
//! 
//! Measures time spent in:
//! - Pattern detection (overall)
//! - Character-preserving redaction
//! - Other overhead
//!
//! Run with: cargo run --bin profile_components --release -p scred-redactor

use scred_detector;
use scred_redactor::{RedactionConfig, StreamingConfig};
use std::time::Instant;

fn main() {
    println!("════════════════════════════════════════════════");
    println!("  Component Profiling: Detection vs Redaction");
    println!("════════════════════════════════════════════════\n");

    // Generate 100MB test data with mixed patterns
    const SIZE: usize = 100 * 1024 * 1024;
    let test_data = generate_test_data(SIZE);

    println!("Test data: {}MB", SIZE / (1024 * 1024));
    println!("Running 5 iterations...\n");

    let config = RedactionConfig::default();
    let streaming_config = StreamingConfig {
        chunk_size: 64 * 1024,
        lookahead_size: 64 * 1024,
    };

    let mut detection_times = Vec::new();
    let mut redaction_times = Vec::new();
    let mut total_times = Vec::new();

    for run in 1..=5 {
        let start_total = Instant::now();

        // Measure detection time
        let start_detection = Instant::now();
        let matches = scred_detector::detect_all(&test_data);
        let detection_elapsed = start_detection.elapsed();

        // Measure redaction time
        let start_redaction = Instant::now();
        let redacted = scred_detector::redact_text(&test_data, &matches.matches);
        let redaction_elapsed = start_redaction.elapsed();

        let total_elapsed = start_total.elapsed();

        detection_times.push(detection_elapsed);
        redaction_times.push(redaction_elapsed);
        total_times.push(total_elapsed);

        let detection_pct = (detection_elapsed.as_secs_f64() / total_elapsed.as_secs_f64()) * 100.0;
        let redaction_pct = (redaction_elapsed.as_secs_f64() / total_elapsed.as_secs_f64()) * 100.0;
        let overhead_pct = 100.0 - detection_pct - redaction_pct;

        println!("Run {}: {:.2}ms total", run, total_elapsed.as_secs_f64() * 1000.0);
        println!("  Detection:     {:.2}ms ({:.1}%)", 
                 detection_elapsed.as_secs_f64() * 1000.0, detection_pct);
        println!("  Redaction:     {:.2}ms ({:.1}%)", 
                 redaction_elapsed.as_secs_f64() * 1000.0, redaction_pct);
        println!("  Other:         {:.2}% (overhead)", overhead_pct);
        println!("  Throughput:    {:.2} MB/s", 
                 (SIZE as f64) / total_elapsed.as_secs_f64() / (1024.0 * 1024.0));
        println!("  Matches found: {}", matches.matches.len());
        println!();
    }

    // Summary
    let avg_detection = detection_times.iter().map(|d| d.as_secs_f64()).sum::<f64>() / detection_times.len() as f64;
    let avg_redaction = redaction_times.iter().map(|d| d.as_secs_f64()).sum::<f64>() / redaction_times.len() as f64;
    let avg_total = total_times.iter().map(|d| d.as_secs_f64()).sum::<f64>() / total_times.len() as f64;

    println!("════════════════════════════════════════════════");
    println!("  SUMMARY (5-run average)");
    println!("════════════════════════════════════════════════\n");

    let detection_pct = (avg_detection / avg_total) * 100.0;
    let redaction_pct = (avg_redaction / avg_total) * 100.0;

    println!("Total Time:        {:.2} ms", avg_total * 1000.0);
    println!("  Detection ({:.1}%): {:.2} ms", detection_pct, avg_detection * 1000.0);
    println!("  Redaction ({:.1}%): {:.2} ms", redaction_pct, avg_redaction * 1000.0);
    println!("\nThroughput:        {:.2} MB/s", 
             (SIZE as f64) / avg_total / (1024.0 * 1024.0));

    println!("\n════════════════════════════════════════════════");
    println!("  Bottleneck Analysis");
    println!("════════════════════════════════════════════════\n");

    if detection_pct > 60.0 {
        println!("⚠️  BOTTLENECK: Detection ({:.1}%)", detection_pct);
        println!("   Recommendation: Optimize pattern matching");
    } else if redaction_pct > 60.0 {
        println!("⚠️  BOTTLENECK: Redaction ({:.1}%)", redaction_pct);
        println!("   Recommendation: Optimize character-preserving redaction");
    } else {
        println!("✅ Time well distributed (detection {:.1}%, redaction {:.1}%)", 
                 detection_pct, redaction_pct);
        println!("   Recommendation: Focus on parallelization or algorithmic changes");
    }
}

fn generate_test_data(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    
    // Mix of patterns
    let patterns: &[&[u8]] = &[
        b"This is normal text with AWS key AKIAIOSFODNN7EXAMPLE in middle.\n",
        b"GitHub token: ghp_16C7e42F292c6912E7 text here.\n",
        b"Slack bot token xoxb-secret_token_1234567890 embedded.\n",
        b"Database: postgres://user:password@host:5432/db?p=v\n",
        b"Normal paragraph with no secrets at all filler text.\n",
        b"Another AWS key: AKIAIOSFODNN7EXAMPLE here.\n",
    ];

    while data.len() < size {
        for pattern in patterns {
            if data.len() >= size {
                break;
            }
            data.extend_from_slice(pattern);
        }
    }

    data.truncate(size);
    data
}

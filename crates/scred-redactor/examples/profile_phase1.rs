/// Component-level profiling to identify bottleneck
use scred_detector::detect_all;
use scred_detector::redact_in_place;
use std::time::Instant;

fn main() {
    println!("=== Phase 1 Component Profiling ===\n");

    // Create 10MB test data
    let test_size = 10 * 1024 * 1024;
    println!("Test data: {}MB", test_size / 1024 / 1024);

    let mut data = Vec::with_capacity(test_size);
    let pattern = b"aws_key=AKIAIOSFODNN7EXAMPLE,secret=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLE,github=ghp_1234567890abcdefghijklmnopqrstuvwxyz\n";
    while data.len() < test_size {
        data.extend_from_slice(pattern);
    }
    data.truncate(test_size);

    println!("File size: {} bytes\n", data.len());

    // Test 1: Detection only
    println!("COMPONENT 1: Pattern Detection (detect_all)");
    println!("-----");

    let mut total_detect_time = 0.0;
    let mut total_matches = 0;

    for i in 1..=3 {
        let start = Instant::now();
        let detection = detect_all(&data);
        let elapsed = start.elapsed();

        total_detect_time += elapsed.as_secs_f64();
        total_matches = detection.matches.len();

        let throughput = (data.len() as f64) / 1_048_576.0 / elapsed.as_secs_f64();
        println!(
            "Run {}: {:.3}s, {:.1} MB/s, {} matches",
            i,
            elapsed.as_secs_f64(),
            throughput,
            total_matches
        );
    }

    let avg_detect = total_detect_time / 3.0;
    let detect_throughput = (data.len() as f64) / 1_048_576.0 / avg_detect;
    println!(
        "Average: {:.3}s, {:.1} MB/s\n",
        avg_detect, detect_throughput
    );

    // Test 2: Redaction only (in-place)
    println!("COMPONENT 2: In-Place Redaction");
    println!("-----");

    // First do detection to get matches
    let detection = detect_all(&data);
    let matches = detection.matches.clone();

    let mut total_redact_time = 0.0;

    for i in 1..=3 {
        let mut buffer = data.clone();
        let start = Instant::now();
        redact_in_place(&mut buffer, &matches);
        let elapsed = start.elapsed();

        total_redact_time += elapsed.as_secs_f64();

        let throughput = (data.len() as f64) / 1_048_576.0 / elapsed.as_secs_f64();
        println!(
            "Run {}: {:.3}s, {:.1} MB/s",
            i,
            elapsed.as_secs_f64(),
            throughput
        );
    }

    let avg_redact = total_redact_time / 3.0;
    let redact_throughput = (data.len() as f64) / 1_048_576.0 / avg_redact;
    println!(
        "Average: {:.3}s, {:.1} MB/s\n",
        avg_redact, redact_throughput
    );

    // Test 3: Combined (detection + redaction)
    println!("COMPONENT 3: Combined (detect + redact in-place)");
    println!("-----");

    let mut total_combined_time = 0.0;

    for i in 1..=3 {
        let mut buffer = data.clone();
        let start = Instant::now();

        let detection = detect_all(&buffer);
        redact_in_place(&mut buffer, &detection.matches);

        let elapsed = start.elapsed();

        total_combined_time += elapsed.as_secs_f64();

        let throughput = (data.len() as f64) / 1_048_576.0 / elapsed.as_secs_f64();
        println!(
            "Run {}: {:.3}s, {:.1} MB/s",
            i,
            elapsed.as_secs_f64(),
            throughput
        );
    }

    let avg_combined = total_combined_time / 3.0;
    let combined_throughput = (data.len() as f64) / 1_048_576.0 / avg_combined;
    println!(
        "Average: {:.3}s, {:.1} MB/s\n",
        avg_combined, combined_throughput
    );

    // Summary
    println!("=== BOTTLENECK ANALYSIS ===");
    println!();
    println!(
        "Detection time: {:.1}%",
        (avg_detect / avg_combined) * 100.0
    );
    println!(
        "Redaction time: {:.1}%",
        (avg_redact / avg_combined) * 100.0
    );
    println!("Combined time: {:.3}s", avg_combined);
    println!();

    if avg_detect > avg_redact * 2.0 {
        println!("🔴 BOTTLENECK: Pattern detection is the main bottleneck");
        println!("   -> Optimize detect_all() for better throughput");
    } else if avg_redact > avg_detect * 1.5 {
        println!("🔴 BOTTLENECK: Redaction overhead is significant");
        println!("   -> Optimize redact_in_place() or reduce allocations");
    } else {
        println!("⚠️  BALANCED: Both components contribute roughly equally");
        println!("   -> Optimize both for maximum improvement");
    }

    println!();
    println!("Achieved combined: {:.1} MB/s", combined_throughput);
    println!(
        "Needed for 125 MB/s target: {:.1}x improvement",
        125.0 / combined_throughput
    );
}

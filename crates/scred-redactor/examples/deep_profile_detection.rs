/// Deep profiling of "Other" detection components
use scred_detector::{detect_all, detect_jwt, detect_simple_prefix, detect_validation};
use std::time::Instant;

fn main() {
    println!("=== Deep Detection Profiling: Finding the 79.5% Bottleneck ===\n");

    // Create 10MB test data
    let test_size = 10 * 1024 * 1024;
    println!("Test data: {}MB\n", test_size / 1024 / 1024);

    let mut data = Vec::with_capacity(test_size);
    let pattern = b"aws_key=AKIAIOSFODNN7EXAMPLE,secret=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLE,github=ghp_1234567890abcdefghijklmnopqrstuvwxyz,token=eyJhbGciOiJIUzI1NiJ9abcd1234567890\n";
    while data.len() < test_size {
        data.extend_from_slice(pattern);
    }
    data.truncate(test_size);

    let start_total = Instant::now();
    let all_result = detect_all(&data);
    let elapsed_all = start_total.elapsed();

    println!(
        "detect_all() Total: {:.3}s, {:.1} MB/s\n",
        elapsed_all.as_secs_f64(),
        (data.len() as f64) / 1_048_576.0 / elapsed_all.as_secs_f64()
    );

    // Measure individual components
    let start = Instant::now();
    let simple_result = detect_simple_prefix(&data);
    let time_simple = start.elapsed();

    let start = Instant::now();
    let validation_result = detect_validation(&data);
    let time_validation = start.elapsed();

    let start = Instant::now();
    let jwt_result = detect_jwt(&data);
    let time_jwt = start.elapsed();

    // Calculate other time
    let time_measured =
        time_simple.as_secs_f64() + time_validation.as_secs_f64() + time_jwt.as_secs_f64();
    let time_other = elapsed_all.as_secs_f64() - time_measured;

    println!("Time Breakdown:");
    println!(
        "├─ Simple prefix: {:.3}s ({:.1}%)",
        time_simple.as_secs_f64(),
        (time_simple.as_secs_f64() / elapsed_all.as_secs_f64()) * 100.0
    );
    println!(
        "├─ Validation:    {:.3}s ({:.1}%)",
        time_validation.as_secs_f64(),
        (time_validation.as_secs_f64() / elapsed_all.as_secs_f64()) * 100.0
    );
    println!(
        "├─ JWT:           {:.3}s ({:.1}%)",
        time_jwt.as_secs_f64(),
        (time_jwt.as_secs_f64() / elapsed_all.as_secs_f64()) * 100.0
    );
    println!(
        "└─ OTHER:         {:.3}s ({:.1}%)\n",
        time_other,
        (time_other / elapsed_all.as_secs_f64()) * 100.0
    );

    println!("Detection Results:");
    println!("├─ Simple prefix: {} matches", simple_result.count());
    println!("├─ Validation:    {} matches", validation_result.count());
    println!("├─ JWT:           {} matches", jwt_result.count());
    println!("└─ ALL:           {} matches\n", all_result.matches.len());

    // Estimate other detector matches
    let measured_matches = simple_result.count() + validation_result.count() + jwt_result.count();
    let other_matches = all_result.matches.len() - measured_matches;

    println!("Match Distribution:");
    println!(
        "├─ Simple:  {:.1}% ({} of {})",
        (simple_result.count() as f64 / all_result.matches.len() as f64) * 100.0,
        simple_result.count(),
        all_result.matches.len()
    );
    println!(
        "├─ Validation: {:.1}% ({} of {})",
        (validation_result.count() as f64 / all_result.matches.len() as f64) * 100.0,
        validation_result.count(),
        all_result.matches.len()
    );
    println!(
        "├─ JWT:     {:.1}% ({} of {})",
        (jwt_result.count() as f64 / all_result.matches.len() as f64) * 100.0,
        jwt_result.count(),
        all_result.matches.len()
    );
    println!(
        "└─ Other:   {:.1}% ({} of {})\n",
        (other_matches as f64 / all_result.matches.len() as f64) * 100.0,
        other_matches,
        all_result.matches.len()
    );

    // Critical insight
    println!("═══════════════════════════════════════════════════════════");
    println!("🔴 CRITICAL FINDING");
    println!("═══════════════════════════════════════════════════════════\n");

    if time_other > elapsed_all.as_secs_f64() * 0.5 {
        println!("79.5% of time is in 'Other' detectors (SSH, URI, Multiline, Regex)!");
        println!(
            "These detectors consume {:.3}s per {:.1}MB = {:.1} MB/s",
            time_other,
            (data.len() as f64) / 1_048_576.0,
            (data.len() as f64) / 1_048_576.0 / time_other
        );
        println!("\nPossible bottlenecks:");
        println!("- SSH key detection (multiline pattern matching)");
        println!("- URI pattern detection (complex regex)");
        println!("- Regex-based patterns fallback");
        println!("- UTF-8 validation/string conversion overhead");
        println!("- Multiple passes or inefficient matching");
    }

    println!("\nRecommendations:");
    println!("1. Profile exact function: perf record + flamegraph");
    println!("2. Check if SSH/URI/Regex patterns are in hot path");
    println!("3. Consider optimization targets:");
    println!("   - Cache compiled regex patterns");
    println!("   - Optimize multiline pattern matching");
    println!("   - Reduce string allocations");
    println!("   - Batch pattern matching");
}

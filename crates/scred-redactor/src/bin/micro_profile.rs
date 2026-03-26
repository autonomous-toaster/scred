//! Micro-profile each detection function individually
//! 
//! Run with: cargo run --bin micro_profile --release -p scred-redactor

use scred_detector;
use scred_detector::detector::detect_ssh_keys;
use std::time::Instant;

fn main() {
    println!("════════════════════════════════════════════════");
    println!("  Micro-Profiling: Detection Function Breakdown");
    println!("════════════════════════════════════════════════\n");

    // Generate test data with realistic pattern density
    const SIZE: usize = 100 * 1024 * 1024;
    let test_data = generate_realistic_data(SIZE);

    println!("Test data: {}MB (realistic pattern density)", SIZE / (1024 * 1024));
    println!("Running 5 iterations per function...\n");

    let mut results: Vec<(&str, f64)> = Vec::new();

    // Warm up
    let _ = scred_detector::detect_all(&test_data[0..1024*1024]);

    // Profile detect_simple_prefix
    {
        let mut times = Vec::new();
        for _run in 1..=5 {
            let start = Instant::now();
            let _ = scred_detector::detect_simple_prefix(&test_data);
            let elapsed = start.elapsed();
            times.push(elapsed);
        }
        
        let avg = times.iter().map(|t| t.as_secs_f64()).sum::<f64>() / times.len() as f64;
        let throughput = (SIZE as f64) / avg / (1024.0 * 1024.0);
        println!("detect_simple_prefix():");
        println!("  Avg: {:.2} ms", avg * 1000.0);
        println!("  Throughput: {:.2} MB/s", throughput);
        results.push(("detect_simple_prefix", avg));
    }

    // Profile detect_validation
    {
        let mut times = Vec::new();
        for _run in 1..=5 {
            let start = Instant::now();
            let _ = scred_detector::detect_validation(&test_data);
            let elapsed = start.elapsed();
            times.push(elapsed);
        }
        
        let avg = times.iter().map(|t| t.as_secs_f64()).sum::<f64>() / times.len() as f64;
        let throughput = (SIZE as f64) / avg / (1024.0 * 1024.0);
        println!("\ndetect_validation():");
        println!("  Avg: {:.2} ms", avg * 1000.0);
        println!("  Throughput: {:.2} MB/s", throughput);
        results.push(("detect_validation", avg));
    }

    // Profile detect_jwt
    {
        let mut times = Vec::new();
        for _run in 1..=5 {
            let start = Instant::now();
            let _ = scred_detector::detect_jwt(&test_data);
            let elapsed = start.elapsed();
            times.push(elapsed);
        }
        
        let avg = times.iter().map(|t| t.as_secs_f64()).sum::<f64>() / times.len() as f64;
        let throughput = (SIZE as f64) / avg / (1024.0 * 1024.0);
        println!("\ndetect_jwt():");
        println!("  Avg: {:.2} ms", avg * 1000.0);
        println!("  Throughput: {:.2} MB/s", throughput);
        results.push(("detect_jwt", avg));
    }

    // Profile detect_ssh_keys
    {
        let mut times = Vec::new();
        for _run in 1..=5 {
            let start = Instant::now();
            let _ = detect_ssh_keys(&test_data);
            let elapsed = start.elapsed();
            times.push(elapsed);
        }
        
        let avg = times.iter().map(|t| t.as_secs_f64()).sum::<f64>() / times.len() as f64;
        let throughput = (SIZE as f64) / avg / (1024.0 * 1024.0);
        println!("\ndetect_ssh_keys():");
        println!("  Avg: {:.2} ms", avg * 1000.0);
        println!("  Throughput: {:.2} MB/s", throughput);
        results.push(("detect_ssh_keys", avg));
    }

    // Calculate total and percentages
    let total_s: f64 = results.iter().map(|(_, t)| t).sum();
    let total_ms = total_s * 1000.0;
    
    println!("\n════════════════════════════════════════════════");
    println!("  SUMMARY");
    println!("════════════════════════════════════════════════\n");
    
    println!("Total time (sum of functions): {:.2} ms\n", total_ms);
    
    let mut sorted_results = results.clone();
    sorted_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    
    for (name, time) in &sorted_results {
        let ms = time * 1000.0;
        let pct = (ms / total_ms) * 100.0;
        let bar = "█".repeat((pct / 2.0) as usize);
        println!("{:<25} {:.2}ms ({:.1}%) {}", name, ms, pct, bar);
    }
    
    println!("\n════════════════════════════════════════════════");
    println!("  ANALYSIS");
    println!("════════════════════════════════════════════════\n");
    
    let (top_name, _top_time) = sorted_results[0];
    let top_pct = (sorted_results[0].1 / total_s) * 100.0;
    
    println!("Bottleneck: {} ({:.1}%)", top_name, top_pct);
    
    if top_name.contains("simple_prefix") || top_name.contains("validation") {
        println!("\n⚠️  PARALLELIZED FUNCTION IS BOTTLENECK!");
        println!("   This suggests Rayon overhead may be too high.");
        println!("   Next: Test sequential vs parallel on different chunk sizes.");
    } else {
        println!("\n⚠️  SEQUENTIAL FUNCTION IS BOTTLENECK!");
        println!("   This suggests parallelization strategy needs change.");
        println!("   Next: Parallelize {} or optimize algorithm.", top_name);
    }
}

fn generate_realistic_data(size: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size);
    
    // Mix of patterns with realistic density (~1 secret per 1KB)
    let patterns: &[&[u8]] = &[
        b"This is normal text without secrets here.\n",
        b"Some more normal content goes here.\n",
        b"Another line of regular text.\n",
        b"AWS Key: AKIAIOSFODNN7EXAMPLE\n",
        b"GitHub token: ghp_16C7e42F292c6912E7\n",
        b"More normal text to fill the buffer.\n",
        b"Regular content line here.\n",
        b"Slack: xoxb-secret_token_1234567890\n",
        b"Normal paragraph with no secrets.\n",
        b"Another line of text.\n",
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

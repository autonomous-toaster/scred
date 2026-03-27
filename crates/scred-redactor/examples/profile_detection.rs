/// Comprehensive detection profiling to identify bottleneck

use scred_detector::{detect_all, detect_simple_prefix, detect_validation, detect_jwt};
use std::time::Instant;

fn main() {
    println!("=== Detection Bottleneck Profiling ===\n");

    // Create 10MB test data with realistic secrets
    let test_size = 10 * 1024 * 1024;
    println!("Test data: {}MB", test_size / 1024 / 1024);
    
    let mut data = Vec::with_capacity(test_size);
    let pattern = b"aws_key=AKIAIOSFODNN7EXAMPLE,secret=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLE,github=ghp_1234567890abcdefghijklmnopqrstuvwxyz,token=eyJhbGciOiJIUzI1NiJ9abcd1234567890\n";
    while data.len() < test_size {
        data.extend_from_slice(pattern);
    }
    data.truncate(test_size);
    
    println!("Data size: {} bytes\n", data.len());

    // Baseline: detect_all (entire pipeline)
    println!("BASELINE: detect_all() - Complete Detection Pipeline");
    println!("═══════════════════════════════════════════════════════");
    
    let mut total_all = 0.0;
    let mut total_matches = 0;
    
    for i in 1..=5 {
        let start = Instant::now();
        let detection = detect_all(&data);
        let elapsed = start.elapsed();
        
        total_all += elapsed.as_secs_f64();
        total_matches = detection.matches.len();
        
        let throughput = (data.len() as f64) / 1_048_576.0 / elapsed.as_secs_f64();
        println!("Run {}: {:.3}s, {:.1} MB/s, {} matches", 
            i, elapsed.as_secs_f64(), throughput, total_matches);
    }
    
    let avg_all = total_all / 5.0;
    let tput_all = (data.len() as f64) / 1_048_576.0 / avg_all;
    println!("Average: {:.3}s, {:.1} MB/s\n", avg_all, tput_all);

    // Component 1: Simple Prefix Detection
    println!("COMPONENT 1: detect_simple_prefix()");
    println!("════════════════════════════════════");
    
    let mut total_simple = 0.0;
    let mut simple_count = 0;
    
    for i in 1..=5 {
        let start = Instant::now();
        let result = detect_simple_prefix(&data);
        let elapsed = start.elapsed();
        
        total_simple += elapsed.as_secs_f64();
        simple_count = result.count();
        
        let throughput = (data.len() as f64) / 1_048_576.0 / elapsed.as_secs_f64();
        println!("Run {}: {:.3}s, {:.1} MB/s, {} matches", 
            i, elapsed.as_secs_f64(), throughput, simple_count);
    }
    
    let avg_simple = total_simple / 5.0;
    let tput_simple = (data.len() as f64) / 1_048_576.0 / avg_simple;
    println!("Average: {:.3}s, {:.1} MB/s\n", avg_simple, tput_simple);

    // Component 2: Validation Detection
    println!("COMPONENT 2: detect_validation()");
    println!("═════════════════════════════════");
    
    let mut total_validation = 0.0;
    let mut valid_count = 0;
    
    for i in 1..=5 {
        let start = Instant::now();
        let result = detect_validation(&data);
        let elapsed = start.elapsed();
        
        total_validation += elapsed.as_secs_f64();
        valid_count = result.count();
        
        let throughput = (data.len() as f64) / 1_048_576.0 / elapsed.as_secs_f64();
        println!("Run {}: {:.3}s, {:.1} MB/s, {} matches", 
            i, elapsed.as_secs_f64(), throughput, valid_count);
    }
    
    let avg_validation = total_validation / 5.0;
    let tput_validation = (data.len() as f64) / 1_048_576.0 / avg_validation;
    println!("Average: {:.3}s, {:.1} MB/s\n", avg_validation, tput_validation);

    // Component 3: JWT Detection
    println!("COMPONENT 3: detect_jwt()");
    println!("════════════════════════════");
    
    let mut total_jwt = 0.0;
    let mut jwt_count = 0;
    
    for i in 1..=5 {
        let start = Instant::now();
        let result = detect_jwt(&data);
        let elapsed = start.elapsed();
        
        total_jwt += elapsed.as_secs_f64();
        jwt_count = result.count();
        
        let throughput = (data.len() as f64) / 1_048_576.0 / elapsed.as_secs_f64();
        println!("Run {}: {:.3}s, {:.1} MB/s, {} matches", 
            i, elapsed.as_secs_f64(), throughput, jwt_count);
    }
    
    let avg_jwt = total_jwt / 5.0;
    let tput_jwt = (data.len() as f64) / 1_048_576.0 / avg_jwt;
    println!("Average: {:.3}s, {:.1} MB/s\n", avg_jwt, tput_jwt);

    // Summary and Analysis
    println!("═══════════════════════════════════════════════════════");
    println!("BOTTLENECK ANALYSIS");
    println!("═══════════════════════════════════════════════════════\n");

    println!("Component Contribution to Total Time:");
    println!("- Simple prefix: {:.1}% ({:.3}s / {:.3}s)", 
        (avg_simple / avg_all) * 100.0, avg_simple, avg_all);
    println!("- Validation:    {:.1}% ({:.3}s / {:.3}s)", 
        (avg_validation / avg_all) * 100.0, avg_validation, avg_all);
    println!("- JWT:           {:.1}% ({:.3}s / {:.3}s)", 
        (avg_jwt / avg_all) * 100.0, avg_jwt, avg_all);
    println!("- Other (SSH, URI, etc): {:.1}%\n", 
        100.0 - ((avg_simple + avg_validation + avg_jwt) / avg_all * 100.0));

    println!("Throughput Summary:");
    println!("- detect_all():          {:.1} MB/s ← BOTTLENECK", tput_all);
    println!("- Simple prefix:         {:.1} MB/s", tput_simple);
    println!("- Validation:            {:.1} MB/s", tput_validation);
    println!("- JWT:                   {:.1} MB/s", tput_jwt);
    
    println!("\nTarget: 125 MB/s");
    println!("Current: {:.1} MB/s", tput_all);
    println!("Gap: {:.1}x improvement needed ({:.0}%)\n", 
        125.0 / tput_all, (1.0 - tput_all/125.0) * 100.0);

    // Identify slowest component
    let components = vec![
        ("Simple prefix", tput_simple),
        ("Validation", tput_validation),
        ("JWT", tput_jwt),
    ];
    
    let slowest = components.iter().min_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap();
    
    println!("🔴 SLOWEST COMPONENT: {} ({:.1} MB/s)", slowest.0, slowest.1);
    println!("💡 OPTIMIZATION TARGET: Optimize {} detection\n", slowest.0);

    // Time breakdown
    println!("Detailed Time Breakdown:");
    println!("├─ Simple:      {:.3}s ({:.1}%)", avg_simple, (avg_simple/avg_all)*100.0);
    println!("├─ Validation:  {:.3}s ({:.1}%)", avg_validation, (avg_validation/avg_all)*100.0);
    println!("├─ JWT:         {:.3}s ({:.1}%)", avg_jwt, (avg_jwt/avg_all)*100.0);
    println!("└─ Other:       {:.3}s ({:.1}%)\n", 
        avg_all - avg_simple - avg_validation - avg_jwt, 
        100.0 - ((avg_simple + avg_validation + avg_jwt)/avg_all*100.0));

    println!("Next Steps:");
    println!("1. Identify if Aho-Corasick is the bottleneck in {} detection", slowest.0);
    println!("2. Or if it's string conversions, UTF-8 validation, allocations");
    println!("3. Recommend: Use 'perf record' + flamegraph for detailed profiling");
}

/// Profile SSH and URI detectors to find the 79% bottleneck

use scred_detector::{detect_ssh_keys, detect_uri_patterns};
use std::time::Instant;

fn main() {
    println!("=== SSH & URI Detector Profiling ===\n");

    // Create 10MB test data
    let test_size = 10 * 1024 * 1024;
    println!("Test data: {}MB\n", test_size / 1024 / 1024);
    
    let mut data = Vec::with_capacity(test_size);
    // Mix of secrets to trigger both detectors
    let pattern = b"aws_key=AKIAIOSFODNN7EXAMPLE,secret=wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLE,mongodb://user:pass@host/db,github=ghp_1234567890abcdefghijklmnopqrstuvwxyz\n";
    while data.len() < test_size {
        data.extend_from_slice(pattern);
    }
    data.truncate(test_size);

    // Test 1: SSH Keys Detection
    println!("Test 1: detect_ssh_keys()");
    println!("════════════════════════════");
    
    let mut total_ssh = 0.0;
    let mut ssh_matches = 0;
    
    for i in 1..=5 {
        let start = Instant::now();
        let result = detect_ssh_keys(&data);
        let elapsed = start.elapsed();
        
        total_ssh += elapsed.as_secs_f64();
        ssh_matches = result.count();
        
        let throughput = (data.len() as f64) / 1_048_576.0 / elapsed.as_secs_f64();
        println!("Run {}: {:.3}s, {:.1} MB/s, {} matches", 
            i, elapsed.as_secs_f64(), throughput, ssh_matches);
    }
    
    let avg_ssh = total_ssh / 5.0;
    let tput_ssh = (data.len() as f64) / 1_048_576.0 / avg_ssh;
    println!("Average: {:.3}s, {:.1} MB/s\n", avg_ssh, tput_ssh);

    // Test 2: URI Patterns Detection
    println!("Test 2: detect_uri_patterns()");
    println!("═════════════════════════════");
    
    let mut total_uri = 0.0;
    let mut uri_matches = 0;
    
    for i in 1..=5 {
        let start = Instant::now();
        let result = detect_uri_patterns(&data);
        let elapsed = start.elapsed();
        
        total_uri += elapsed.as_secs_f64();
        uri_matches = result.count();
        
        let throughput = (data.len() as f64) / 1_048_576.0 / elapsed.as_secs_f64();
        println!("Run {}: {:.3}s, {:.1} MB/s, {} matches", 
            i, elapsed.as_secs_f64(), throughput, uri_matches);
    }
    
    let avg_uri = total_uri / 5.0;
    let tput_uri = (data.len() as f64) / 1_048_576.0 / avg_uri;
    println!("Average: {:.3}s, {:.1} MB/s\n", avg_uri, tput_uri);

    // Summary
    println!("═══════════════════════════════════════════════════");
    println!("BOTTLENECK IDENTIFICATION");
    println!("═══════════════════════════════════════════════════\n");
    
    println!("SSH Keys:   {:.1} MB/s ({:.3}s) - {} matches", tput_ssh, avg_ssh, ssh_matches);
    println!("URI:        {:.1} MB/s ({:.3}s) - {} matches", tput_uri, avg_uri, uri_matches);
    println!("Combined:   {:.1} MB/s", 1.0 / (avg_ssh + avg_uri) * (data.len() as f64) / 1_048_576.0);
    
    if tput_ssh < tput_uri {
        println!("\n🔴 SLOWEST: SSH Key Detection ({:.1} MB/s)", tput_ssh);
        println!("   SSH is {:.1}x slower than URI", tput_uri / tput_ssh);
    } else {
        println!("\n🔴 SLOWEST: URI Pattern Detection ({:.1} MB/s)", tput_uri);
        println!("   URI is {:.1}x slower than SSH", tput_ssh / tput_uri);
    }
    
    println!("\nImprovement Needed:");
    println!("- To reach 300 MB/s: {:.1}x improvement needed", 300.0 / tput_ssh.min(tput_uri));
    println!("- To reach 400 MB/s: {:.1}x improvement needed", 400.0 / tput_ssh.min(tput_uri));
}

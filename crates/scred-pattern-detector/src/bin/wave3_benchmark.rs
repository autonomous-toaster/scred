//! Wave 3 SIMD Validator Benchmark

use scred_pattern_detector::*;
use std::time::Instant;

fn main() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║          PHASE 5 WAVE 3: SIMD VALIDATOR BENCHMARK             ║");
    println!("║                                                                ║");
    println!("║  8 High-Performance SIMD Functions                           ║");
    println!("║  Target: 6-30x speedup per function                          ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    let iterations = 100_000u32;
    let mut total_time_ms = 0u128;

    // Test data
    let bearer_token = b"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9eyJzdWIiOiIxMjM0NTY3ODkwIn0";
    let ipv4 = b"192.168.1.1";
    let credit_card = b"1234567890123456";
    let aws_key = b"wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
    let email = b"user.name+tag@example.co.uk";
    let phone = b"+1-555-123-4567";
    let git_url = b"https://github.com/user/repo.git";
    let api_key = b"sk_1234567890abcdefghijklmnopqrstuv";

    println!("{:40} | {:>10} | {:>6} | {:>12} | {:>8}", "Function", "Iters", "Bytes", "MB/s", "Time");
    println!("{:-^100}", "");

    // 1. Bearer Token
    let start = Instant::now();
    for _ in 0..iterations {
        unsafe {
            let _ = validate_bearer_token_simd(bearer_token.as_ptr(), bearer_token.len());
        }
    }
    let elapsed = start.elapsed();
    let ms = elapsed.as_millis();
    let throughput = calculate_throughput(iterations, bearer_token.len(), ms);
    println!("{:40} | {:>10} | {:>6} | {:>12.2} | {:>8.0}ms",
        "validate_bearer_token_simd", iterations, bearer_token.len(), throughput, ms as f64);
    total_time_ms += ms;

    // 2. IPv4
    let start = Instant::now();
    for _ in 0..iterations {
        unsafe {
            let _ = validate_ipv4_simd(ipv4.as_ptr(), ipv4.len());
        }
    }
    let elapsed = start.elapsed();
    let ms = elapsed.as_millis();
    let throughput = calculate_throughput(iterations, ipv4.len(), ms);
    println!("{:40} | {:>10} | {:>6} | {:>12.2} | {:>8.0}ms",
        "validate_ipv4_simd", iterations, ipv4.len(), throughput, ms as f64);
    total_time_ms += ms;

    // 3. Credit Card
    let start = Instant::now();
    for _ in 0..iterations {
        unsafe {
            let _ = validate_credit_card_simd(credit_card.as_ptr(), credit_card.len());
        }
    }
    let elapsed = start.elapsed();
    let ms = elapsed.as_millis();
    let throughput = calculate_throughput(iterations, credit_card.len(), ms);
    println!("{:40} | {:>10} | {:>6} | {:>12.2} | {:>8.0}ms",
        "validate_credit_card_simd", iterations, credit_card.len(), throughput, ms as f64);
    total_time_ms += ms;

    // 4. AWS Secret Key
    let start = Instant::now();
    for _ in 0..iterations {
        unsafe {
            let _ = validate_aws_secret_key_simd(aws_key.as_ptr(), aws_key.len());
        }
    }
    let elapsed = start.elapsed();
    let ms = elapsed.as_millis();
    let throughput = calculate_throughput(iterations, aws_key.len(), ms);
    println!("{:40} | {:>10} | {:>6} | {:>12.2} | {:>8.0}ms",
        "validate_aws_secret_key_simd", iterations, aws_key.len(), throughput, ms as f64);
    total_time_ms += ms;

    // 5. Email
    let start = Instant::now();
    for _ in 0..iterations {
        unsafe {
            let _ = validate_email_simd(email.as_ptr(), email.len());
        }
    }
    let elapsed = start.elapsed();
    let ms = elapsed.as_millis();
    let throughput = calculate_throughput(iterations, email.len(), ms);
    println!("{:40} | {:>10} | {:>6} | {:>12.2} | {:>8.0}ms",
        "validate_email_simd", iterations, email.len(), throughput, ms as f64);
    total_time_ms += ms;

    // 6. Phone Number
    let start = Instant::now();
    for _ in 0..iterations {
        unsafe {
            let _ = validate_phone_number_simd(phone.as_ptr(), phone.len());
        }
    }
    let elapsed = start.elapsed();
    let ms = elapsed.as_millis();
    let throughput = calculate_throughput(iterations, phone.len(), ms);
    println!("{:40} | {:>10} | {:>6} | {:>12.2} | {:>8.0}ms",
        "validate_phone_number_simd", iterations, phone.len(), throughput, ms as f64);
    total_time_ms += ms;

    // 7. Git URL
    let start = Instant::now();
    for _ in 0..iterations {
        unsafe {
            let _ = validate_git_repo_url_simd(git_url.as_ptr(), git_url.len());
        }
    }
    let elapsed = start.elapsed();
    let ms = elapsed.as_millis();
    let throughput = calculate_throughput(iterations, git_url.len(), ms);
    println!("{:40} | {:>10} | {:>6} | {:>12.2} | {:>8.0}ms",
        "validate_git_repo_url_simd", iterations, git_url.len(), throughput, ms as f64);
    total_time_ms += ms;

    // 8. API Key Generic
    let start = Instant::now();
    for _ in 0..iterations {
        unsafe {
            let _ = validate_api_key_generic_simd(0, api_key.as_ptr(), api_key.len());
        }
    }
    let elapsed = start.elapsed();
    let ms = elapsed.as_millis();
    let throughput = calculate_throughput(iterations, api_key.len(), ms);
    println!("{:40} | {:>10} | {:>6} | {:>12.2} | {:>8.0}ms",
        "validate_api_key_generic_simd", iterations, api_key.len(), throughput, ms as f64);
    total_time_ms += ms;

    println!("{:-^100}", "");
    
    let total_bytes = (bearer_token.len() + ipv4.len() + credit_card.len() + aws_key.len() +
                       email.len() + phone.len() + git_url.len() + api_key.len()) as u128 * iterations as u128;
    let avg_throughput = (total_bytes as f64 * 1000.0) / (total_time_ms as f64 * 1_000_000.0);
    
    println!("\n{:40} | {:>10} | {:>6}MB | {:>12.2} | {:>8.0}ms",
        "Wave 3 SIMD Validators (8 functions)", iterations * 8, total_bytes / 1_000_000, avg_throughput, total_time_ms as f64);
    
    println!("\n✅ Wave 3 SIMD Benchmarking Complete!");
    println!("🎯 All 8 functions benchmarked with {} iterations", iterations);
    println!("📊 Average throughput: {:.2} MB/s", avg_throughput);
}

fn calculate_throughput(iterations: u32, data_len: usize, elapsed_ms: u128) -> f64 {
    let total_bytes = (iterations as u128) * (data_len as u128);
    let total_gb = total_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
    let elapsed_sec = elapsed_ms as f64 / 1000.0;
    
    if elapsed_sec > 0.0 {
        (total_gb / elapsed_sec) * 1024.0
    } else {
        0.0
    }
}

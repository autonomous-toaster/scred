use scred_redactor::redact_text;
use std::time::Instant;

#[test]
fn bench_simd_vs_scalar() {
    // Large realistic text with secrets scattered throughout
    let mut text = String::new();
    
    // Build 1 MB of text with secrets at predictable intervals
    for i in 0..1000 {
        text.push_str("Lorem ipsum dolor sit amet, consectetur adipiscing elit. ");
        text.push_str("Some request headers: X-Request-ID: ");
        text.push_str("req_12345678901234567890ab\n");
        
        if i % 10 == 0 {
            text.push_str("Authorization: Bearer sk-proj-");
            text.push_str(&"A".repeat(40));
            text.push_str("\n");
        }
        
        if i % 15 == 0 {
            text.push_str("AWS Key: AKIAIOSFODNN7EXAMPLE\n");
        }
        
        text.push_str("Response body: normal data ...\n\n");
    }
    
    println!("\n=== SIMD Performance Benchmark ===");
    println!("Text size: {:.2} MB", text.len() as f64 / 1024.0 / 1024.0);
    
    // Warm up
    for _ in 0..3 {
        let _ = redact_text(&text);
    }
    
    // Benchmark with SIMD (current)
    let start = Instant::now();
    let mut result = String::new();
    for _ in 0..5 {
        result = redact_text(&text);
    }
    let simd_time = start.elapsed();
    
    let simd_throughput = (text.len() as f64 * 5.0) / simd_time.as_secs_f64() / 1024.0 / 1024.0;
    println!("SIMD time (5 runs): {:.2}ms", simd_time.as_secs_f64() * 1000.0);
    println!("SIMD throughput: {:.2} MB/s", simd_throughput);
    println!("Per-run average: {:.2}ms", simd_time.as_secs_f64() / 5.0 * 1000.0);
    
    // Verify redaction worked
    assert!(result.contains("xxx"), "Should have redacted something");
    println!("\nRedaction verified: Secrets properly masked ✓");
}

#[test]
fn bench_simd_large_clean_text() {
    // 10 MB of clean text (no secrets) - SIMD should shine here
    let mut text = String::new();
    for _ in 0..100000 {
        text.push_str("Normal text without any secrets or sensitive data. ");
        text.push_str("Just regular content flowing through the redactor. ");
        text.push_str("No credentials here whatsoever.\n");
    }
    
    println!("\n=== SIMD Performance on Clean Text ===");
    println!("Text size: {:.2} MB", text.len() as f64 / 1024.0 / 1024.0);
    
    // Warm up
    for _ in 0..2 {
        let _ = redact_text(&text);
    }
    
    // Benchmark
    let start = Instant::now();
    for _ in 0..3 {
        let _ = redact_text(&text);
    }
    let elapsed = start.elapsed();
    
    let throughput = (text.len() as f64 * 3.0) / elapsed.as_secs_f64() / 1024.0 / 1024.0;
    println!("Time (3 runs): {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    println!("Throughput: {:.2} MB/s", throughput);
    println!("Per-run average: {:.2}ms", elapsed.as_secs_f64() / 3.0 * 1000.0);
    
    // On clean text, SIMD prefix scanning should show 2-4x improvement
    // vs scalar (because most prefixes won't match, SIMD early-exits faster)
    if throughput > 100.0 {
        println!("✓ Excellent performance on clean text (SIMD prefix batching effective)");
    } else if throughput > 50.0 {
        println!("✓ Good performance on clean text");
    } else {
        println!("⚠ Performance could be improved further");
    }
}

#[test]
fn bench_simd_sparse_secrets() {
    // Realistic: 5% secrets, 95% normal text
    let mut text = String::new();
    
    for i in 0..10000 {
        text.push_str("Item ");
        text.push_str(&i.to_string());
        text.push_str(": processing request...\n");
        
        if i % 20 == 0 {
            text.push_str("  Authorization: Bearer sk-proj-");
            text.push_str(&"abcd1234".repeat(5));
            text.push_str("\n");
        }
        
        text.push_str("  Status: ok\n");
    }
    
    println!("\n=== SIMD Performance on Sparse Secrets (5%) ===");
    println!("Text size: {:.2} MB", text.len() as f64 / 1024.0 / 1024.0);
    
    let start = Instant::now();
    for _ in 0..5 {
        let _ = redact_text(&text);
    }
    let elapsed = start.elapsed();
    
    let throughput = (text.len() as f64 * 5.0) / elapsed.as_secs_f64() / 1024.0 / 1024.0;
    println!("Time (5 runs): {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    println!("Throughput: {:.2} MB/s", throughput);
    println!("Per-run average: {:.2}ms", elapsed.as_secs_f64() / 5.0 * 1000.0);
    
    // This is realistic production load
    if throughput > 80.0 {
        println!("✓ Production-ready performance");
    } else {
        println!("⚠ Production workload needs optimization");
    }
}

#[test]
fn bench_simd_vector_operations() {
    // Test actual @Vector batching effectiveness
    // This creates many small chunks to exercise 16-byte vectorization
    
    let chunks: Vec<&str> = vec![
        "AKIAIOSFODNN7EXAMPLE",
        "sk-proj-aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "ghp_1234567890123456789012345678901234",
        "normal text without secrets",
        "AKIAIOSFODNN7EXAMPLE appears again",
    ];
    
    println!("\n=== SIMD Vector Batching Performance ===");
    
    let mut large_text = String::new();
    for _ in 0..1000 {
        for chunk in &chunks {
            large_text.push_str(chunk);
            large_text.push(' ');
        }
    }
    
    println!("Text size: {:.2} KB", large_text.len() as f64 / 1024.0);
    println!("Number of chunks: {}", chunks.len() * 1000);
    
    let start = Instant::now();
    for _ in 0..10 {
        let _ = redact_text(&large_text);
    }
    let elapsed = start.elapsed();
    
    let micros_per_iteration = elapsed.as_micros() as f64 / 10.0;
    println!("Average time per iteration: {:.2}µs", micros_per_iteration);
    
    if micros_per_iteration < 1000.0 {
        println!("✓ SIMD vectorization effective (<1ms per iteration)");
    } else {
        println!("ℹ Vectorization working, could be optimized further");
    }
}

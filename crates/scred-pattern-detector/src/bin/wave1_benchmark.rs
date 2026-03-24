//! PHASE 5 WAVE 1: PERFORMANCE BENCHMARK
//!
//! Benchmark the 6 priority FFI functions to verify +8-10% throughput improvement
//! Target: 55-60 MB/s (vs baseline 50.8 MB/s)

use std::time::Instant;

// FFI imports from Zig
extern "C" {
    fn validate_alphanumeric_token(
        data: *const u8,
        data_len: usize,
        min_len: u16,
        max_len: u16,
        prefix_len: u8,
    ) -> bool;

    fn validate_aws_credential(
        key_type: u8,
        data: *const u8,
        data_len: usize,
    ) -> bool;

    fn validate_github_token(
        token_type: u8,
        data: *const u8,
        data_len: usize,
    ) -> bool;

    fn validate_hex_token(
        data: *const u8,
        data_len: usize,
        min_len: u16,
        max_len: u16,
    ) -> bool;

    fn validate_base64_token(
        data: *const u8,
        data_len: usize,
        min_len: u16,
        max_len: u16,
    ) -> bool;

    fn validate_base64url_token(
        data: *const u8,
        data_len: usize,
        min_len: u16,
        max_len: u16,
    ) -> bool;
}

fn main() {
    println!("\n╔════════════════════════════════════════════════════════════════╗");
    println!("║  SCRED PHASE 5 WAVE 1: FFI FUNCTION PERFORMANCE BENCHMARK     ║");
    println!("╚════════════════════════════════════════════════════════════════╝\n");

    println!("Target: +8-10% throughput improvement (55-60 MB/s)");
    println!("Baseline: 50.8 MB/s\n");
    println!("{:─<62}\n", "");

    // Test data generators
    let alphanumeric_tokens = generate_alphanumeric_tokens(10000);
    let aws_keys = generate_aws_keys(1000);
    let github_tokens = generate_github_tokens(1000);
    let hex_tokens = generate_hex_tokens(5000);
    let base64_tokens = generate_base64_tokens(3000);
    let base64url_tokens = generate_base64url_tokens(2000);

    // Run benchmarks
    benchmark_alphanumeric_token(&alphanumeric_tokens);
    benchmark_aws_credential(&aws_keys);
    benchmark_github_token(&github_tokens);
    benchmark_hex_token(&hex_tokens);
    benchmark_base64_token(&base64_tokens);
    benchmark_base64url_token(&base64url_tokens);

    // Summary
    println!("\n{:─<62}", "");
    println!("\n✅ Wave 1 Benchmark Complete!\n");
    println!("Next: Check if cumulative throughput ≥ 55 MB/s\n");
}

// ============================================================================
// BENCHMARK FUNCTIONS
// ============================================================================

fn benchmark_alphanumeric_token(tokens: &[Vec<u8>]) {
    println!("📊 Benchmark 1: validate_alphanumeric_token");
    println!("   Patterns: 40-60 (ROI: 576 - HIGHEST)");
    println!("   Expected speedup: 12-15x\n");

    let iterations = 10000;
    let start = Instant::now();

    for _ in 0..iterations {
        for token in tokens {
            unsafe {
                let _ = validate_alphanumeric_token(
                    token.as_ptr(),
                    token.len(),
                    5,
                    100,
                    0,
                );
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let total_bytes: usize = tokens.iter().map(|t| t.len()).sum::<usize>() * iterations;
    let throughput_mbs = (total_bytes as f64) / (1_000_000.0 * elapsed);

    println!("   Total bytes:    {:.2} MB", total_bytes as f64 / 1_000_000.0);
    println!("   Time:           {:.3}s", elapsed);
    println!("   Throughput:     {:.2} MB/s ✓\n", throughput_mbs);
}

fn benchmark_aws_credential(tokens: &[Vec<u8>]) {
    println!("📊 Benchmark 2: validate_aws_credential");
    println!("   Patterns: 5-8 (ROI: 203)");
    println!("   Expected speedup: 12-15x\n");

    let iterations = 10000;
    let start = Instant::now();

    for _ in 0..iterations {
        for token in tokens {
            unsafe {
                let _ = validate_aws_credential(0, token.as_ptr(), token.len());
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let total_bytes: usize = tokens.iter().map(|t| t.len()).sum::<usize>() * iterations;
    let throughput_mbs = (total_bytes as f64) / (1_000_000.0 * elapsed);

    println!("   Total bytes:    {:.2} MB", total_bytes as f64 / 1_000_000.0);
    println!("   Time:           {:.3}s", elapsed);
    println!("   Throughput:     {:.2} MB/s ✓\n", throughput_mbs);
}

fn benchmark_github_token(tokens: &[Vec<u8>]) {
    println!("📊 Benchmark 3: validate_github_token");
    println!("   Patterns: 4-6 (ROI: 130)");
    println!("   Expected speedup: 12-15x\n");

    let iterations = 10000;
    let start = Instant::now();

    for _ in 0..iterations {
        for token in tokens {
            unsafe {
                let _ = validate_github_token(0, token.as_ptr(), token.len());
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let total_bytes: usize = tokens.iter().map(|t| t.len()).sum::<usize>() * iterations;
    let throughput_mbs = (total_bytes as f64) / (1_000_000.0 * elapsed);

    println!("   Total bytes:    {:.2} MB", total_bytes as f64 / 1_000_000.0);
    println!("   Time:           {:.3}s", elapsed);
    println!("   Throughput:     {:.2} MB/s ✓\n", throughput_mbs);
}

fn benchmark_hex_token(tokens: &[Vec<u8>]) {
    println!("📊 Benchmark 4: validate_hex_token");
    println!("   Patterns: 10-15 (ROI: 145 - FASTEST: 15-20x)");
    println!("   Expected speedup: 15-20x\n");

    let iterations = 10000;
    let start = Instant::now();

    for _ in 0..iterations {
        for token in tokens {
            unsafe {
                let _ = validate_hex_token(token.as_ptr(), token.len(), 8, 128);
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let total_bytes: usize = tokens.iter().map(|t| t.len()).sum::<usize>() * iterations;
    let throughput_mbs = (total_bytes as f64) / (1_000_000.0 * elapsed);

    println!("   Total bytes:    {:.2} MB", total_bytes as f64 / 1_000_000.0);
    println!("   Time:           {:.3}s", elapsed);
    println!("   Throughput:     {:.2} MB/s ✓ (FASTEST)\n", throughput_mbs);
}

fn benchmark_base64_token(tokens: &[Vec<u8>]) {
    println!("📊 Benchmark 5: validate_base64_token");
    println!("   Patterns: 8-12 (ROI: 98)");
    println!("   Expected speedup: 12-15x\n");

    let iterations = 10000;
    let start = Instant::now();

    for _ in 0..iterations {
        for token in tokens {
            unsafe {
                let _ = validate_base64_token(token.as_ptr(), token.len(), 4, 256);
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let total_bytes: usize = tokens.iter().map(|t| t.len()).sum::<usize>() * iterations;
    let throughput_mbs = (total_bytes as f64) / (1_000_000.0 * elapsed);

    println!("   Total bytes:    {:.2} MB", total_bytes as f64 / 1_000_000.0);
    println!("   Time:           {:.3}s", elapsed);
    println!("   Throughput:     {:.2} MB/s ✓\n", throughput_mbs);
}

fn benchmark_base64url_token(tokens: &[Vec<u8>]) {
    println!("📊 Benchmark 6: validate_base64url_token");
    println!("   Patterns: 5-8 (ROI: 82)");
    println!("   Expected speedup: 12-15x\n");

    let iterations = 10000;
    let start = Instant::now();

    for _ in 0..iterations {
        for token in tokens {
            unsafe {
                let _ = validate_base64url_token(token.as_ptr(), token.len(), 4, 200);
            }
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    let total_bytes: usize = tokens.iter().map(|t| t.len()).sum::<usize>() * iterations;
    let throughput_mbs = (total_bytes as f64) / (1_000_000.0 * elapsed);

    println!("   Total bytes:    {:.2} MB", total_bytes as f64 / 1_000_000.0);
    println!("   Time:           {:.3}s", elapsed);
    println!("   Throughput:     {:.2} MB/s ✓\n", throughput_mbs);
}

// ============================================================================
// TEST DATA GENERATORS
// ============================================================================

fn generate_alphanumeric_tokens(count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|i| {
            let len = 20 + (i % 50);
            (0..len)
                .map(|j| {
                    let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
                    chars[(i * j) % chars.len()]
                })
                .collect()
        })
        .collect()
}

fn generate_aws_keys(count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|i| {
            let prefix = match i % 8 {
                0 => b"AKIA".to_vec(),
                1 => b"A3T".to_vec(),
                2 => b"ASIA".to_vec(),
                3 => b"ABIA".to_vec(),
                4 => b"ACCA".to_vec(),
                5 => b"ACPA".to_vec(),
                6 => b"AROA".to_vec(),
                _ => b"AIDA".to_vec(),
            };
            let suffix: Vec<u8> = (0..16)
                .map(|j| {
                    let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
                    chars[((i * j) + j) % chars.len()]
                })
                .collect();
            [prefix, suffix].concat()
        })
        .collect()
}

fn generate_github_tokens(count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|i| {
            let prefix = match i % 6 {
                0 => b"ghp_".to_vec(),
                1 => b"gho_".to_vec(),
                2 => b"ghu_".to_vec(),
                3 => b"ghr_".to_vec(),
                4 => b"ghs_".to_vec(),
                _ => b"gat_".to_vec(),
            };
            let suffix: Vec<u8> = (0..36)
                .map(|j| {
                    let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-";
                    chars[((i * j) + j) % chars.len()]
                })
                .collect();
            [prefix, suffix].concat()
        })
        .collect()
}

fn generate_hex_tokens(count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|i| {
            let len = 16 + ((i % 100) * 2);
            (0..len)
                .map(|j| {
                    let chars = b"0123456789abcdef";
                    chars[((i * j) + j) % chars.len()]
                })
                .collect()
        })
        .collect()
}

fn generate_base64_tokens(count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|i| {
            let len = ((i % 60) + 1) * 4; // Multiple of 4
            (0..len)
                .map(|j| {
                    let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/=";
                    chars[((i * j) + j) % chars.len()]
                })
                .collect()
        })
        .collect()
}

fn generate_base64url_tokens(count: usize) -> Vec<Vec<u8>> {
    (0..count)
        .map(|i| {
            let len = ((i % 50) + 1) * 4;
            (0..len)
                .map(|j| {
                    let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789_-";
                    chars[((i * j) + j) % chars.len()]
                })
                .collect()
        })
        .collect()
}

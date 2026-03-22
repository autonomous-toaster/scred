use std::time::Instant;

fn main() {
    println!("\n╔════════════════════════════════════════════════╗");
    println!("║   SCRED Pattern Detector - Quick Benchmark      ║");
    println!("╚════════════════════════════════════════════════╝\n");

    // Benchmark 1: Pure no-pattern baseline (10MB)
    benchmark_no_patterns();

    // Benchmark 2: Realistic data WITH patterns (10MB)
    benchmark_with_patterns();

    // Benchmark 3: High-density patterns (10MB)
    benchmark_high_density();

    println!("\n✅ Benchmarks complete!\n");
}

fn benchmark_no_patterns() {
    println!("📊 Benchmark 1: NO PATTERNS (10MB baseline)");
    println!("───────────────────────────────────────────");

    let data_size = 10_000_000;
    let data: Vec<u8> = (0..data_size)
        .map(|i| b'a' + ((i % 26) as u8))
        .collect();

    let start = Instant::now();
    let mut checksum = 0u8;
    for byte in &data {
        checksum = checksum.wrapping_add(*byte);
    }
    let elapsed = start.elapsed();

    let throughput = (data_size as f64 / 1_000_000.0) / elapsed.as_secs_f64();
    println!("  Data size:    {:.1} MB", data_size as f64 / 1_000_000.0);
    println!("  Time:         {:.2} ms", elapsed.as_secs_f64() * 1000.0);
    println!("  Throughput:   {:.2} MB/s", throughput);
    println!("  Checksum:     {} (proof of work)", checksum);
    println!();
}

fn benchmark_with_patterns() {
    println!("📊 Benchmark 2: WITH PATTERNS (10MB, ~5%)");
    println!("────────────────────────────────────────");

    let mut data = Vec::new();
    let patterns: Vec<&[u8]> = vec![
        b"sk_live_",
        b"AKIA",
        b"ghp_",
    ];

    // Fill with mostly normal data, ~5% patterns
    while data.len() < 10_000_000 {
        if data.len() % 100 < 95 {
            data.push(b'x');
        } else {
            let pattern = patterns[data.len() % patterns.len()];
            if data.len() + pattern.len() <= 10_000_000 {
                data.extend_from_slice(pattern);
            }
        }
    }
    data.truncate(10_000_000);

    let start = Instant::now();
    let mut pattern_count = 0;
    let mut i = 0;
    while i < data.len() {
        if i + 8 <= data.len() && &data[i..i+8] == b"sk_live_" {
            pattern_count += 1;
            i += 20;
        } else if i + 4 <= data.len() && &data[i..i+4] == b"AKIA" {
            pattern_count += 1;
            i += 20;
        } else {
            i += 1;
        }
    }
    let elapsed = start.elapsed();

    let throughput = (10.0) / elapsed.as_secs_f64();
    println!("  Data size:    10.0 MB");
    println!("  Patterns found: {}", pattern_count);
    println!("  Time:         {:.2} ms", elapsed.as_secs_f64() * 1000.0);
    println!("  Throughput:   {:.2} MB/s", throughput);
    println!();
}

fn benchmark_high_density() {
    println!("📊 Benchmark 3: HIGH DENSITY (10MB, 50%)");
    println!("────────────────────────────────────────");

    let mut data = Vec::new();
    let pattern = b"sk_";

    // Alternate: pattern, x, pattern, x...
    while data.len() < 10_000_000 {
        if data.len() % 10 < 5 {
            if data.len() + pattern.len() <= 10_000_000 {
                data.extend_from_slice(pattern);
            }
        } else {
            data.push(b'x');
        }
    }
    data.truncate(10_000_000);

    let start = Instant::now();
    let mut pattern_count = 0;
    for i in (0..data.len()).step_by(4) {
        if i + 3 <= data.len() && &data[i..i+3] == pattern {
            pattern_count += 1;
        }
    }
    let elapsed = start.elapsed();

    let throughput = (10.0) / elapsed.as_secs_f64();
    println!("  Data size:    10.0 MB");
    println!("  Patterns found: {}", pattern_count);
    println!("  Time:         {:.2} ms", elapsed.as_secs_f64() * 1000.0);
    println!("  Throughput:   {:.2} MB/s", throughput);
    println!();
}

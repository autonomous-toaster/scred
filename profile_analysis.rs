// Quick profiling test to measure time breakdown
use std::time::Instant;

fn main() {
    println!("\n╔════════════════════════════════════════════════╗");
    println!("║   Pattern Detector - Time Breakdown Analysis    ║");
    println!("╚════════════════════════════════════════════════╝\n");

    // Test 1: Measure pattern matching overhead alone
    println!("📊 Test 1: Pattern Matching Overhead");
    measure_pattern_matching();

    // Test 2: Measure buffer copying overhead
    println!("\n📊 Test 2: Buffer Copying Overhead");
    measure_buffer_copying();

    // Test 3: Measure token scanning overhead
    println!("\n📊 Test 3: Token Boundary Scanning");
    measure_token_scanning();

    println!("\n✅ Analysis complete!\n");
}

fn measure_pattern_matching() {
    // Create data with many potential pattern starts
    let data: Vec<u8> = (0..10_000_000)
        .map(|i| if i % 100 == 0 { b's' } else { b'x' })
        .collect();

    let start = Instant::now();
    let mut count = 0;
    for i in 0..data.len() {
        if data[i] == b's' {
            count += 1;
        }
    }
    let elapsed = start.elapsed();

    println!("  First-char checks: 10M bytes in {:.2} ms ({:.1} ns/byte)",
        elapsed.as_secs_f64() * 1000.0,
        elapsed.as_nanos() as f64 / 10_000_000.0
    );
    println!("  Matches: {}", count);
}

fn measure_buffer_copying() {
    let data: Vec<u8> = (0..10_000_000).map(|i| (i % 256) as u8).collect();
    let mut output = vec![0u8; 10_000_000];

    let start = Instant::now();
    output.copy_from_slice(&data);
    let elapsed = start.elapsed();

    println!("  memcpy 10M bytes: {:.2} ms ({:.1} GB/s)",
        elapsed.as_secs_f64() * 1000.0,
        10.0 / elapsed.as_secs_f64()
    );
}

fn measure_token_scanning() {
    let data: Vec<u8> = (0..10_000_000)
        .map(|i| {
            if i % 50 == 0 { b' ' } else { b'a' }
        })
        .collect();

    let valid_chars = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.:/+=";
    let mut valid_set = [false; 256];
    for &c in valid_chars {
        valid_set[c as usize] = true;
    }

    let start = Instant::now();
    let mut total_len = 0usize;
    let mut i = 0;
    while i < data.len() {
        let mut token_len = 0;
        while i < data.len() && token_len < 128 {
            if valid_set[data[i] as usize] {
                token_len += 1;
                i += 1;
            } else {
                break;
            }
        }
        total_len += token_len;
        i += 1; // Skip delimiter
    }
    let elapsed = start.elapsed();

    println!("  Token scanning (10M, cap=128): {:.2} ms ({:.1} ns/byte)",
        elapsed.as_secs_f64() * 1000.0,
        elapsed.as_nanos() as f64 / 10_000_000.0
    );
    println!("  Total token length: {}", total_len);
}

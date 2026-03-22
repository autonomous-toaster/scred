use std::time::Instant;

fn main() {
    println!("\n╔════════════════════════════════════════════════╗");
    println!("║   SCRED - Minimal Profiling                     ║");
    println!("╚════════════════════════════════════════════════╝\n");

    // Test 1: Simple loop
    println!("📊 Test 1: Simple loop (10MB)");
    test_simple_loop();

    // Test 2: Slice comparison
    println!("\n📊 Test 2: Slice comparison (10MB)");
    test_slice_comparison();

    // Test 3: FFI overhead
    println!("\n📊 Test 3: FFI call overhead");
    test_ffi_overhead();

    println!("\n✅ Done!\n");
}

fn test_simple_loop() {
    let data: Vec<u8> = (0..10_000_000).map(|i| (i % 256) as u8).collect();
    let start = Instant::now();
    let mut count = 0;
    for i in 0..data.len() {
        if data[i] == b'A' {
            count += 1;
        }
    }
    let elapsed = start.elapsed();
    println!("  Found {} matches in {:.2} ms ({:.2} MB/s)", 
        count, elapsed.as_secs_f64() * 1000.0, 
        10.0 / elapsed.as_secs_f64());
}

fn test_slice_comparison() {
    let data: Vec<u8> = (0..10_000_000).map(|i| (i % 256) as u8).collect();
    let start = Instant::now();
    let mut count = 0;
    let mut i = 0;
    while i < data.len() {
        // Check for "AKIA" (4 bytes)
        if i + 4 <= data.len() && &data[i..i+4] == b"AKIA" {
            count += 1;
            i += 20;
        } else {
            i += 1;
        }
    }
    let elapsed = start.elapsed();
    println!("  Found {} matches in {:.2} ms ({:.2} MB/s)", 
        count, elapsed.as_secs_f64() * 1000.0, 
        10.0 / elapsed.as_secs_f64());
}

fn test_ffi_overhead() {
    extern "C" {
        fn strlen(s: *const u8) -> usize;
    }

    let pattern = b"test pattern\0";
    let start = Instant::now();
    let mut total = 0;
    for _ in 0..1_000_000 {
        unsafe {
            total += strlen(pattern.as_ptr());
        }
    }
    let elapsed = start.elapsed();
    println!("  1M FFI calls in {:.2} ms (result: {})", elapsed.as_secs_f64() * 1000.0, total);
}

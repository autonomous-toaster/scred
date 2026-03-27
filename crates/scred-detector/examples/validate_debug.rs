//! Debug validation performance


fn main() {
    // Generate realistic data
    const SIZE: usize = 10 * 1024 * 1024;  // Start smaller
    let mut data = Vec::with_capacity(SIZE);
    let patterns: &[&[u8]] = &[
        b"This is normal text without secrets here.\n",
        b"AWS Key: AKIAIOSFODNN7EXAMPLE\n",
    ];
    while data.len() < SIZE {
        for pattern in patterns {
            if data.len() >= SIZE { break; }
            data.extend_from_slice(pattern);
        }
    }
    data.truncate(SIZE);

    println!("Profiling detect_validation on {}MB...", SIZE / (1024*1024));
    
    for i in 1..=3 {
        let start = std::time::Instant::now();
        let result = scred_detector::detect_validation(&data);
        let elapsed = start.elapsed();
        
        let throughput = (SIZE as f64) / elapsed.as_secs_f64() / (1024.0 * 1024.0);
        println!("Run {}: {:.2}ms, {:.2} MB/s, {} matches",
                 i, elapsed.as_secs_f64() * 1000.0, throughput, result.matches.len());
    }
}

use scred_detector::detector::detect_all;
use std::time::Instant;

fn main() {
    // Create a 100MB test payload with mixed patterns
    const MB: usize = 1024 * 1024;
    const SIZE: usize = 100 * MB;
    
    // Mix of SSH keys and regular text
    let ssh_key = b"-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEA2a2rwplTCHpjyY7X0X0bVTMdNPaKQ5SzLYrJzxJ5E4Kj4nQX
YZ7S9XY8Fkl0ZAB1CdEfGhIjKlOpQrRkXxA9TvMpZqZ0pPqLmNzXyK5rQxZpZ8Qx
-----END RSA PRIVATE KEY-----";
    
    let regular_text = b"This is regular application log text with some AWS_SECRET_ACCESS_KEY=abc123456def values\n";
    
    let mut test_data = Vec::with_capacity(SIZE);
    
    // Fill with mixed pattern
    while test_data.len() < SIZE {
        test_data.extend_from_slice(regular_text);
        if test_data.len() < SIZE {
            test_data.extend_from_slice(ssh_key);
        }
    }
    test_data.truncate(SIZE);
    
    println!("Test data size: {:.2} MB", test_data.len() as f64 / MB as f64);
    
    // Warm up
    let _ = detect_all(&test_data);
    
    // Measure
    let start = Instant::now();
    let matches = detect_all(&test_data);
    let elapsed = start.elapsed();
    
    let throughput = SIZE as f64 / elapsed.as_secs_f64() / MB as f64;
    
    println!("Time:       {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    println!("Throughput: {:.2} MB/s", throughput);
    println!("Patterns:   {}", matches.matches.len());
    
    if throughput >= 120.0 {
        println!("✅ Phase A achieving ~120 MB/s baseline!");
    } else if throughput >= 100.0 {
        println!("✅ Phase A achieving solid performance (~100 MB/s)");
    } else {
        println!("⚠️  Performance lower than expected: {:.2} MB/s", throughput);
    }
}

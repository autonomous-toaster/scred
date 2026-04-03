use scred_detector::detect_all;
use scred_detector::redact_in_place;
use std::time::Instant;

fn main() {
    // Generate 10MB test data - dense case
    let pattern = b"Line with AKIAIOSFODNN7EXAMPLE secret key and more data\n";
    let target_size = 10 * 1024 * 1024;
    let mut data = Vec::with_capacity(target_size);
    while data.len() < target_size {
        data.extend_from_slice(pattern);
    }
    data.truncate(target_size);

    // Profile just detection
    let start = Instant::now();
    for _ in 0..10 {
        let _ = detect_all(&data);
    }
    let detect_time = start.elapsed();

    // Profile just redaction (reuse detection results)
    let detection = detect_all(&data);
    let start = Instant::now();
    for _ in 0..10 {
        let mut test_data = data.clone();
        redact_in_place(&mut test_data, &detection.matches);
    }
    let redact_time = start.elapsed();

    eprintln!("Detection (10 runs): {:.0}ms", detect_time.as_millis());
    eprintln!("Redaction (10 runs): {:.0}ms", redact_time.as_millis());
    eprintln!("Total: {:.0}ms", (detect_time + redact_time).as_millis());
    eprintln!(
        "\nRatio: detect {} : redact {}",
        detect_time.as_millis(),
        redact_time.as_millis()
    );
}

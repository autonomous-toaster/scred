use std::time::Instant;

fn main() {
    let target_size = 1 * 1024 * 1024; // 1MB

    // Create very dense pattern (secret every 20 bytes)
    let mut data = Vec::with_capacity(target_size);
    let pattern = b"AKIAIOSFODNN7EXAMPLE_";
    while data.len() < target_size {
        data.extend_from_slice(pattern);
    }
    data.truncate(target_size);

    println!("Data size: {} bytes", data.len());

    // Profile individual detectors
    let start = Instant::now();
    let result = scred_detector::detect_simple_prefix(&data);
    let time1 = start.elapsed().as_millis();
    println!(
        "simple_prefix: {}ms, {} matches",
        time1,
        result.matches.len()
    );

    let start = Instant::now();
    let result = scred_detector::detect_validation(&data);
    let time2 = start.elapsed().as_millis();
    println!("validation: {}ms, {} matches", time2, result.matches.len());

    let start = Instant::now();
    let result = scred_detector::detect_jwt(&data);
    let time3 = start.elapsed().as_millis();
    println!("jwt: {}ms, {} matches", time3, result.matches.len());

    let start = Instant::now();
    let result = scred_detector::detect_ssh_keys(&data);
    let time4 = start.elapsed().as_millis();
    println!("ssh_keys: {}ms, {} matches", time4, result.matches.len());

    let start = Instant::now();
    let result = scred_detector::detect_uri_patterns(&data);
    let time5 = start.elapsed().as_millis();
    println!(
        "uri_patterns: {}ms, {} matches",
        time5,
        result.matches.len()
    );

    println!("Total: {}ms", time1 + time2 + time3 + time4 + time5);
}

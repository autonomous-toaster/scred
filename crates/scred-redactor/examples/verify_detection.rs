use scred_detector::detect_all;

fn main() {
    let normal_line = b"This is completely normal data with no secrets whatsoever here\n";
    let mut data = Vec::new();
    while data.len() < 1024 * 1024 {
        data.extend_from_slice(normal_line);
    }
    data.truncate(1024 * 1024);

    let result = detect_all(&data);
    eprintln!(
        "Patterns found in 1MB normal data: {}",
        result.matches.len()
    );

    let secret_line = b"Line with AKIAIOSFODNN7EXAMPLE secret key and more data\n";
    let mut data2 = Vec::new();
    while data2.len() < 1024 * 1024 {
        data2.extend_from_slice(secret_line);
    }
    data2.truncate(1024 * 1024);

    let result2 = detect_all(&data2);
    eprintln!(
        "Patterns found in 1MB AWS key data: {}",
        result2.matches.len()
    );
}

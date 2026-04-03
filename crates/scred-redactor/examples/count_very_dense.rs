use scred_detector::detect_all;

fn main() {
    let pattern = b"AKIAIOSFODNN7EXAMPLE_";
    let mut data = Vec::new();
    let target_size = 10 * 1024 * 1024;
    while data.len() < target_size {
        data.extend_from_slice(pattern);
    }
    data.truncate(target_size);

    let result = detect_all(&data);
    eprintln!(
        "Patterns found in very dense data: {}",
        result.matches.len()
    );
}

use scred_detector::detect_all;

fn main() {
    let target_size = 10 * 1024 * 1024;
    let mut data = Vec::with_capacity(target_size);
    
    let normal_lines: &[&[u8]] = &[
        b"[2024-03-27T10:00:00Z] Starting application\n",
        b"[2024-03-27T10:00:01Z] Connecting to database\n",
        b"[2024-03-27T10:00:02Z] Database connection established\n",
        b"[2024-03-27T10:00:03Z] Loading configuration\n",
        b"[2024-03-27T10:00:04Z] Configuration loaded successfully\n",
    ];
    let secret_line = b"[2024-03-27T10:00:05Z] Using credentials: AKIAIOSFODNN7EXAMPLE\n";
    
    let lines_per_secret = (100 * 1024) / 46;
    let mut line_count = 0;
    while data.len() < target_size {
        if line_count % lines_per_secret == 0 {
            data.extend_from_slice(secret_line);
        } else {
            data.extend_from_slice(normal_lines[line_count % normal_lines.len()]);
        }
        line_count += 1;
    }
    data.truncate(target_size);
    
    let result = detect_all(&data);
    eprintln!("Patterns found in realistic data: {}", result.matches.len());
}

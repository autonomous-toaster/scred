use scred_redactor::{RedactionConfig, RedactionEngine, StreamingRedactor};
use std::sync::Arc;
use std::time::Instant;

fn benchmark_scenario(name: &str, data: &[u8]) {
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let redactor = StreamingRedactor::with_defaults(engine);

    let chunk_size = 64 * 1024;
    let chunks: Vec<&[u8]> = data.chunks(chunk_size).collect();

    let start = Instant::now();
    let mut lookahead = Vec::new();
    for (i, chunk) in chunks.iter().enumerate() {
        let is_eof = i == chunks.len() - 1;
        let _ = redactor.process_chunk(chunk, &mut lookahead, is_eof);
    }
    let elapsed = start.elapsed();

    let throughput = (data.len() as f64) / 1024.0 / 1024.0 / elapsed.as_secs_f64();
    eprintln!("{:30}: {:.1} MB/s", name, throughput);
}

fn main() {
    let target_size = 10 * 1024 * 1024;

    // Scenario 1: No secrets
    let mut data = Vec::with_capacity(target_size);
    let normal = b"This is completely normal data with no secrets whatsoever\n";
    while data.len() < target_size {
        data.extend_from_slice(normal);
    }
    data.truncate(target_size);
    benchmark_scenario("No secrets", &data);

    // Scenario 2: Sparse (realistic logs)
    let mut data = Vec::with_capacity(target_size);
    let normal_lines: &[&[u8]] = &[
        b"[2024-03-27T10:00:00Z] Starting application\n",
        b"[2024-03-27T10:00:01Z] Connecting to database\n",
        b"[2024-03-27T10:00:02Z] Database connection established\n",
        b"[2024-03-27T10:00:03Z] Loading configuration\n",
        b"[2024-03-27T10:00:04Z] Configuration loaded successfully\n",
    ];
    let secret = b"[2024-03-27T10:00:05Z] Using credentials: AKIAIOSFODNN7EXAMPLE\n";
    let lines_per_secret = (100 * 1024) / 46;
    let mut line_count = 0;
    while data.len() < target_size {
        if line_count % lines_per_secret == 0 {
            data.extend_from_slice(secret);
        } else {
            data.extend_from_slice(normal_lines[line_count % normal_lines.len()]);
        }
        line_count += 1;
    }
    data.truncate(target_size);
    benchmark_scenario("Realistic (1 secret/100KB)", &data);

    // Scenario 3: Dense (every line)
    let mut data = Vec::with_capacity(target_size);
    let secret_line = b"Line with AKIAIOSFODNN7EXAMPLE secret key and more data\n";
    while data.len() < target_size {
        data.extend_from_slice(secret_line);
    }
    data.truncate(target_size);
    benchmark_scenario("Dense (every line)", &data);

    // Scenario 4: High density (secrets every 10 bytes)
    let mut data = Vec::with_capacity(target_size);
    let pattern = b"AKIAIOSFODNN7EXAMPLE_";
    while data.len() < target_size {
        data.extend_from_slice(pattern);
    }
    data.truncate(target_size);
    benchmark_scenario("Very dense (every 20 bytes)", &data);
}

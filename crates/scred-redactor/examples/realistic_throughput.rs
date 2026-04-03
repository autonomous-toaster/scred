use scred_redactor::{RedactionConfig, RedactionEngine, StreamingRedactor};
use std::sync::Arc;
use std::time::Instant;

fn benchmark_pattern(name: &str, pattern_every_n_lines: usize) {
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let redactor = StreamingRedactor::with_defaults(engine);

    // Generate realistic data: secrets every N lines
    let target_size = 10 * 1024 * 1024;
    let mut data = Vec::with_capacity(target_size);

    let secret_line = b"Line with AKIAIOSFODNN7EXAMPLE secret key and more data\n";
    let normal_line = b"This is completely normal data with no secrets at all here\n";

    let mut line_count = 0;
    while data.len() < target_size {
        if line_count % pattern_every_n_lines == 0 {
            data.extend_from_slice(secret_line);
        } else {
            data.extend_from_slice(normal_line);
        }
        line_count += 1;
    }
    data.truncate(target_size);

    // Process in 64KB chunks
    let chunk_size = 64 * 1024;
    let chunks: Vec<&[u8]> = data.chunks(chunk_size).collect();

    let start = Instant::now();
    let mut lookahead = Vec::new();
    for (i, chunk) in chunks.iter().enumerate() {
        let is_eof = i == chunks.len() - 1;
        let _ = redactor.process_chunk(chunk, &mut lookahead, is_eof);
    }
    let elapsed = start.elapsed();

    let throughput = 10.0 / elapsed.as_secs_f64();
    eprintln!(
        "{}: 1 per {} lines = {:.2}ms = {:.1} MB/s",
        name,
        pattern_every_n_lines,
        elapsed.as_secs_f64() * 1000.0,
        throughput
    );
}

fn main() {
    eprintln!("Testing realistic pattern densities:");
    benchmark_pattern("100%", 1);
    benchmark_pattern("50%", 2);
    benchmark_pattern("10%", 10);
    benchmark_pattern("1%", 100);
    benchmark_pattern("0.1%", 1000);
}

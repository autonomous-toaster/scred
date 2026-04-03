use scred_redactor::{RedactionConfig, RedactionEngine, StreamingRedactor};
use std::sync::Arc;
use std::time::Instant;

fn benchmark_chunk_size(size_kb: usize) {
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let redactor = StreamingRedactor::with_defaults(engine);

    // Generate test data
    let pattern = b"Line with AKIAIOSFODNN7EXAMPLE secret key and more data\n";
    let target_size = 10 * 1024 * 1024;
    let mut data = Vec::with_capacity(target_size);
    while data.len() < target_size {
        data.extend_from_slice(pattern);
    }
    data.truncate(target_size);

    // Benchmark this chunk size
    let chunk_size = size_kb * 1024;
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
        "{:3}KB: {:.2}ms = {:.1} MB/s",
        size_kb,
        elapsed.as_secs_f64() * 1000.0,
        throughput
    );
}

fn main() {
    eprintln!("Chunk size tuning:");
    for size_kb in &[16, 32, 48, 64, 96, 128, 192, 256, 512] {
        benchmark_chunk_size(*size_kb);
    }
}

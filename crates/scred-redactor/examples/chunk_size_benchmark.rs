//! Benchmark different chunk sizes to find optimal performance

use scred_redactor::{RedactionConfig, RedactionEngine, StreamingConfig, StreamingRedactor};
use std::sync::Arc;
use std::time::Instant;

fn main() {
    // Generate test data
    const SIZE: usize = 100 * 1024 * 1024;
    let mut data = Vec::new();
    let pattern = b"Line with AKIAIOSFODNN7EXAMPLE secret key\n";
    while data.len() < SIZE {
        data.extend_from_slice(pattern);
    }
    data.truncate(SIZE);

    println!("\n════════════════════════════════════════════════════════");
    println!("  CHUNK SIZE PERFORMANCE BENCHMARK");
    println!("════════════════════════════════════════════════════════\n");

    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let chunk_sizes = vec![16 * 1024, 32 * 1024, 64 * 1024, 128 * 1024, 256 * 1024];

    for chunk_size in chunk_sizes {
        let config = StreamingConfig {
            chunk_size,
            lookahead_size: 512,
        };
        let redactor = StreamingRedactor::new(engine.clone(), config);

        let mut lookahead = Vec::with_capacity(512);
        let mut total_output: u64 = 0;
        let mut pattern_count: u64 = 0;

        let start = Instant::now();
        for chunk in data.chunks(chunk_size) {
            let is_eof = chunk.len() < chunk_size;
            let (output, bytes_written, patterns) =
                redactor.process_chunk(chunk, &mut lookahead, is_eof);
            total_output += bytes_written;
            pattern_count += patterns;
        }
        let elapsed = start.elapsed();

        let throughput = (SIZE as f64) / elapsed.as_secs_f64() / (1024.0 * 1024.0);

        println!(
            "Chunk size: {:6} bytes ({:3}KB)",
            chunk_size,
            chunk_size / 1024
        );
        println!("  Time:       {:.2}ms", elapsed.as_secs_f64() * 1000.0);
        println!("  Throughput: {:.2} MB/s", throughput);
        println!("  Patterns:   {}", pattern_count);
        println!("  Output:     {} bytes\n", total_output);
    }

    println!("════════════════════════════════════════════════════════");
}

use scred_redactor::{RedactionConfig, RedactionEngine, StreamingRedactor};
use std::sync::Arc;
use std::time::Instant;

fn main() {
    // Test streaming redaction with multiple chunks
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let redactor = StreamingRedactor::with_defaults(engine);

    // Generate 10MB of test data with secrets
    let pattern = b"Line with AKIAIOSFODNN7EXAMPLE secret key and more data\n";
    let target_size = 10 * 1024 * 1024; // 10MB
    let mut data = Vec::with_capacity(target_size);
    while data.len() < target_size {
        data.extend_from_slice(pattern);
    }
    data.truncate(target_size);

    // Simulate streaming by processing in 64KB chunks
    let chunk_size = 64 * 1024;
    let chunks: Vec<&[u8]> = data.chunks(chunk_size).collect();

    let start = Instant::now();
    let mut total_output_len = 0;
    let mut total_patterns = 0;
    let mut lookahead = Vec::new();

    for (i, chunk) in chunks.iter().enumerate() {
        let is_eof = i == chunks.len() - 1;
        let (output, patterns_found, _lookahead_len) =
            redactor.process_chunk(chunk, &mut lookahead, is_eof);
        total_output_len += output.len();
        total_patterns += patterns_found;
    }

    // Handle final lookahead
    total_output_len += lookahead.len();

    let elapsed = start.elapsed();

    let mb = 10.0;
    let secs = elapsed.as_secs_f64();
    let throughput = mb / secs;

    eprintln!("METRIC throughput_mbs={:.1}", throughput);
    eprintln!("Time: {:.2}s", elapsed.as_secs_f64());
    eprintln!("Total output size: {} bytes", total_output_len);
    eprintln!("Total patterns found: {}", total_patterns);
    eprintln!("Chunks processed: {}", chunks.len());
}

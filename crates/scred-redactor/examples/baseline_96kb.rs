use std::time::Instant;
use scred_redactor::{RedactionEngine, RedactionConfig, StreamingRedactor};
use std::sync::Arc;

fn main() {
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let redactor = StreamingRedactor::with_defaults(engine);
    
    // Generate 10MB test data
    let pattern = b"Line with AKIAIOSFODNN7EXAMPLE secret key and more data\n";
    let target_size = 10 * 1024 * 1024;
    let mut data = Vec::with_capacity(target_size);
    while data.len() < target_size {
        data.extend_from_slice(pattern);
    }
    data.truncate(target_size);
    
    // Process in 96KB chunks (new optimal size)
    let chunk_size = 96 * 1024;
    let chunks: Vec<&[u8]> = data.chunks(chunk_size).collect();
    
    let start = Instant::now();
    let mut lookahead = Vec::new();
    for (i, chunk) in chunks.iter().enumerate() {
        let is_eof = i == chunks.len() - 1;
        let _ = redactor.process_chunk(chunk, &mut lookahead, is_eof);
    }
    let elapsed = start.elapsed();
    
    let throughput = 10.0 / elapsed.as_secs_f64();
    eprintln!("METRIC throughput_mbs={:.1}", throughput);
    eprintln!("Time: {:.2}ms", elapsed.as_secs_f64() * 1000.0);
    eprintln!("Chunks: {}", chunks.len());
}

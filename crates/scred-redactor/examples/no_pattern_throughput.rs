use std::time::Instant;
use scred_redactor::{RedactionEngine, RedactionConfig, StreamingRedactor};
use std::sync::Arc;

fn main() {
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let redactor = StreamingRedactor::with_defaults(engine);
    
    // Generate 10MB with NO secrets (normal text)
    let normal_line = b"This is completely normal data with no secrets whatsoever here\n";
    let target_size = 10 * 1024 * 1024;
    let mut data = Vec::with_capacity(target_size);
    while data.len() < target_size {
        data.extend_from_slice(normal_line);
    }
    data.truncate(target_size);
    
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
    eprintln!("No patterns: {:.1} MB/s ({:.0}ms)", throughput, elapsed.as_millis());
}

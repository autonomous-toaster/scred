use std::time::Instant;
use scred_redactor::{RedactionEngine, RedactionConfig, StreamingRedactor};
use std::sync::Arc;

fn main() {
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let redactor = StreamingRedactor::with_defaults(engine);
    
    // Generate test data - 10% pattern density (optimal from earlier test)
    let target_size = 10 * 1024 * 1024;
    let mut data = Vec::with_capacity(target_size);
    let secret_line = b"Line with AKIAIOSFODNN7EXAMPLE secret key and more data\n";
    let normal_line = b"This is completely normal data with no secrets at all here\n";
    
    let mut line_count = 0;
    while data.len() < target_size {
        if line_count % 10 == 0 {
            data.extend_from_slice(secret_line);
        } else {
            data.extend_from_slice(normal_line);
        }
        line_count += 1;
    }
    data.truncate(target_size);
    
    let chunk_size = 64 * 1024;
    let chunks: Vec<&[u8]> = data.chunks(chunk_size).collect();
    
    // Test process_chunk
    let start = Instant::now();
    let mut lookahead = Vec::new();
    for (i, chunk) in chunks.iter().enumerate() {
        let is_eof = i == chunks.len() - 1;
        let _ = redactor.process_chunk(chunk, &mut lookahead, is_eof);
    }
    let elapsed1 = start.elapsed();
    
    // Test process_chunk_in_place  
    let start = Instant::now();
    let mut lookahead = Vec::new();
    for (i, chunk) in chunks.iter().enumerate() {
        let is_eof = i == chunks.len() - 1;
        let _ = redactor.process_chunk_in_place(chunk, &mut lookahead, is_eof);
    }
    let elapsed2 = start.elapsed();
    
    eprintln!("process_chunk:         {:.1} MB/s ({:.0}ms)", 10.0 / elapsed1.as_secs_f64(), elapsed1.as_millis());
    eprintln!("process_chunk_in_place: {:.1} MB/s ({:.0}ms)", 10.0 / elapsed2.as_secs_f64(), elapsed2.as_millis());
    
    if elapsed2 < elapsed1 {
        let improvement = (elapsed1.as_secs_f64() - elapsed2.as_secs_f64()) / elapsed1.as_secs_f64() * 100.0;
        eprintln!("\nprocess_chunk_in_place is {:.1}% faster", improvement);
    }
}

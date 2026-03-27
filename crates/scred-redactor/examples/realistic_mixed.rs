use std::time::Instant;
use scred_redactor::{RedactionEngine, RedactionConfig, StreamingRedactor};
use std::sync::Arc;

fn main() {
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let redactor = StreamingRedactor::with_defaults(engine);
    
    // Realistic data: mostly logs with ONE AWS key hidden per 100KB
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
    
    let lines_per_secret = (100 * 1024) / 46; // ~100KB per secret
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
    eprintln!("METRIC throughput_mbs={:.1}", throughput);
    eprintln!("Realistic log data (~1 secret per 100KB): {:.1} MB/s ({:.0}ms)", throughput, elapsed.as_millis());
}

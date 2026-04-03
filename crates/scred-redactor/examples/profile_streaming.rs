use scred_redactor::{RedactionConfig, RedactionEngine, StreamingRedactor};
use std::sync::Arc;
use std::time::Instant;

fn main() {
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

    // Run 3 times to get average and warm up
    let mut times = vec![];
    for run in 0..3 {
        let chunk_size = 64 * 1024;
        let chunks: Vec<&[u8]> = data.chunks(chunk_size).collect();

        let start = Instant::now();
        let mut lookahead = Vec::new();
        for (i, chunk) in chunks.iter().enumerate() {
            let is_eof = i == chunks.len() - 1;
            let _ = redactor.process_chunk(chunk, &mut lookahead, is_eof);
        }
        let elapsed = start.elapsed();
        times.push(elapsed);

        let throughput = 10.0 / elapsed.as_secs_f64();
        eprintln!(
            "Run {}: {:.2}ms = {:.1} MB/s",
            run + 1,
            elapsed.as_secs_f64() * 1000.0,
            throughput
        );
    }

    let avg = times.iter().map(|t| t.as_secs_f64()).sum::<f64>() / times.len() as f64;
    eprintln!("\nAverage: {:.2}ms = {:.1} MB/s", avg * 1000.0, 10.0 / avg);
    eprintln!("METRIC throughput_mbs={:.1}", 10.0 / avg);
}

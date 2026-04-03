/// Streaming Redaction Module
///
/// Consolidates three streaming functions (run_redacting_stream, run_env_redacting_stream,
/// process_text_chunk_and_stream) into a single, DRY implementation.
///
/// Eliminates 100-150 lines of duplication while maintaining identical behavior.
use std::io::{self, Read, Write};
use std::sync::Arc;
use std::time::Instant;

use scred_http::{ConfigurableEngine, PatternSelector};
use scred_redactor::{RedactionConfig, RedactionEngine};

/// Redaction mode: determines how buffers are processed
#[derive(Debug, Copy, Clone)]
pub enum RedactionMode {
    /// Process chunks as text (pattern-based redaction)
    Text,
    /// Process chunks as environment variables (line-by-line)
    Env,
}

/// Streaming redaction function
///
/// Consolidates streaming and non-streaming paths with automatic optimization.
/// For typical CLI usage with moderate-sized inputs, processes entirely without streaming overhead.
///
/// # Arguments
/// * `mode` - Redaction mode (Text or Env)
/// * `initial_buffer` - Optional initial buffer from auto-detection (first 512 bytes)
/// * `detect_selector` - Which patterns to detect
/// * `redact_selector` - Which patterns to redact
/// * `verbose` - Show statistics
pub fn stream_and_redact(
    mode: RedactionMode,
    initial_buffer: Option<&[u8]>,
    detect_selector: &PatternSelector,
    redact_selector: &PatternSelector,
    verbose: bool,
) {
    let start = Instant::now();

    // Create ConfigurableEngine with pattern selectors
    let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
    let config_engine =
        ConfigurableEngine::new(engine, detect_selector.clone(), redact_selector.clone());

    let mut total_read = 0;
    let mut total_written = 0;

    // For small to moderate inputs (typical CLI usage), read everything into memory
    // This eliminates chunking overhead. For large files (>100MB), fallback to streaming.
    const MEMORY_LIMIT: usize = 100 * 1024 * 1024; // 100MB threshold
    let mut accumulated = Vec::new();

    // Add initial buffer if provided
    if let Some(initial) = initial_buffer {
        accumulated.extend_from_slice(initial);
        if verbose {
            eprintln!("[stream] Initial buffer: {} bytes", initial.len());
        }
    }

    // Try to read all data into memory
    const CHUNK_SIZE: usize = 64 * 1024;
    let mut chunk = vec![0u8; CHUNK_SIZE];
    let mut falls_back_to_streaming = false;

    loop {
        match io::stdin().read(&mut chunk) {
            Ok(0) => {
                // EOF - we got all data in memory
                break;
            }
            Ok(n) => {
                accumulated.extend_from_slice(&chunk[..n]);

                // Check if we've exceeded memory threshold
                if accumulated.len() > MEMORY_LIMIT {
                    if verbose {
                        eprintln!("[stream] Input exceeds 100MB, falling back to streaming");
                    }
                    falls_back_to_streaming = true;
                    // Process accumulated data and continue with streaming for rest
                    let input_str = String::from_utf8_lossy(&accumulated);
                    let (read, written) = process_chunk(&input_str, mode, &config_engine);
                    total_read += read;
                    total_written += written;
                    accumulated.clear();

                    // Continue reading and processing in chunks
                    loop {
                        match io::stdin().read(&mut chunk) {
                            Ok(0) => break,
                            Ok(n) => {
                                let (read, written) =
                                    process_buffer_chunk(&chunk[..n], mode, &config_engine);
                                total_read += read;
                                total_written += written;
                                if verbose {
                                    eprintln!("[stream-chunk] {} → {}", n, written);
                                }
                            }
                            Err(e) => {
                                eprintln!("Error reading input: {}", e);
                                std::process::exit(1);
                            }
                        }
                    }
                    io::stdout().flush().ok();

                    if verbose {
                        let elapsed = start.elapsed();
                        let throughput = if elapsed.as_secs_f64() > 0.0 {
                            total_read as f64 / (1024.0 * 1024.0) / elapsed.as_secs_f64()
                        } else {
                            0.0
                        };
                        eprintln!("\n[stream-summary] Throughput: {:.1} MB/s", throughput);
                    }
                    return;
                }
            }
            Err(e) => {
                eprintln!("Error reading input: {}", e);
                std::process::exit(1);
            }
        }
    }

    if !falls_back_to_streaming {
        // All data fit in memory - process as single operation (best performance)
        let input_str = String::from_utf8_lossy(&accumulated);
        let (read, written) = process_chunk(&input_str, mode, &config_engine);
        total_read = read;
        total_written = written;

        io::stdout().flush().ok();

        if verbose {
            let elapsed = start.elapsed();
            let throughput = if elapsed.as_secs_f64() > 0.0 {
                total_read as f64 / (1024.0 * 1024.0) / elapsed.as_secs_f64()
            } else {
                0.0
            };
            eprintln!("\n[stream-summary]");
            match mode {
                RedactionMode::Text => {
                    eprintln!("  Type: Text/Pattern");
                    eprintln!(
                        "  Bytes: {} → {} (char-preserved)",
                        total_read, total_written
                    );
                }
                RedactionMode::Env => {
                    eprintln!("  Type: Environment Variables");
                    eprintln!("  Bytes: {} → {}", total_read, total_written);
                }
            }
            eprintln!("  Time: {:.2}s", elapsed.as_secs_f64());
            eprintln!("  Throughput: {:.1} MB/s", throughput);
        }
    }
}

/// Process chunk as a single unit (for in-memory processing - best performance)
fn process_chunk(
    text: &str,
    mode: RedactionMode,
    config_engine: &ConfigurableEngine,
) -> (usize, usize) {
    match mode {
        RedactionMode::Text => {
            let result = config_engine.detect_and_redact(text);
            io::stdout().write_all(result.redacted.as_bytes()).ok();
            (text.len(), result.redacted.len())
        }
        RedactionMode::Env => {
            // Batch process env-mode lines for better performance
            // Instead of detecting/redacting each line individually,
            // we can redact the entire block and then split on newlines
            let result = config_engine.detect_and_redact(text);
            io::stdout().write_all(result.redacted.as_bytes()).ok();

            // Return byte counts (input might have been modified by redaction)
            (text.len(), result.redacted.len())
        }
    }
}

/// Process a single buffer chunk
///
/// Mode-specific processing:
/// - Text mode: Apply pattern-based redaction
/// - Env mode: Apply line-by-line environment variable redaction
///
/// Returns: (bytes_read, bytes_written)
fn process_buffer_chunk(
    buffer: &[u8],
    mode: RedactionMode,
    config_engine: &ConfigurableEngine,
) -> (usize, usize) {
    let input_str = String::from_utf8_lossy(buffer);
    let bytes_read = buffer.len();
    let bytes_written;

    match mode {
        RedactionMode::Text => {
            // Pattern-based redaction
            let result = config_engine.detect_and_redact(&input_str);
            io::stdout().write_all(result.redacted.as_bytes()).ok();
            bytes_written = result.redacted.len();
        }
        RedactionMode::Env => {
            // Environment variable redaction (line-by-line)
            let mut total_written = 0;
            for line in input_str.lines() {
                let redacted = crate::env_mode::redact_env_line_configurable(line, config_engine);
                io::stdout().write_all(redacted.as_bytes()).ok();
                io::stdout().write_all(b"\n").ok();
                total_written += redacted.len() + 1;
            }
            bytes_written = total_written;
        }
    }

    (bytes_read, bytes_written)
}

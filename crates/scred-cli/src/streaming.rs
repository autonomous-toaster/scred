/// Unified Streaming Redaction Module
/// 
/// Consolidates three streaming functions (run_redacting_stream, run_env_redacting_stream,
/// process_text_chunk_and_stream) into a single, DRY implementation.
/// 
/// Eliminates 100-150 lines of duplication while maintaining identical behavior.

use std::io::{self, Read, Write};
use std::time::Instant;
use std::sync::Arc;

use scred_redactor::{RedactionEngine, RedactionConfig};
use scred_http::{ConfigurableEngine, PatternSelector};

/// Redaction mode: determines how buffers are processed
#[derive(Debug, Copy, Clone)]
pub enum RedactionMode {
    /// Process chunks as text (pattern-based redaction)
    Text,
    /// Process chunks as environment variables (line-by-line)
    Env,
}

/// Unified streaming redaction function
/// 
/// Replaces three separate functions with a single, consolidated implementation.
/// Handles text mode, environment mode, and auto-detected mode with initial buffer.
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
    let config_engine = ConfigurableEngine::new(
        engine,
        detect_selector.clone(),
        redact_selector.clone(),
    );

    let mut total_read = 0;
    let mut total_written = 0;

    // Process initial buffer if provided (from auto-detection)
    if let Some(initial) = initial_buffer {
        let (read, written) = process_buffer_chunk(initial, mode, &config_engine);
        total_read += read;
        total_written += written;
        
        if verbose {
            eprintln!("[stream] Initial buffer: {} → {}", read, written);
        }
    }

    // Continue streaming remaining chunks
    const CHUNK_SIZE: usize = 64 * 1024;
    let mut chunk = vec![0u8; CHUNK_SIZE];

    loop {
        match io::stdin().read(&mut chunk) {
            Ok(0) => break, // EOF
            Ok(n) => {
                let (read, written) = process_buffer_chunk(&chunk[..n], mode, &config_engine);
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

    // Flush any remaining buffered output
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
                eprintln!("  Bytes: {} → {} (char-preserved)", total_read, total_written);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redaction_mode_copy() {
        // Ensure RedactionMode can be copied (used in match statements)
        let mode = RedactionMode::Text;
        let _mode2 = mode; // Copy
        let _mode3 = mode; // Copy
    }
}

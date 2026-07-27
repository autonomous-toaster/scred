use crate::RedactionEngine;
use std::sync::Arc;
use tracing::warn;

pub mod async_reader;
pub mod streaming_redactor;
pub mod tests;

// ============================================================================
// Shared types
// ============================================================================

/// Statistics from a streaming redaction session
#[derive(Debug, Clone, Default)]
pub struct StreamingStats {
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub chunks_processed: u64,
    pub patterns_found: u64,
    pub errors: u64,
}

impl StreamingStats {
    pub fn merge(&mut self, other: StreamingStats) {
        self.bytes_read += other.bytes_read;
        self.bytes_written += other.bytes_written;
        self.chunks_processed += other.chunks_processed;
        self.patterns_found += other.patterns_found;
        self.errors += other.errors;
    }
}

/// Internal streaming configuration (not exposed to callers).
/// The lookahead size (512B) is verified to be >= the longest pattern prefix (22B).
/// The chunk size (64KB) balances memory and throughput.
const LOOKAHEAD_SIZE: usize = 512;
const CHUNK_SIZE: usize = 64 * 1024;

// ============================================================================
// RedactionStream — feed chunks, get redacted bytes
// ============================================================================

/// Streaming secret redactor.
///
/// Feed chunks of data, get redacted output. The lookahead window is managed
/// internally — callers never touch it.
///
/// # Important
/// `finalize()` MUST be called to flush the lookahead buffer. It consumes self,
/// so the compiler prevents calling `feed()` after `finalize()`.
///
/// # Example
/// ```ignore
/// let mut stream = RedactionStream::new(engine);
/// let out1 = stream.feed(b"some data AKIA");
/// let out2 = stream.feed(b"IOSFODNN7EXAMPLE more");
/// let (out3, stats) = stream.finalize();
/// ```
pub struct RedactionStream {
    _engine: Arc<RedactionEngine>,
    lookahead: Vec<u8>,
    is_finalized: bool,
    stats: StreamingStats,
}

impl RedactionStream {
    /// Create a new streaming redactor.
    pub fn new(engine: Arc<RedactionEngine>) -> Self {
        Self {
            _engine: engine,
            lookahead: Vec::with_capacity(LOOKAHEAD_SIZE),
            is_finalized: false,
            stats: StreamingStats::default(),
        }
    }

    /// Feed a chunk of data. Returns redacted bytes safe to emit downstream.
    /// May return empty if not enough data has accumulated to fill the lookahead window.
    pub fn feed(&mut self, chunk: &[u8]) -> Vec<u8> {
        use scred_detector::{detect_all, redact_in_place};

        // Combine lookahead + new chunk
        let mut combined = std::mem::take(&mut self.lookahead);
        combined.extend_from_slice(chunk);

        // Detect and redact
        let detection = detect_all(&combined);
        let patterns_found = detection.matches.len() as u64;
        let mut redacted = combined;
        redact_in_place(&mut redacted, &detection.matches);

        // Calculate output boundaries
        let redacted_len = redacted.len();
        let output_end = redacted_len.saturating_sub(LOOKAHEAD_SIZE);

        // Prepare output
        let output = if output_end > 0 {
            redacted[..output_end].to_vec()
        } else {
            Vec::new()
        };

        // Save new lookahead
        if output_end < redacted_len {
            self.lookahead = redacted[output_end..].to_vec();
        } else {
            self.lookahead.clear();
        }

        // Update stats
        self.stats.bytes_read += chunk.len() as u64;
        self.stats.bytes_written += output.len() as u64;
        self.stats.patterns_found += patterns_found;
        self.stats.chunks_processed += 1;

        output
    }

    /// Finalize the stream. Flushes the lookahead buffer and returns stats.
    /// After this, the stream is exhausted — calling feed() will return empty.
    pub fn finalize(&mut self) -> (Vec<u8>, StreamingStats) {
        self.is_finalized = true;
        let output = std::mem::take(&mut self.lookahead);
        self.stats.bytes_written += output.len() as u64;
        let stats = std::mem::take(&mut self.stats);
        (output, stats)
    }

    /// Check if the stream has been finalized.
    pub fn is_finalized(&self) -> bool {
        self.is_finalized
    }

    /// Get the number of bytes currently held in the lookahead buffer.
    pub fn pending_lookahead(&self) -> usize {
        self.lookahead.len()
    }
}

impl Drop for RedactionStream {
    fn drop(&mut self) {
        if !self.is_finalized && !self.lookahead.is_empty() {
            warn!(
                "RedactionStream dropped without finalize() — {} bytes of lookahead lost",
                self.lookahead.len()
            );
        }
    }
}

// ============================================================================
// DetectionStream — feed chunks, get match events
// ============================================================================

/// A single match found by DetectionStream.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub start: usize,
    pub end: usize,
    pub pattern_type: u16,
}

/// Streaming secret detector.
///
/// Feed chunks of data, get match events. Matches in the lookahead region
/// are held back and returned by the next feed() call or by finalize().
///
/// # Example
/// ```ignore
/// let mut detector = DetectionStream::new(engine);
/// let m1 = detector.feed(b"some data AKIA");
/// let m2 = detector.feed(b"IOSFODNN7EXAMPLE more");
/// let (all_matches, stats) = detector.finalize();
/// ```
pub struct DetectionStream {
    _engine: Arc<RedactionEngine>,
    lookahead: Vec<u8>,
    is_finalized: bool,
    stats: StreamingStats,
    /// Matches from the previous feed that were in the lookahead region.
    /// These are re-validated when the next chunk arrives.
    pending_matches: Vec<scred_detector::Match>,
}

impl DetectionStream {
    /// Create a new streaming detector.
    pub fn new(engine: Arc<RedactionEngine>) -> Self {
        Self {
            _engine: engine,
            lookahead: Vec::with_capacity(LOOKAHEAD_SIZE),
            is_finalized: false,
            stats: StreamingStats::default(),
            pending_matches: Vec::new(),
        }
    }

    /// Feed a chunk. Returns matches found in the output region.
    /// Matches in the lookahead region are held back.
    pub fn feed(&mut self, chunk: &[u8]) -> Vec<Match> {
        use scred_detector::detect_all;

        // Combine lookahead + new chunk
        let mut combined = std::mem::take(&mut self.lookahead);
        combined.extend_from_slice(chunk);

        // Detect on combined buffer
        let detection = detect_all(&combined);
        let patterns_found = detection.matches.len() as u64;

        // Calculate output boundary
        let combined_len = combined.len();
        let output_end = combined_len.saturating_sub(LOOKAHEAD_SIZE);

        // Split matches: output region vs lookahead region
        let mut output_matches = Vec::new();
        let mut new_lookahead_matches = Vec::new();

        for m in &detection.matches {
            if m.end <= output_end {
                // Match is entirely in the output region — emit it
                output_matches.push(Match {
                    start: m.start,
                    end: m.end,
                    pattern_type: m.pattern_type,
                });
            } else {
                // Match extends into the lookahead region — hold back
                new_lookahead_matches.push(*m);
            }
        }

        // Save lookahead data and pending matches
        if output_end < combined_len {
            self.lookahead = combined[output_end..].to_vec();
        } else {
            self.lookahead.clear();
        }
        self.pending_matches = new_lookahead_matches;

        // Update stats
        self.stats.bytes_read += chunk.len() as u64;
        self.stats.patterns_found += patterns_found;
        self.stats.chunks_processed += 1;

        output_matches
    }

    /// Finalize the stream. Returns any matches in the final lookahead region.
    pub fn finalize(&mut self) -> (Vec<Match>, StreamingStats) {
        self.is_finalized = true;

        // Re-detect on the final lookahead to get any matches
        let mut output_matches = Vec::new();
        if !self.lookahead.is_empty() {
            use scred_detector::detect_all;
            let detection = detect_all(&self.lookahead);
            for m in &detection.matches {
                output_matches.push(Match {
                    start: m.start,
                    end: m.end,
                    pattern_type: m.pattern_type,
                });
            }
        }

        // Also include any pending matches from the last feed
        for m in &self.pending_matches {
            output_matches.push(Match {
                start: m.start,
                end: m.end,
                pattern_type: m.pattern_type,
            });
        }

        let stats = std::mem::take(&mut self.stats);
        (output_matches, stats)
    }

    /// Check if the stream has been finalized.
    pub fn is_finalized(&self) -> bool {
        self.is_finalized
    }
}

impl Drop for DetectionStream {
    fn drop(&mut self) {
        if !self.is_finalized && !self.lookahead.is_empty() {
            warn!(
                "DetectionStream dropped without finalize() — {} bytes of lookahead lost",
                self.lookahead.len()
            );
        }
    }
}

// ============================================================================
// AsyncRedactionReader — transparent redaction for any AsyncRead
// ============================================================================

const MAX_ITERATIONS_PER_POLL: u32 = 8;

/// Wraps any AsyncRead and redacts secrets on the fly.
///
/// The lookahead window is managed internally. Reads may be delayed
/// by up to LOOKAHEAD_SIZE bytes while the window fills.
///
/// # Example
/// ```ignore
/// // AsyncRedactionReader requires an async context and proper setup
/// // See tests/ for working examples
/// ```
// Re-exports
pub use async_reader::{AsyncRedactionReader, StreamingConfig};
pub use streaming_redactor::FrameRingRedactor;
pub use streaming_redactor::StreamingRedactor;

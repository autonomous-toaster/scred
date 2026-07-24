use crate::RedactionEngine;
use std::sync::Arc;
use tracing::warn;

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
    engine: Arc<RedactionEngine>,
    lookahead: Vec<u8>,
    is_finalized: bool,
    stats: StreamingStats,
}

impl RedactionStream {
    /// Create a new streaming redactor.
    pub fn new(engine: Arc<RedactionEngine>) -> Self {
        Self {
            engine,
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
    engine: Arc<RedactionEngine>,
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
            engine,
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
        let output_end = if combined_len > LOOKAHEAD_SIZE {
            combined_len - LOOKAHEAD_SIZE
        } else {
            0
        };

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
/// let mut reader = AsyncRedactionReader::new(tcp_stream, engine);
/// tokio::io::copy(&mut reader, &mut output).await?;
/// ```
pub struct AsyncRedactionReader<R> {
    inner: Option<R>,
    stream: RedactionStream,
    read_buf: Vec<u8>,
    output_buf: Vec<u8>,
    output_pos: usize,
}

impl<R> AsyncRedactionReader<R> {
    /// Create a new AsyncRedactionReader wrapping an AsyncRead source.
    pub fn new(inner: R, engine: Arc<RedactionEngine>) -> Self {
        Self {
            inner: Some(inner),
            stream: RedactionStream::new(engine),
            read_buf: vec![0u8; CHUNK_SIZE],
            output_buf: Vec::new(),
            output_pos: 0,
        }
    }

    /// Get a reference to the inner reader.
    pub fn inner(&self) -> &R {
        match self.inner.as_ref() {
            Some(r) => r,
            None => unreachable!("AsyncRedactionReader inner should always be Some"),
        }
    }

    /// Get a mutable reference to the inner reader.
    pub fn inner_mut(&mut self) -> &mut R {
        match self.inner.as_mut() {
            Some(r) => r,
            None => unreachable!("AsyncRedactionReader inner should always be Some"),
        }
    }

    /// Consume the reader and return the inner reader and any unread redacted data.
    pub fn into_inner(mut self) -> (R, Vec<u8>) {
        let inner = match self.inner.take() {
            Some(r) => r,
            None => unreachable!("AsyncRedactionReader inner should always be Some"),
        };
        // Drain remaining output buffer
        let mut remaining = Vec::new();
        if self.output_pos < self.output_buf.len() {
            remaining.extend_from_slice(&self.output_buf[self.output_pos..]);
        }
        // Finalize to get lookahead
        let (lookahead, _) = self.stream.finalize();
        remaining.extend_from_slice(&lookahead);
        (inner, remaining)
    }
}

impl<R: tokio::io::AsyncRead + Unpin> tokio::io::AsyncRead for AsyncRedactionReader<R> {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        use std::pin::Pin;
        use std::task::Poll;

        let this = self.get_mut();

        // Phase 1: Drain buffered output from previous calls
        if this.output_pos < this.output_buf.len() {
            let available = this.output_buf.len() - this.output_pos;
            let to_copy = std::cmp::min(available, buf.remaining());
            buf.put_slice(&this.output_buf[this.output_pos..this.output_pos + to_copy]);
            this.output_pos += to_copy;
            if this.output_pos == this.output_buf.len() {
                this.output_buf.clear();
                this.output_pos = 0;
            }
            return Poll::Ready(Ok(()));
        }

        // Phase 2: Read from inner until we have output or inner is exhausted
        let mut iterations = 0u32;
        loop {
            iterations += 1;
            if iterations > MAX_ITERATIONS_PER_POLL {
                // Safety valve: yield to executor.
                // Prevents starvation if inner always has data but
                // the stream hasn't accumulated enough for output.
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }

            let mut inner_buf = tokio::io::ReadBuf::new(&mut this.read_buf);
            let inner_pin = match this.inner.as_mut() {
                Some(r) => r,
                None => unreachable!("AsyncRedactionReader inner should always be Some"),
            };
            match Pin::new(inner_pin).poll_read(cx, &mut inner_buf) {
                Poll::Ready(Ok(())) => {
                    let n = inner_buf.filled().len();
                    if n == 0 {
                        // EOF — finalize the stream, flush lookahead
                        let (final_output, _stats) = this.stream.finalize();
                        return Self::put_output(this, buf, final_output);
                    }
                    let output = this.stream.feed(&this.read_buf[..n]);
                    if !output.is_empty() {
                        return Self::put_output(this, buf, output);
                    }
                    // No output — all held as lookahead. Loop to read more.
                }
                Poll::Pending => {
                    // Inner has no data. Waker is registered — we'll be
                    // polled again when data arrives.
                    return Poll::Pending;
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
            }
        }
    }
}

impl<R> AsyncRedactionReader<R> {
    /// Place output into the consumer's buffer, buffering any overflow.
    fn put_output(
        this: &mut Self,
        buf: &mut tokio::io::ReadBuf<'_>,
        output: Vec<u8>,
    ) -> std::task::Poll<std::io::Result<()>> {
        use std::task::Poll;

        if output.is_empty() {
            return Poll::Ready(Ok(()));
        }
        let to_copy = std::cmp::min(output.len(), buf.remaining());
        buf.put_slice(&output[..to_copy]);
        if to_copy < output.len() {
            this.output_buf = output;
            this.output_pos = to_copy;
        }
        Poll::Ready(Ok(()))
    }
}

// ============================================================================
// Pipe — read → redact → write
// ============================================================================

impl RedactionStream {
    /// Read from an AsyncRead source, redact through this stream, and write to an AsyncWrite destination.
    /// Returns stats when the source is exhausted.
    pub async fn pipe<R, W>(
        engine: Arc<RedactionEngine>,
        reader: &mut R,
        writer: &mut W,
    ) -> std::io::Result<StreamingStats>
    where
        R: tokio::io::AsyncRead + Unpin,
        W: tokio::io::AsyncWrite + Unpin,
    {
        use tokio::io::AsyncReadExt;
        use tokio::io::AsyncWriteExt;

        let mut stream = Self::new(engine);
        let mut buf = vec![0u8; CHUNK_SIZE];
        let mut stats = StreamingStats::default();

        loop {
            let n = reader.read(&mut buf).await?;
            if n == 0 {
                let (output, final_stats) = stream.finalize();
                if !output.is_empty() {
                    writer.write_all(&output).await?;
                }
                stats.merge(final_stats);
                return Ok(stats);
            }
            let output = stream.feed(&buf[..n]);
            stats.bytes_read += n as u64;
            stats.bytes_written += output.len() as u64;
            if !output.is_empty() {
                writer.write_all(&output).await?;
            }
            stats.chunks_processed += 1;
        }
    }
}

// ============================================================================
// Legacy API — kept for backward compatibility
// ============================================================================

/// Configuration for streaming redaction
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    pub chunk_size: usize,
    pub lookahead_size: usize,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            chunk_size: CHUNK_SIZE,
            lookahead_size: LOOKAHEAD_SIZE,
        }
    }
}

/// Legacy streaming redactor. Prefer `RedactionStream` for new code.
pub struct StreamingRedactor {
    engine: Arc<RedactionEngine>,
    config: StreamingConfig,
    selector: Option<crate::pattern_selector::PatternSelector>,
    buffer_pool: crate::buffer_pool::BufferPool,
}

impl StreamingRedactor {
    pub fn new(engine: Arc<RedactionEngine>, config: StreamingConfig) -> Self {
        Self {
            engine,
            config,
            selector: None,
            buffer_pool: crate::buffer_pool::BufferPool::with_defaults(),
        }
    }

    pub fn with_selector(
        engine: Arc<RedactionEngine>,
        config: StreamingConfig,
        selector: crate::pattern_selector::PatternSelector,
    ) -> Self {
        Self {
            engine,
            config,
            selector: Some(selector),
            buffer_pool: crate::buffer_pool::BufferPool::with_defaults(),
        }
    }

    pub fn has_selector(&self) -> bool {
        self.selector.is_some()
    }

    pub fn get_selector(&self) -> Option<&crate::pattern_selector::PatternSelector> {
        self.selector.as_ref()
    }

    pub fn engine(&self) -> &Arc<RedactionEngine> {
        &self.engine
    }

    pub fn with_defaults(engine: Arc<RedactionEngine>) -> Self {
        Self::new(engine, StreamingConfig::default())
    }

    #[inline]
    pub fn config(&self) -> &StreamingConfig {
        &self.config
    }

    pub fn buffer_pool_mut(&mut self) -> &mut crate::buffer_pool::BufferPool {
        &mut self.buffer_pool
    }

    /// Process a chunk of data with lookahead buffer management.
    /// Prefer `RedactionStream` for new code.
    #[inline]
    pub fn process_chunk(
        &self,
        chunk: &[u8],
        lookahead: &mut Vec<u8>,
        is_eof: bool,
    ) -> (String, u64, u64) {
        use scred_detector::detect_all;

        let mut combined = std::mem::take(lookahead);
        combined.extend_from_slice(chunk);

        let detection = detect_all(&combined);
        let patterns_found = detection.matches.len() as u64;

        let mut redacted = combined;
        scred_detector::redact_in_place(&mut redacted, &detection.matches);

        let redacted_str = String::from_utf8_lossy(&redacted);
        let redacted_len = redacted_str.len();
        let output_end = if is_eof {
            redacted_len
        } else if redacted_len > self.config.lookahead_size {
            redacted_len - self.config.lookahead_size
        } else {
            0
        };

        let output_text = if output_end > 0 {
            redacted_str[..output_end].to_string()
        } else {
            String::new()
        };

        if !is_eof && output_end < redacted_len {
            *lookahead = redacted_str[output_end..].as_bytes().to_vec();
        } else {
            lookahead.clear();
        }

        let bytes_written = output_text.len() as u64;
        (output_text, bytes_written, patterns_found)
    }

    /// Byte-preserving variant. Prefer `RedactionStream` for new code.
    pub fn process_chunk_bytes(
        &self,
        chunk: &[u8],
        lookahead: &mut Vec<u8>,
        is_eof: bool,
    ) -> (Vec<u8>, u64, u64) {
        use scred_detector::detect_all;

        let mut combined = std::mem::take(lookahead);
        combined.extend_from_slice(chunk);

        let detection = detect_all(&combined);
        let patterns_found = detection.matches.len() as u64;

        let mut redacted = combined;
        scred_detector::redact_in_place(&mut redacted, &detection.matches);

        let redacted_len = redacted.len();
        let output_end = if is_eof {
            redacted_len
        } else if redacted_len > self.config.lookahead_size {
            redacted_len - self.config.lookahead_size
        } else {
            0
        };

        let output = if output_end > 0 {
            redacted[..output_end].to_vec()
        } else {
            Vec::new()
        };

        if !is_eof && output_end < redacted_len {
            *lookahead = redacted[output_end..].to_vec();
        } else {
            lookahead.clear();
        }

        let bytes_written = output.len() as u64;
        (output, bytes_written, patterns_found)
    }

    /// Redact reader to writer using byte-preserving streaming.
    pub fn redact_reader_to_writer<R: std::io::Read, W: std::io::Write>(
        &self,
        reader: &mut R,
        writer: &mut W,
    ) -> std::io::Result<StreamingStats> {
        let mut stats = StreamingStats::default();
        let mut lookahead = Vec::with_capacity(self.config.lookahead_size);
        let mut buf = vec![0u8; self.config.chunk_size];

        loop {
            let n = reader.read(&mut buf)?;
            if n == 0 {
                let (out, bytes_written, patterns_found) =
                    self.process_chunk_bytes(&[], &mut lookahead, true);
                if !out.is_empty() {
                    writer.write_all(&out)?;
                }
                stats.bytes_written += bytes_written;
                stats.patterns_found += patterns_found;
                break;
            }

            let (out, bytes_written, patterns_found) =
                self.process_chunk_bytes(&buf[..n], &mut lookahead, false);
            if !out.is_empty() {
                writer.write_all(&out)?;
            }

            stats.bytes_read += n as u64;
            stats.bytes_written += bytes_written;
            stats.patterns_found += patterns_found;
            stats.chunks_processed += 1;
        }

        Ok(stats)
    }

    pub fn redact_buffer(&self, data: &[u8]) -> (String, StreamingStats) {
        self.redact_buffer_in_place(data, false)
    }

    pub fn redact_buffer_copy_based(&self, data: &[u8]) -> (String, StreamingStats) {
        let mut stats = StreamingStats::default();
        let mut lookahead = Vec::with_capacity(self.config.lookahead_size);
        let mut output = String::new();

        for chunk in data.chunks(self.config.chunk_size) {
            let is_eof = chunk.len() < self.config.chunk_size;
            let (chunk_output, bytes_written, patterns) =
                self.process_chunk(chunk, &mut lookahead, is_eof);

            output.push_str(&chunk_output);
            stats.bytes_read += chunk.len() as u64;
            stats.bytes_written += bytes_written;
            stats.patterns_found += patterns;
            stats.chunks_processed += 1;
        }

        (output, stats)
    }

    pub fn redact_buffer_in_place(
        &self,
        data: &[u8],
        _use_copy_based: bool,
    ) -> (String, StreamingStats) {
        let mut stats = StreamingStats::default();
        let mut lookahead = Vec::with_capacity(self.config.lookahead_size);
        let mut output = String::new();

        for chunk in data.chunks(self.config.chunk_size) {
            let is_eof = chunk.len() < self.config.chunk_size;
            let (chunk_output, bytes_written, patterns) =
                self.process_chunk_in_place(chunk, &mut lookahead, is_eof);

            output.push_str(&chunk_output);
            stats.bytes_read += chunk.len() as u64;
            stats.bytes_written += bytes_written;
            stats.patterns_found += patterns;
            stats.chunks_processed += 1;
        }

        (output, stats)
    }

    pub fn redact_buffer_bytes(&self, data: &[u8]) -> (Vec<u8>, StreamingStats) {
        let mut stats = StreamingStats::default();
        let mut lookahead = Vec::with_capacity(self.config.lookahead_size);
        let mut output = Vec::with_capacity(data.len());

        for (i, chunk) in data.chunks(self.config.chunk_size).enumerate() {
            let is_eof = (i + 1) * self.config.chunk_size >= data.len();
            let (chunk_output, bytes_written, patterns) =
                self.process_chunk_bytes(chunk, &mut lookahead, is_eof);

            output.extend_from_slice(&chunk_output);
            stats.bytes_read += chunk.len() as u64;
            stats.bytes_written += bytes_written;
            stats.patterns_found += patterns;
            stats.chunks_processed += 1;
        }

        if data.is_empty() {
            let (chunk_output, bytes_written, patterns) =
                self.process_chunk_bytes(&[], &mut lookahead, true);
            output.extend_from_slice(&chunk_output);
            stats.bytes_written += bytes_written;
            stats.patterns_found += patterns;
        }

        (output, stats)
    }

    pub fn process_chunk_in_place(
        &self,
        chunk: &[u8],
        lookahead: &mut Vec<u8>,
        is_eof: bool,
    ) -> (String, u64, u64) {
        let mut combined = std::mem::take(lookahead);
        combined.extend_from_slice(chunk);

        use scred_detector::detect_all;
        let detection = detect_all(&combined);
        let patterns_found = detection.matches.len() as u64;

        let mut redacted = combined;
        scred_detector::redact_in_place(&mut redacted, &detection.matches);

        let redacted_str = String::from_utf8_lossy(&redacted).into_owned();

        let output_end = if is_eof {
            redacted_str.len()
        } else if redacted_str.len() > self.config.lookahead_size {
            redacted_str.len() - self.config.lookahead_size
        } else {
            0
        };

        let output = redacted_str.clone();
        let output_text = if output_end > 0 {
            output[..output_end].to_string()
        } else {
            String::new()
        };

        if !is_eof && output_end < output.len() {
            *lookahead = output[output_end..].as_bytes().to_vec();
        } else {
            lookahead.clear();
        }

        let bytes_written = output_text.len() as u64;
        (output_text, bytes_written, patterns_found)
    }
}

/// Frame-Ring-optimized streaming redactor. Prefer `RedactionStream` for new code.
pub struct FrameRingRedactor {
    engine: Arc<RedactionEngine>,
    config: StreamingConfig,
    ring: crate::frame_ring::FrameRing<65536, 3>,
}

impl FrameRingRedactor {
    pub fn new(engine: Arc<RedactionEngine>, config: StreamingConfig) -> Self {
        use crate::frame_ring::FrameRing;
        Self {
            engine,
            config,
            ring: FrameRing::new(),
        }
    }

    pub fn with_defaults(engine: Arc<RedactionEngine>) -> Self {
        Self::new(engine, StreamingConfig::default())
    }

    pub fn process_chunk(&mut self, chunk: &[u8], is_eof: bool) -> (String, u64) {
        let read_frame = self.ring.get_read_frame();
        read_frame.clear();
        read_frame.extend_from_slice(chunk);
        self.ring.mark_ready_and_rotate_read();

        let process_frame = self.ring.get_process_frame();

        use scred_detector::{detect_all, redact_in_place};
        let detection = detect_all(process_frame);
        let patterns_found = detection.matches.len() as u64;

        let mut redacted = process_frame.to_vec();
        redact_in_place(&mut redacted, &detection.matches);
        let output = String::from_utf8_lossy(&redacted).into_owned();

        self.ring.mark_process_done_and_rotate();

        let output_end = if is_eof {
            output.len()
        } else if output.len() > self.config.lookahead_size {
            output.len() - self.config.lookahead_size
        } else {
            0
        };

        let output_text = if output_end > 0 {
            output[..output_end].to_string()
        } else {
            String::new()
        };

        self.ring.mark_written_and_rotate();

        (output_text, patterns_found)
    }

    pub fn redact_buffer(&mut self, data: &[u8]) -> (String, StreamingStats) {
        let mut stats = StreamingStats::default();
        let mut output = String::new();

        for chunk in data.chunks(self.config.chunk_size) {
            let is_eof = chunk.len() < self.config.chunk_size;
            let (chunk_output, patterns) = self.process_chunk(chunk, is_eof);

            output.push_str(&chunk_output);
            stats.bytes_read += chunk.len() as u64;
            stats.bytes_written += chunk_output.len() as u64;
            stats.patterns_found += patterns;
            stats.chunks_processed += 1;
        }

        (output, stats)
    }

    pub fn config(&self) -> &StreamingConfig {
        &self.config
    }

    pub fn engine(&self) -> &Arc<RedactionEngine> {
        &self.engine
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]
    use super::*;
    use crate::{RedactionConfig, RedactionEngine};

    fn test_engine() -> Arc<RedactionEngine> {
        Arc::new(RedactionEngine::new(RedactionConfig { enabled: true }))
    }

    // ========================================================================
    // RedactionStream tests
    // ========================================================================

    #[test]
    fn test_redaction_stream_empty() {
        let mut stream = RedactionStream::new(test_engine());
        let out = stream.feed(b"");
        assert!(out.is_empty());
        let (final_out, stats) = stream.finalize();
        assert!(final_out.is_empty());
        assert_eq!(stats.bytes_read, 0);
    }

    #[test]
    fn test_redaction_stream_no_secrets() {
        let mut stream = RedactionStream::new(test_engine());
        // Data < 512B is held in lookahead, returned by finalize
        let out = stream.feed(b"hello world this is plain text");
        assert!(out.is_empty(), "small data held in lookahead");
        let (final_out, stats) = stream.finalize();
        assert_eq!(final_out.len(), 30);
        assert_eq!(stats.patterns_found, 0);
    }

    #[test]
    fn test_redaction_stream_single_secret() {
        let mut stream = RedactionStream::new(test_engine());
        // Data < 512B is held in lookahead
        let out = stream.feed(b"AKIAIOSFODNN7EXAMPLE");
        assert!(out.is_empty(), "small data held in lookahead");
        let (final_out, _) = stream.finalize();
        assert_eq!(String::from_utf8_lossy(&final_out), "AKIAxxxxxxxxxxxxxxxx");
    }

    #[test]
    fn test_redaction_stream_secret_spanning_chunks() {
        let mut stream = RedactionStream::new(test_engine());

        // First chunk: 400 bytes of padding (not 'A' to avoid false prefix match) + prefix
        let mut chunk1 = vec![b'X'; 400];
        chunk1.extend_from_slice(b"some data AKIA");
        let out1 = stream.feed(&chunk1);
        // 413 < 512, all held in lookahead
        assert!(out1.is_empty(), "413B < 512B lookahead");

        // Second chunk: rest of the secret + padding to exceed 512
        let mut chunk2 = b"IOSFODNN7EXAMPLE more data"[..].to_vec();
        chunk2.extend_from_slice(&vec![b'Y'; 200]);
        let out2 = stream.feed(&chunk2);
        // Combined = 413 + 219 = 632 > 512, output is first 120 bytes (all X's)
        // The redacted secret is in the lookahead (bytes 120-631)
        assert_eq!(out2.len(), 128, "output should be 640-512=128 bytes");

        let (final_out, stats) = stream.finalize();
        let final_str = String::from_utf8_lossy(&final_out);
        assert!(final_str.contains("AKIAxxxxxxxxxxxxxxxx"), "final: {}", final_str);
        assert!(stats.patterns_found > 0);
    }

    #[test]
    fn test_redaction_stream_multiple_chunks() {
        let mut stream = RedactionStream::new(test_engine());

        // Feed enough data to exceed lookahead
        let prefix = vec![b'A'; 400]; // 400 bytes of padding
        let out1 = stream.feed(&prefix);
        assert!(out1.is_empty(), "400B < 512B lookahead");

        let out2 = stream.feed(b"AKIAIOSFODNN7EXAMPLE");
        // Combined = 400 + 20 = 420, still < 512
        assert!(out2.is_empty(), "420B < 512B lookahead");

        let out3 = stream.feed(b" and more data here");
        // Combined = 420 + 19 = 439, still < 512
        assert!(out3.is_empty(), "439B < 512B lookahead");

        let out4 = stream.feed(b"X"); // 440 total
        // Still < 512
        assert!(out4.is_empty());

        let (final_out, stats) = stream.finalize();
        let final_str = String::from_utf8_lossy(&final_out);
        assert!(final_str.contains("AKIAxxxxxxxxxxxxxxxx"), "final: {}", final_str);
        assert!(stats.patterns_found > 0);
    }

    #[test]
    fn test_redaction_stream_finalize_empty() {
        let mut stream = RedactionStream::new(test_engine());
        let (out, stats) = stream.finalize();
        assert!(out.is_empty());
        assert_eq!(stats.bytes_read, 0);
    }

    #[test]
    fn test_redaction_stream_large_data() {
        let mut stream = RedactionStream::new(test_engine());
        // Feed enough data to fill the lookahead and produce output
        let data = vec![b'A'; LOOKAHEAD_SIZE + 100];
        let out = stream.feed(&data);
        // Should produce output (data > lookahead)
        assert!(!out.is_empty());
        assert_eq!(out.len(), 100); // 100 bytes past lookahead
        let (final_out, _) = stream.finalize();
        assert_eq!(final_out.len(), LOOKAHEAD_SIZE);
    }

    // ========================================================================
    // DetectionStream tests
    // ========================================================================

    #[test]
    fn test_detection_stream_empty() {
        let mut detector = DetectionStream::new(test_engine());
        let matches = detector.feed(b"");
        assert!(matches.is_empty());
        let (final_matches, _) = detector.finalize();
        assert!(final_matches.is_empty());
    }

    #[test]
    fn test_detection_stream_no_secrets() {
        let mut detector = DetectionStream::new(test_engine());
        let matches = detector.feed(b"hello world");
        assert!(matches.is_empty());
        let (final_matches, _) = detector.finalize();
        assert!(final_matches.is_empty());
    }

    #[test]
    fn test_detection_stream_single_secret() {
        let mut detector = DetectionStream::new(test_engine());
        // Data < 512B is held in lookahead
        let matches = detector.feed(b"AKIAIOSFODNN7EXAMPLE");
        assert!(matches.is_empty(), "small data held in lookahead");
        let (final_matches, _) = detector.finalize();
        assert!(!final_matches.is_empty(), "should detect AWS key on finalize");
    }

    #[test]
    fn test_detection_stream_secret_spanning_chunks() {
        let mut detector = DetectionStream::new(test_engine());

        // First chunk: padding + prefix at the end
        let mut chunk1 = vec![b'X'; 400];
        chunk1.extend_from_slice(b"some data AKIA");
        let matches1 = detector.feed(&chunk1);
        assert!(matches1.is_empty(), "prefix in lookahead should not match yet");

        // Second chunk: rest of the secret
        let mut chunk2 = b"IOSFODNN7EXAMPLE more data"[..].to_vec();
        chunk2.extend_from_slice(&vec![b'Y'; 200]);
        let matches2 = detector.feed(&chunk2);
        // Matches in the output region (first 120 bytes) — none, they're in lookahead
        // The match will be returned by finalize()

        let (final_matches, _) = detector.finalize();
        assert!(!final_matches.is_empty(), "should detect spanning secret on finalize");
    }

    // ========================================================================
    // AsyncRedactionReader tests (sync mock)
    // ========================================================================

    /// A simple sync reader that implements AsyncRead via a cursor
    struct SyncReader {
        data: Vec<u8>,
        pos: usize,
    }

    impl SyncReader {
        fn new(data: &[u8]) -> Self {
            Self {
                data: data.to_vec(),
                pos: 0,
            }
        }
    }

    impl tokio::io::AsyncRead for SyncReader {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            use std::task::Poll;
            let this = self.get_mut();
            let remaining = this.data.len() - this.pos;
            if remaining == 0 {
                return Poll::Ready(Ok(()));
            }
            let to_copy = std::cmp::min(remaining, buf.remaining());
            buf.put_slice(&this.data[this.pos..this.pos + to_copy]);
            this.pos += to_copy;
            Poll::Ready(Ok(()))
        }
    }

    #[tokio::test]
    async fn test_async_reader_basic() {
        let engine = test_engine();
        let data = b"hello AKIAIOSFODNN7EXAMPLE world";
        let reader = SyncReader::new(data);
        let mut redacted_reader = AsyncRedactionReader::new(reader, engine);

        let mut output = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut redacted_reader, &mut output)
            .await
            .unwrap();

        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("hello "), "output: {}", output_str);
        assert!(output_str.contains("AKIAxxxxxxxxxxxxxxxx"), "output: {}", output_str);
        assert!(output_str.contains(" world"), "output: {}", output_str);
    }

    #[tokio::test]
    async fn test_async_reader_empty() {
        let engine = test_engine();
        let reader = SyncReader::new(b"");
        let mut redacted_reader = AsyncRedactionReader::new(reader, engine);

        let mut output = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut redacted_reader, &mut output)
            .await
            .unwrap();

        assert!(output.is_empty());
    }

    #[tokio::test]
    async fn test_async_reader_partial_reads() {
        let engine = test_engine();
        let data = b"AKIAIOSFODNN7EXAMPLE";
        let reader = SyncReader::new(data);
        let mut redacted_reader = AsyncRedactionReader::new(reader, engine);

        // Read 1 byte at a time
        let mut output = Vec::new();
        let mut buf = [0u8; 1];
        loop {
            let n = tokio::io::AsyncReadExt::read(&mut redacted_reader, &mut buf)
                .await
                .unwrap();
            if n == 0 {
                break;
            }
            output.push(buf[0]);
        }

        let output_str = String::from_utf8_lossy(&output);
        assert_eq!(output_str, "AKIAxxxxxxxxxxxxxxxx");
    }

    #[tokio::test]
    async fn test_async_reader_into_inner() {
        let engine = test_engine();
        let data = b"hello world";
        let reader = SyncReader::new(data);
        let redacted_reader = AsyncRedactionReader::new(reader, engine);

        // into_inner should return the inner reader without panicking
        let (_inner, remaining) = redacted_reader.into_inner();
        // Data was never fed to the stream (no poll_read calls),
        // so remaining should be empty
        assert!(remaining.is_empty());
    }

    // ========================================================================
    // Pipe tests
    // ========================================================================

    #[tokio::test]
    async fn test_pipe_basic() {
        let engine = test_engine();
        let input = b"hello AKIAIOSFODNN7EXAMPLE world";
        let mut reader = &input[..];
        let mut output = Vec::new();
        let stats = RedactionStream::pipe(
            engine,
            &mut reader,
            &mut output,
        )
        .await
        .unwrap();

        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("AKIAxxxxxxxxxxxxxxxx"), "output: {}", output_str);
        assert!(stats.patterns_found > 0);
        assert!(stats.bytes_read > 0);
    }
}

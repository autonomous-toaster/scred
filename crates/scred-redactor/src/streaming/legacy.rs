use super::*;

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
        } else {
            redacted_len.saturating_sub(self.config.lookahead_size)
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
        } else {
            redacted_len.saturating_sub(self.config.lookahead_size)
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
            *lookahead = output.as_bytes()[output_end..].to_vec();
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

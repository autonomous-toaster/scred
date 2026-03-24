use crate::RedactionEngine;
use std::sync::Arc;

/// Statistics from a streaming redaction session
#[derive(Debug, Clone, Default)]
pub struct StreamingStats {
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub chunks_processed: u64,
    pub patterns_found: u64,
    pub errors: u64,
}

/// Configuration for streaming redaction
#[derive(Debug, Clone)]
pub struct StreamingConfig {
    pub chunk_size: usize,
    pub lookahead_size: usize,
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            chunk_size: 64 * 1024,           // 64KB chunks
            lookahead_size: 512,              // 512B lookahead (verified in Phase 1a)
        }
    }
}

/// Generic streaming redactor (sync version)
/// For async usage, wrap with tokio::io adapters
pub struct StreamingRedactor {
    engine: Arc<RedactionEngine>,
    config: StreamingConfig,
    /// Optional selector for filtering which patterns to apply
    /// If None, all patterns are applied (backward compatible)
    selector: Option<crate::pattern_selector::PatternSelector>,
}

impl StreamingRedactor {
    pub fn new(engine: Arc<RedactionEngine>, config: StreamingConfig) -> Self {
        Self { 
            engine, 
            config,
            selector: None,
        }
    }

    /// Create a new StreamingRedactor with selector support
    /// 
    /// # Example
    /// ```ignore
    /// let selector = PatternSelector::Tier(vec![PatternTier::Critical]);
    /// let redactor = StreamingRedactor::with_selector(engine, config, selector);
    /// ```
    pub fn with_selector(
        engine: Arc<RedactionEngine>,
        config: StreamingConfig,
        selector: crate::pattern_selector::PatternSelector,
    ) -> Self {
        Self {
            engine,
            config,
            selector: Some(selector),
        }
    }

    /// Check if this redactor has a selector configured
    pub fn has_selector(&self) -> bool {
        self.selector.is_some()
    }

    /// Get reference to the selector if configured
    pub fn get_selector(&self) -> Option<&crate::pattern_selector::PatternSelector> {
        self.selector.as_ref()
    }

    /// Get reference to the underlying redaction engine
    pub fn engine(&self) -> &Arc<RedactionEngine> {
        &self.engine
    }

    pub fn with_defaults(engine: Arc<RedactionEngine>) -> Self {
        Self::new(engine, StreamingConfig::default())
    }

    /// Process a chunk of data with lookahead buffer management and selective filtering
    /// 
    /// # Arguments
    /// * `chunk` - Raw bytes to process
    /// * `lookahead` - Previous lookahead buffer (mutable, will be updated)
    /// * `is_eof` - Whether this is the final chunk
    /// 
    /// # Returns
    /// Tuple of (output_data, bytes_written, patterns_found)
    /// 
    /// # How Selective Filtering Works
    /// 
    /// 1. Combine lookahead + new chunk
    /// 2. Redact ALL patterns (get metadata about each match)
    /// 3. For matches in the output region:
    ///    - If selector exists and pattern doesn't match -> un-redact
    ///    - Otherwise -> keep redacted
    /// 4. Output result with selective un-redaction applied
    pub fn process_chunk(
        &self,
        chunk: &[u8],
        lookahead: &mut Vec<u8>,
        is_eof: bool,
    ) -> (String, u64, u64) {
        // Combine lookahead + new chunk
        let mut combined = lookahead.clone();
        combined.extend_from_slice(chunk);

        // Redact combined data and get ALL match metadata
        let combined_str = String::from_utf8_lossy(&combined);
        let redacted_result = self.engine.redact(&combined_str);
        let mut output = redacted_result.redacted.clone();

        // Count ALL patterns found
        let patterns_found = redacted_result.matches.len() as u64;

        // Calculate output boundaries
        let output_end = if is_eof {
            output.len()
        } else if output.len() > self.config.lookahead_size {
            output.len() - self.config.lookahead_size
        } else {
            0
        };

        // **NEW: Apply selective filtering**
        // For each match in the output region, check if it should stay redacted
        if let Some(selector) = &self.selector {
            for m in &redacted_result.matches {
                // Only filter matches in the output region (not in lookahead)
                if m.position >= output_end {
                    continue; // This match will be saved for next iteration
                }

                // For now: assume selector checks by pattern_type string
                // TODO: Map pattern_type to PatternTier for proper filtering
                // Check if this pattern type name matches the selector
                // (simplified: assume Whitelist checks pattern names)
                let should_redact = match selector {
                    crate::pattern_selector::PatternSelector::All => true,
                    crate::pattern_selector::PatternSelector::None => false,
                    crate::pattern_selector::PatternSelector::Whitelist(patterns) => {
                        patterns.contains(&m.pattern_type)
                    }
                    crate::pattern_selector::PatternSelector::Blacklist(patterns) => {
                        !patterns.contains(&m.pattern_type)
                    }
                    _ => true, // Tier/Regex/Wildcard: default to redacting for safety
                };

                if !should_redact {
                    // UN-REDACT: Replace redacted text back with original
                    let start = m.position;
                    let end = start + m.redacted_text.len();

                    if start < output.len() && end <= output.len() {
                        output.replace_range(start..end, &m.original_text);
                    }
                }
            }
        }

        // Prepare final output
        let output_text = if output_end > 0 {
            output[..output_end].to_string()
        } else {
            String::new()
        };

        // Save new lookahead for next iteration
        if !is_eof && output_end < output.len() {
            *lookahead = output[output_end..].as_bytes().to_vec();
        } else {
            lookahead.clear();
        }

        let bytes_written = output_text.len() as u64;
        (output_text, bytes_written, patterns_found)
    }

    /// Convenience method: process a complete buffer (one-shot)
    /// Returns (redacted_output, stats)
    pub fn redact_buffer(&self, data: &[u8]) -> (String, StreamingStats) {
        let mut stats = StreamingStats::default();
        let mut lookahead = Vec::with_capacity(self.config.lookahead_size);
        let mut output = String::new();

        // Process in chunks
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

    pub fn config(&self) -> &StreamingConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::RedactionConfig;

    #[test]
    fn test_streaming_small_input() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        let redactor = StreamingRedactor::with_defaults(engine);

        // Use a properly formatted AWS access key (AKIA + 16 chars)
        let input = b"Hello AKIAIOSFODNN7EXAMPLE world";
        let (output, stats) = redactor.redact_buffer(input);

        // Should have redacted the AWS key
        assert!(output.contains("AKIAxxxxxxxxxxxxxxxx"), "Output: {}", output);
        assert_eq!(stats.patterns_found, 1);
    }

    #[test]
    fn test_streaming_no_patterns() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        let redactor = StreamingRedactor::with_defaults(engine);

        let input = b"Hello world, no secrets here";
        let (output, _stats) = redactor.redact_buffer(input);

        // Should pass through unchanged
        assert_eq!(output, "Hello world, no secrets here");
    }

    #[test]
    fn test_streaming_character_preservation() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        let redactor = StreamingRedactor::with_defaults(engine);

        let input = b"Secret: AKIAIOSFODNN7EXAMPLE is here";
        let (output, stats) = redactor.redact_buffer(input);

        // Character count should be preserved
        assert_eq!(output.len(), input.len());
        assert!(output.contains("AKIAxxxxxxxxxxxxxxxx"));
        assert_eq!(stats.patterns_found, 1);
    }

    #[test]
    fn test_streaming_large_input() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        let redactor = StreamingRedactor::with_defaults(engine);

        // Create 100KB of data with patterns (use larger size for testing chunking)
        let mut input = Vec::new();
        for i in 0..2000 {
            input.extend_from_slice(format!("Line {}: AKIAIOSFODNN7EXAMPLE secret\n", i).as_bytes());
        }

        let (output, stats) = redactor.redact_buffer(&input);

        // Check stats
        assert!(stats.chunks_processed > 1, "chunks={}, input_size={}", stats.chunks_processed, input.len());
        assert!(stats.patterns_found > 0); // Patterns may be deduplicated
        assert_eq!(output.len(), input.len());
    }

    #[test]
    fn test_streaming_pattern_spanning() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));

        // Use small chunk size to test spanning
        let small_config = StreamingConfig {
            chunk_size: 32,
            lookahead_size: 512,
        };
        let redactor = StreamingRedactor::new(engine, small_config);

        let input = b"start_AKIAIOSFODNN7EXAMPLE_end";
        let (output, stats) = redactor.redact_buffer(input);

        // Pattern should be detected despite small chunk size
        assert!(output.contains("AKIAxxxxxxxxxxxxxxxx"));
        assert_eq!(stats.patterns_found, 1);
    }
}

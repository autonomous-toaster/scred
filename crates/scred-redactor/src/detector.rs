/// Pattern detection events
///
/// This module provides detection event types for secret pattern matching.

#[derive(Clone, Debug)]
pub struct SecretDetectionEvent {
    /// Pattern name (e.g., "aws-access-token", "github-pat")
    pub pattern_name: String,
    /// Position in the input where the secret was found
    pub position: usize,
    /// Length of the detected secret
    pub length: usize,
}

/// Streaming detector for pattern-based secret detection
pub struct StreamingDetector {
    // Placeholder for future implementation
}

impl StreamingDetector {
    /// Create a new detector
    pub fn new() -> Result<Self, &'static str> {
        Ok(StreamingDetector {})
    }

    /// Process input and detect secrets
    pub fn process(
        &mut self,
        _input: &[u8],
        _is_eof: bool,
    ) -> Result<Vec<SecretDetectionEvent>, String> {
        Ok(Vec::new())
    }
}

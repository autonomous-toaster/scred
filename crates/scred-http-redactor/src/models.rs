//! Redaction statistics and metrics
//!
//! Tracks redaction operations and their outcomes.

use serde::{Deserialize, Serialize};

/// Statistics about a redaction operation
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RedactionStats {
    /// Number of headers redacted
    pub headers_redacted: usize,
    /// Number of sensitive patterns found and redacted
    pub patterns_found: usize,
    /// Total bytes processed
    pub bytes_processed: u64,
    /// Total bytes redacted (replaced)
    pub bytes_redacted: u64,
    /// Number of errors encountered
    pub errors: usize,
}

impl RedactionStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// Merge statistics from another operation
    pub fn merge(&mut self, other: &RedactionStats) {
        self.headers_redacted += other.headers_redacted;
        self.patterns_found += other.patterns_found;
        self.bytes_processed += other.bytes_processed;
        self.bytes_redacted += other.bytes_redacted;
        self.errors += other.errors;
    }

    /// Calculate redaction ratio
    pub fn redaction_ratio(&self) -> f64 {
        if self.bytes_processed == 0 {
            0.0
        } else {
            self.bytes_redacted as f64 / self.bytes_processed as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_merge() {
        let mut stats1 = RedactionStats::new();
        stats1.headers_redacted = 2;
        stats1.patterns_found = 5;

        let mut stats2 = RedactionStats::new();
        stats2.headers_redacted = 1;
        stats2.patterns_found = 3;

        stats1.merge(&stats2);
        assert_eq!(stats1.headers_redacted, 3);
        assert_eq!(stats1.patterns_found, 8);
    }

    #[test]
    fn test_redaction_ratio() {
        let mut stats = RedactionStats::new();
        stats.bytes_processed = 1000;
        stats.bytes_redacted = 500;

        assert_eq!(stats.redaction_ratio(), 0.5);
    }

    #[test]
    fn test_redaction_ratio_zero() {
        let stats = RedactionStats::new();
        assert_eq!(stats.redaction_ratio(), 0.0);
    }
}

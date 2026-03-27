//! Match result types for pattern detection

use std::fmt;

/// A detected pattern match in text
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Match {
    pub start: usize,
    pub end: usize,
    pub pattern_type: u16,
}

impl Match {
    pub fn new(start: usize, end: usize, pattern_type: u16) -> Self {
        Self { start, end, pattern_type }
    }

    pub fn len(&self) -> usize {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

impl fmt::Display for Match {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Match({}..{}, type={})", self.start, self.end, self.pattern_type)
    }
}

/// Detection result containing all matches found
#[derive(Debug, Clone)]
pub struct DetectionResult {
    pub matches: Vec<Match>,
}

impl DetectionResult {
    pub fn new() -> Self {
        Self { matches: Vec::new() }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self { matches: Vec::with_capacity(capacity) }
    }

    pub fn add(&mut self, m: Match) {
        self.matches.push(m);
    }

    pub fn count(&self) -> usize {
        self.matches.len()
    }

    /// Extend this result with matches from another result
    pub fn extend(&mut self, other: DetectionResult) {
        self.matches.extend(other.matches);
    }

    /// Remove overlapping matches, keeping the longest match in each overlapping region
    pub fn remove_overlaps(&mut self) {
        if self.matches.len() <= 1 {
            return;
        }

        // Sort by start position, then by end position (longest first)
        self.matches.sort_by(|a, b| {
            a.start.cmp(&b.start).then_with(|| b.end.cmp(&a.end))
        });

        let mut result = Vec::with_capacity(self.matches.len());
        let mut last_end = 0;

        for m in &self.matches {
            // If this match doesn't overlap with the last kept match, add it
            if m.start >= last_end {
                result.push(*m);
                last_end = m.end;
            }
            // Otherwise skip it (overlapping with longer match)
        }

        self.matches = result;
    }
}

impl Default for DetectionResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Redaction result with output and matches
#[derive(Debug)]
pub struct RedactionResult {
    pub output: Vec<u8>,
    pub matches: Vec<Match>,
}

impl RedactionResult {
    pub fn new(output: Vec<u8>, matches: Vec<Match>) -> Self {
        Self { output, matches }
    }

    pub fn match_count(&self) -> usize {
        self.matches.len()
    }
}


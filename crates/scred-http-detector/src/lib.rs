//! SCRED HTTP Detector - Content analysis layer
//!
//! Analyzes HTTP requests and responses to detect what needs redaction.
//! This is a pure analysis layer (no mutations).
//!
//! ## Usage
//!
//! ```ignore
//! use scred_http_detector::{HttpContentAnalyzer, Sensitivity};
//!
//! let detector = HttpContentAnalyzer::new(pattern_detector);
//! let analysis = detector.analyze_headers(&request.headers())?;
//!
//! if analysis.sensitivity >= Sensitivity::Internal {
//!     // This request needs redaction
//! }
//! ```

pub mod analyzer;
pub mod classification;
pub mod header_analysis;
pub mod body_analysis;
pub mod models;

pub use analyzer::ContentAnalyzer;
pub use classification::{Sensitivity, RedactionStrategy};
pub use models::{AnalysisResult, Finding};

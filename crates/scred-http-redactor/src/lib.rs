//! SCRED HTTP Redactor - HTTP-specific redaction strategies
//!
//! Performs actual redaction on HTTP requests and responses.
//! Supports multiple protocols: HTTP/1.1, HTTP/2.
//!
//! ## Usage
//!
//! ```ignore
//! use scred_http_redactor::Http11Redactor;
//!
//! let redactor = Http11Redactor::new(engine);
//! redactor.redact_request(&mut request)?;
//! ```

pub mod core;
pub mod header_redaction;
pub mod body_redaction;
pub mod protocol;
pub mod models;

pub use core::HttpRedactor;
pub use protocol::{Http11Redactor, H2Redactor};
pub use models::RedactionStats;

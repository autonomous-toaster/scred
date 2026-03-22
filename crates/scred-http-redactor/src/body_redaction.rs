//! HTTP body content redaction
//!
//! Handles redaction of different body content types (JSON, XML, Form, etc.)

use crate::models::RedactionStats;
use anyhow::Result;
use scred_http_detector::models::ContentType;
use serde_json::{json, Value};

/// Redactor for HTTP body content
pub struct BodyRedactor;

impl BodyRedactor {
    pub fn new() -> Self {
        Self
    }

    /// Redact body content based on content type
    pub fn redact_body(&self, body: &mut Vec<u8>, content_type: ContentType) -> Result<RedactionStats> {
        match content_type {
            ContentType::Json => self.redact_json_body(body),
            ContentType::Xml => self.redact_xml_body(body),
            ContentType::FormData => self.redact_form_body(body),
            ContentType::PlainText => Ok(RedactionStats::new()),
            ContentType::Html => Ok(RedactionStats::new()),
            _ => Ok(RedactionStats::new()),
        }
    }

    /// Redact JSON body
    fn redact_json_body(&self, body: &mut Vec<u8>) -> Result<RedactionStats> {
        let mut stats = RedactionStats::new();

        // Parse JSON
        let body_str = String::from_utf8_lossy(body);
        match serde_json::from_str::<Value>(&body_str) {
            Ok(mut value) => {
                stats.bytes_processed = body.len() as u64;

                self.redact_json_value(&mut value, &mut stats);

                // Serialize back
                if let Ok(redacted) = serde_json::to_vec(&value) {
                    *body = redacted;
                    stats.bytes_redacted = body.len() as u64;
                }

                Ok(stats)
            }
            Err(_) => Ok(stats), // Can't parse, leave as-is
        }
    }

    fn redact_json_value(&self, value: &mut Value, stats: &mut RedactionStats) {
        match value {
            Value::Object(obj) => {
                for (key, val) in obj.iter_mut() {
                    if self.is_sensitive_json_field(key) {
                        stats.patterns_found += 1;
                        if let Value::String(s) = val {
                            stats.bytes_processed += s.len() as u64;
                            stats.bytes_redacted += 11; // "[REDACTED]"
                        }
                        *val = json!("[REDACTED]");
                    } else {
                        self.redact_json_value(val, stats);
                    }
                }
            }
            Value::Array(arr) => {
                for val in arr.iter_mut() {
                    self.redact_json_value(val, stats);
                }
            }
            _ => {}
        }
    }

    /// Redact XML body
    fn redact_xml_body(&self, body: &mut Vec<u8>) -> Result<RedactionStats> {
        let mut stats = RedactionStats::new();
        let body_str = String::from_utf8_lossy(body).to_string();

        stats.bytes_processed = body.len() as u64;

        let sensitive_tags = vec!["password", "token", "api_key", "secret"];
        let mut redacted = body_str.clone();

        for tag in sensitive_tags {
            let open = format!("<{}>", tag);
            let close = format!("</{}>", tag);

            while let Some(start) = redacted.find(&open) {
                if let Some(end) = redacted[start..].find(&close) {
                    let end_pos = start + end;
                    stats.patterns_found += 1;

                    redacted.replace_range(
                        (start + open.len())..end_pos,
                        "[REDACTED]",
                    );
                }
            }
        }

        let original_len = body.len();
        *body = redacted.into_bytes();
        stats.bytes_redacted = (original_len as i64 - body.len() as i64).max(0) as u64;

        Ok(stats)
    }

    /// Redact form data
    fn redact_form_body(&self, body: &mut Vec<u8>) -> Result<RedactionStats> {
        let mut stats = RedactionStats::new();
        let body_str = String::from_utf8_lossy(body);

        stats.bytes_processed = body.len() as u64;

        let mut redacted = String::new();
        for pair in body_str.split('&') {
            if !redacted.is_empty() {
                redacted.push('&');
            }

            if let Some((key, value)) = pair.split_once('=') {
                if self.is_sensitive_form_field(key) {
                    stats.patterns_found += 1;
                    stats.bytes_redacted += value.len() as u64;
                    redacted.push_str(key);
                    redacted.push('=');
                    redacted.push_str("[REDACTED]");
                } else {
                    redacted.push_str(pair);
                }
            } else {
                redacted.push_str(pair);
            }
        }

        *body = redacted.into_bytes();
        Ok(stats)
    }

    fn is_sensitive_json_field(&self, field: &str) -> bool {
        matches!(
            field.to_lowercase().as_str(),
            "password"
                | "pwd"
                | "token"
                | "api_key"
                | "apikey"
                | "access_token"
                | "secret"
                | "auth_token"
                | "authorization"
        )
    }

    fn is_sensitive_form_field(&self, field: &str) -> bool {
        self.is_sensitive_json_field(field)
    }
}

impl Default for BodyRedactor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_json_body() {
        let redactor = BodyRedactor::new();
        let mut body = br#"{"name":"John","password":"secret123"}"#.to_vec();

        let stats = redactor.redact_json_body(&mut body).unwrap();

        assert_eq!(stats.patterns_found, 1);
        assert!(stats.bytes_redacted > 0);

        let redacted_str = String::from_utf8_lossy(&body);
        assert!(redacted_str.contains("[REDACTED]"));
    }

    #[test]
    fn test_redact_json_nested() {
        let redactor = BodyRedactor::new();
        let mut body = br#"{"user":{"name":"John","api_key":"xyz123"}}"#.to_vec();

        let stats = redactor.redact_json_body(&mut body).unwrap();
        assert_eq!(stats.patterns_found, 1);
    }

    #[test]
    fn test_redact_form_body() {
        let redactor = BodyRedactor::new();
        let mut body = b"username=john&password=secret&remember=true".to_vec();

        let stats = redactor.redact_form_body(&mut body).unwrap();

        assert_eq!(stats.patterns_found, 1);

        let redacted_str = String::from_utf8_lossy(&body);
        assert!(redacted_str.contains("[REDACTED]"));
        assert!(redacted_str.contains("username=john"));
    }
}

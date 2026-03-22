//! Sensitivity classification for HTTP content
//!
//! Determines how sensitive different headers, fields, and patterns are.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Sensitivity levels for content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum Sensitivity {
    /// Public data, no redaction needed
    Public = 0,
    /// Internal data, some redaction
    Internal = 1,
    /// Confidential data, full redaction
    Confidential = 2,
    /// Secret data, always redact
    Secret = 3,
}

impl Sensitivity {
    pub fn description(&self) -> &'static str {
        match self {
            Sensitivity::Public => "Public (no redaction)",
            Sensitivity::Internal => "Internal (partial redaction)",
            Sensitivity::Confidential => "Confidential (full redaction)",
            Sensitivity::Secret => "Secret (always redact)",
        }
    }

    pub fn should_redact(&self) -> bool {
        matches!(
            self,
            Sensitivity::Internal | Sensitivity::Confidential | Sensitivity::Secret
        )
    }
}

/// Redaction strategy for a piece of content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedactionStrategy {
    /// Sensitivity level
    pub sensitivity: Sensitivity,
    /// Whether to completely redact or show hints
    pub full_redaction: bool,
    /// Custom mask string (e.g., "[REDACTED]")
    pub mask: String,
}

impl Default for RedactionStrategy {
    fn default() -> Self {
        Self {
            sensitivity: Sensitivity::Public,
            full_redaction: true,
            mask: "[REDACTED]".to_string(),
        }
    }
}

impl RedactionStrategy {
    pub fn new(sensitivity: Sensitivity) -> Self {
        Self {
            sensitivity,
            full_redaction: sensitivity.should_redact(),
            mask: match sensitivity {
                Sensitivity::Public => "".to_string(),
                Sensitivity::Internal => "[***]".to_string(),
                Sensitivity::Confidential => "[REDACTED]".to_string(),
                Sensitivity::Secret => "[CLASSIFIED]".to_string(),
            },
        }
    }
}

/// Classify sensitivity of headers
pub fn classify_header(name: &str, _value: &str) -> Sensitivity {
    let lower_name = name.to_lowercase();

    // Always secret
    if matches!(
        lower_name.as_str(),
        "authorization" | "proxy-authorization" | "x-api-key" | "x-auth-token" | "x-access-token"
    ) {
        return Sensitivity::Secret;
    }

    // Confidential
    if matches!(
        lower_name.as_str(),
        "cookie"
            | "set-cookie"
            | "x-csrf-token"
            | "x-session-id"
            | "x-user-id"
            | "x-auth"
            | "www-authenticate"
            | "proxy-authenticate"
    ) {
        return Sensitivity::Confidential;
    }

    // Everything else is public (standard headers don't require redaction)
    Sensitivity::Public
}

/// Classify sensitivity of JSON field names
pub fn classify_json_field(field_name: &str) -> Sensitivity {
    let lower = field_name.to_lowercase();

    // Secret fields
    if matches!(
        lower.as_str(),
        "password"
            | "pwd"
            | "secret"
            | "token"
            | "api_key"
            | "apikey"
            | "access_token"
            | "refresh_token"
            | "auth_token"
            | "authorization"
            | "private_key"
    ) {
        return Sensitivity::Secret;
    }

    // Confidential fields
    if matches!(
        lower.as_str(),
        "email"
            | "phone"
            | "ssn"
            | "credit_card"
            | "card_number"
            | "cvv"
            | "social_security"
            | "drivers_license"
            | "passport"
            | "session_id"
            | "user_id"
    ) {
        return Sensitivity::Confidential;
    }

    // Internal fields
    if matches!(
        lower.as_str(),
        "id" | "user" | "name" | "title" | "department" | "url" | "ip"
    ) {
        return Sensitivity::Internal;
    }

    Sensitivity::Public
}

/// Classify sensitivity of XML element names
pub fn classify_xml_element(element_name: &str) -> Sensitivity {
    // Delegate to JSON field logic (similar naming conventions)
    classify_json_field(element_name)
}

/// Classify sensitivity of form field names
pub fn classify_form_field(field_name: &str) -> Sensitivity {
    // Delegate to JSON field logic
    classify_json_field(field_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_header_secret() {
        assert_eq!(classify_header("Authorization", "Bearer xyz"), Sensitivity::Secret);
        assert_eq!(classify_header("X-API-Key", "secret"), Sensitivity::Secret);
        assert_eq!(classify_header("X-Auth-Token", "token"), Sensitivity::Secret);
    }

    #[test]
    fn test_classify_header_confidential() {
        assert_eq!(classify_header("Cookie", "session=123"), Sensitivity::Confidential);
        assert_eq!(classify_header("Set-Cookie", "token=xyz"), Sensitivity::Confidential);
        assert_eq!(classify_header("X-CSRF-Token", "csrf"), Sensitivity::Confidential);
    }

    #[test]
    fn test_classify_header_public() {
        assert_eq!(classify_header("Accept", "application/json"), Sensitivity::Public);
        assert_eq!(classify_header("Cache-Control", "no-cache"), Sensitivity::Public);
    }

    #[test]
    fn test_classify_json_field_secret() {
        assert_eq!(classify_json_field("password"), Sensitivity::Secret);
        assert_eq!(classify_json_field("token"), Sensitivity::Secret);
        assert_eq!(classify_json_field("api_key"), Sensitivity::Secret);
        assert_eq!(classify_json_field("access_token"), Sensitivity::Secret);
    }

    #[test]
    fn test_classify_json_field_confidential() {
        assert_eq!(classify_json_field("email"), Sensitivity::Confidential);
        assert_eq!(classify_json_field("phone"), Sensitivity::Confidential);
        assert_eq!(classify_json_field("ssn"), Sensitivity::Confidential);
        assert_eq!(classify_json_field("credit_card"), Sensitivity::Confidential);
    }

    #[test]
    fn test_classify_json_field_public() {
        assert_eq!(classify_json_field("name"), Sensitivity::Internal);
        assert_eq!(classify_json_field("title"), Sensitivity::Internal);
    }

    #[test]
    fn test_sensitivity_should_redact() {
        assert!(!Sensitivity::Public.should_redact());
        assert!(Sensitivity::Internal.should_redact());
        assert!(Sensitivity::Confidential.should_redact());
        assert!(Sensitivity::Secret.should_redact());
    }

    #[test]
    fn test_redaction_strategy() {
        let strategy = RedactionStrategy::new(Sensitivity::Secret);
        assert!(strategy.full_redaction);
        assert_eq!(strategy.mask, "[CLASSIFIED]");

        let strategy = RedactionStrategy::new(Sensitivity::Public);
        assert!(!strategy.full_redaction);
        assert_eq!(strategy.mask, "");
    }
}

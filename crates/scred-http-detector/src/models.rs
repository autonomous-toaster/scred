//! Data models for HTTP content analysis
//!
//! Represents the results of analyzing HTTP headers and bodies.

use crate::classification::Sensitivity;
use serde::{Deserialize, Serialize};


/// Types of HTTP content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ContentType {
    Json,
    Xml,
    FormData,
    PlainText,
    Html,
    Binary,
    Unknown,
}

impl ContentType {
    pub fn from_header(content_type: &str) -> Self {
        let lower = content_type.to_lowercase();
        if lower.contains("json") {
            ContentType::Json
        } else if lower.contains("xml") {
            ContentType::Xml
        } else if lower.contains("form") {
            ContentType::FormData
        } else if lower.contains("plain") {
            ContentType::PlainText
        } else if lower.contains("html") {
            ContentType::Html
        } else {
            ContentType::Unknown
        }
    }

    pub fn to_string(&self) -> &'static str {
        match self {
            ContentType::Json => "application/json",
            ContentType::Xml => "application/xml",
            ContentType::FormData => "application/x-www-form-urlencoded",
            ContentType::PlainText => "text/plain",
            ContentType::Html => "text/html",
            ContentType::Binary => "application/octet-stream",
            ContentType::Unknown => "unknown",
        }
    }
}

/// A single finding in content (sensitive field, token, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// JSONPath or XPath to the sensitive data location
    pub path: String,
    /// The actual value (may be truncated for display)
    pub value: String,
    /// Optional pattern ID if matched against pattern detector
    pub pattern_id: Option<String>,
    /// Classification of sensitivity
    pub sensitivity: Sensitivity,
    /// Type of finding (e.g., "header", "json_field", "xml_element")
    pub finding_type: String,
}

/// Analysis result for a piece of content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    /// Detected content type
    pub content_type: ContentType,
    /// Overall sensitivity level
    pub sensitivity: Sensitivity,
    /// All findings in this content
    pub findings: Vec<Finding>,
    /// Whether redaction is recommended
    pub needs_redaction: bool,
}

impl AnalysisResult {
    pub fn new(content_type: ContentType) -> Self {
        Self {
            content_type,
            sensitivity: Sensitivity::Public,
            findings: Vec::new(),
            needs_redaction: false,
        }
    }

    /// Add a finding and update sensitivity level
    pub fn add_finding(&mut self, finding: Finding) {
        if finding.sensitivity > self.sensitivity {
            self.sensitivity = finding.sensitivity;
        }
        if finding.sensitivity >= Sensitivity::Internal {
            self.needs_redaction = true;
        }
        self.findings.push(finding);
    }

    /// Check if any findings exist
    pub fn has_findings(&self) -> bool {
        !self.findings.is_empty()
    }
}

/// Analysis of HTTP headers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadersAnalysis {
    pub findings: Vec<Finding>,
    pub sensitivity: Sensitivity,
}

/// Analysis of body content (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BodyAnalysis {
    pub content_type: ContentType,
    pub findings: Vec<Finding>,
    pub sensitivity: Sensitivity,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_type_from_header() {
        assert_eq!(
            ContentType::from_header("application/json; charset=utf-8"),
            ContentType::Json
        );
        assert_eq!(
            ContentType::from_header("application/xml"),
            ContentType::Xml
        );
        assert_eq!(
            ContentType::from_header("application/x-www-form-urlencoded"),
            ContentType::FormData
        );
        assert_eq!(
            ContentType::from_header("text/plain"),
            ContentType::PlainText
        );
        assert_eq!(
            ContentType::from_header("text/html; charset=utf-8"),
            ContentType::Html
        );
    }

    #[test]
    fn test_analysis_result_sensitivity_escalation() {
        let mut result = AnalysisResult::new(ContentType::Json);
        assert_eq!(result.sensitivity, Sensitivity::Public);
        assert!(!result.needs_redaction);

        // Add a public finding (should not trigger redaction)
        result.add_finding(Finding {
            path: "$.name".to_string(),
            value: "John".to_string(),
            pattern_id: None,
            sensitivity: Sensitivity::Public,
            finding_type: "string".to_string(),
        });
        assert_eq!(result.sensitivity, Sensitivity::Public);
        assert!(!result.needs_redaction);

        // Add an internal finding
        result.add_finding(Finding {
            path: "$.token".to_string(),
            value: "secret".to_string(),
            pattern_id: None,
            sensitivity: Sensitivity::Internal,
            finding_type: "token".to_string(),
        });
        assert_eq!(result.sensitivity, Sensitivity::Internal);
        assert!(result.needs_redaction);

        // Add a confidential finding (sensitivity level only increases)
        result.add_finding(Finding {
            path: "$.password".to_string(),
            value: "pass123".to_string(),
            pattern_id: None,
            sensitivity: Sensitivity::Confidential,
            finding_type: "password".to_string(),
        });
        assert_eq!(result.sensitivity, Sensitivity::Confidential);
        assert!(result.needs_redaction);
    }

    #[test]
    fn test_analysis_result_has_findings() {
        let mut result = AnalysisResult::new(ContentType::Json);
        assert!(!result.has_findings());

        result.add_finding(Finding {
            path: "$.name".to_string(),
            value: "John".to_string(),
            pattern_id: None,
            sensitivity: Sensitivity::Public,
            finding_type: "string".to_string(),
        });
        assert!(result.has_findings());
    }
}

//! HTTP body content analysis
//!
//! Analyzes request/response bodies (JSON, XML, form data, etc.)

use crate::classification::{classify_form_field, classify_json_field, classify_xml_element, Sensitivity};
use crate::models::{BodyAnalysis, ContentType, Finding};
use serde_json::Value;

/// Analyzer for HTTP body content
pub struct BodyAnalyzer;

impl BodyAnalyzer {
    /// Create a new body analyzer
    pub fn new() -> Self {
        Self
    }

    /// Detect content type from body and headers
    pub fn detect_content_type(&self, _body: &str, content_type_header: Option<&str>) -> ContentType {
        match content_type_header {
            Some(ct) => ContentType::from_header(ct),
            None => ContentType::Unknown,
        }
    }

    /// Analyze body based on detected content type
    pub fn analyze_body(
        &self,
        body: &str,
        content_type: ContentType,
    ) -> BodyAnalysis {
        match content_type {
            ContentType::Json => self.analyze_json(body),
            ContentType::Xml => self.analyze_xml(body),
            ContentType::FormData => self.analyze_form(body),
            ContentType::PlainText => self.analyze_plain_text(body),
            ContentType::Html => self.analyze_html(body),
            _ => BodyAnalysis {
                content_type,
                findings: Vec::new(),
                sensitivity: Sensitivity::Public,
            },
        }
    }

    /// Analyze JSON content
    pub fn analyze_json(&self, body: &str) -> BodyAnalysis {
        let mut findings = Vec::new();
        let mut max_sensitivity = Sensitivity::Public;

        match serde_json::from_str::<Value>(body) {
            Ok(value) => {
                self.analyze_json_value(&value, "$".to_string(), &mut findings, &mut max_sensitivity);
            }
            Err(_) => {
                // Invalid JSON, skip analysis
            }
        }

        BodyAnalysis {
            content_type: ContentType::Json,
            findings,
            sensitivity: max_sensitivity,
        }
    }

    fn analyze_json_value(
        &self,
        value: &Value,
        path: String,
        findings: &mut Vec<Finding>,
        max_sensitivity: &mut Sensitivity,
    ) {
        match value {
            Value::Object(obj) => {
                for (key, val) in obj.iter() {
                    let key_sensitivity = classify_json_field(key);
                    let new_path = format!("{}.{}", path, key);

                    // Check key sensitivity first
                    if key_sensitivity > Sensitivity::Public {
                        if let Value::String(s) = val {
                            findings.push(Finding {
                                path: new_path.clone(),
                                value: s.clone(),
                                pattern_id: None,
                                sensitivity: key_sensitivity,
                                finding_type: "json_field".to_string(),
                            });
                            if key_sensitivity > *max_sensitivity {
                                *max_sensitivity = key_sensitivity;
                            }
                        }
                    }

                    // Recurse into nested values
                    self.analyze_json_value(val, new_path, findings, max_sensitivity);
                }
            }
            Value::Array(arr) => {
                for (idx, val) in arr.iter().enumerate() {
                    let new_path = format!("{}[{}]", path, idx);
                    self.analyze_json_value(val, new_path, findings, max_sensitivity);
                }
            }
            _ => {}
        }
    }

    /// Analyze XML content (simplified)
    pub fn analyze_xml(&self, body: &str) -> BodyAnalysis {
        let mut findings = Vec::new();
        let mut max_sensitivity = Sensitivity::Public;

        // Very simple XML analysis - look for common sensitive tags
        let sensitive_tags = vec![
            "password", "token", "api_key", "secret", "apikey",
            "access_token", "auth_token", "credit_card",
        ];

        for tag in sensitive_tags {
            let open_tag = format!("<{}>", tag);
            let close_tag = format!("</{}>", tag);

            for (idx, _) in body.match_indices(&open_tag).enumerate() {
                if let Some(start) = body.find(&open_tag) {
                    if let Some(end) = body[start..].find(&close_tag) {
                        let value = &body[start + open_tag.len()..start + end];
                        let sensitivity = classify_xml_element(tag);

                        findings.push(Finding {
                            path: format!("/{}", tag),
                            value: value.to_string(),
                            pattern_id: None,
                            sensitivity,
                            finding_type: "xml_element".to_string(),
                        });

                        if sensitivity > max_sensitivity {
                            max_sensitivity = sensitivity;
                        }

                        if idx > 0 {
                            break; // Only report first occurrence
                        }
                    }
                }
            }
        }

        BodyAnalysis {
            content_type: ContentType::Xml,
            findings,
            sensitivity: max_sensitivity,
        }
    }

    /// Analyze form data
    pub fn analyze_form(&self, body: &str) -> BodyAnalysis {
        let mut findings = Vec::new();
        let mut max_sensitivity = Sensitivity::Public;

        // Parse simple form data (key=value&key=value)
        for pair in body.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                let key_decoded = urlencoding::decode(key).unwrap_or_else(|_| key.into());
                let sensitivity = classify_form_field(&key_decoded);

                if sensitivity > Sensitivity::Public {
                    findings.push(Finding {
                        path: format!("form:{}", key_decoded),
                        value: value.to_string(),
                        pattern_id: None,
                        sensitivity,
                        finding_type: "form_field".to_string(),
                    });

                    if sensitivity > max_sensitivity {
                        max_sensitivity = sensitivity;
                    }
                }
            }
        }

        BodyAnalysis {
            content_type: ContentType::FormData,
            findings,
            sensitivity: max_sensitivity,
        }
    }

    /// Analyze plain text (very basic)
    pub fn analyze_plain_text(&self, _body: &str) -> BodyAnalysis {
        BodyAnalysis {
            content_type: ContentType::PlainText,
            findings: Vec::new(),
            sensitivity: Sensitivity::Public,
        }
    }

    /// Analyze HTML (basic)
    pub fn analyze_html(&self, _body: &str) -> BodyAnalysis {
        BodyAnalysis {
            content_type: ContentType::Html,
            findings: Vec::new(),
            sensitivity: Sensitivity::Public,
        }
    }
}

impl Default for BodyAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_json_with_password() {
        let analyzer = BodyAnalyzer::new();
        let json = r#"{"name": "John", "password": "secret123", "email": "john@example.com"}"#;

        let analysis = analyzer.analyze_json(json);
        assert_eq!(analysis.content_type, ContentType::Json);
        assert_eq!(analysis.sensitivity, Sensitivity::Secret);
        assert!(analysis.findings.iter().any(|f| f.path.contains("password")));
    }

    #[test]
    fn test_analyze_json_nested() {
        let analyzer = BodyAnalyzer::new();
        let json = r#"{
            "user": {
                "name": "John",
                "api_key": "abc123"
            }
        }"#;

        let analysis = analyzer.analyze_json(json);
        assert_eq!(analysis.sensitivity, Sensitivity::Secret);
        assert!(analysis.findings.iter().any(|f| f.path.contains("api_key")));
    }

    #[test]
    fn test_analyze_json_no_sensitive_data() {
        let analyzer = BodyAnalyzer::new();
        let json = r#"{"name": "John", "title": "Engineer", "department": "R&D"}"#;

        let analysis = analyzer.analyze_json(json);
        assert_eq!(analysis.sensitivity, Sensitivity::Internal);
    }

    #[test]
    fn test_analyze_json_invalid() {
        let analyzer = BodyAnalyzer::new();
        let json = "invalid json {";

        let analysis = analyzer.analyze_json(json);
        assert_eq!(analysis.sensitivity, Sensitivity::Public);
        assert!(analysis.findings.is_empty());
    }

    #[test]
    fn test_analyze_form_data() {
        let analyzer = BodyAnalyzer::new();
        let form = "username=john&password=secret123&remember=true";

        let analysis = analyzer.analyze_form(form);
        assert_eq!(analysis.content_type, ContentType::FormData);
        assert_eq!(analysis.sensitivity, Sensitivity::Secret);
        assert!(analysis.findings.iter().any(|f| f.path.contains("password")));
    }

    #[test]
    fn test_detect_content_type() {
        let analyzer = BodyAnalyzer::new();

        assert_eq!(
            analyzer.detect_content_type("", Some("application/json")),
            ContentType::Json
        );
        assert_eq!(
            analyzer.detect_content_type("", Some("application/xml")),
            ContentType::Xml
        );
        assert_eq!(
            analyzer.detect_content_type("", Some("application/x-www-form-urlencoded")),
            ContentType::FormData
        );
    }
}

//! HTTP header analysis
//!
//! Detects sensitive headers and classifies them.

use crate::classification::{classify_header, Sensitivity};
use crate::models::{Finding, HeadersAnalysis};

/// Analyzer for HTTP headers
pub struct HeaderAnalyzer;

impl HeaderAnalyzer {
    /// Create a new header analyzer
    pub fn new() -> Self {
        Self
    }

    /// Analyze a list of headers and return findings
    pub fn analyze_headers(&self, headers: &[(String, String)]) -> HeadersAnalysis {
        let mut findings = Vec::new();
        let mut max_sensitivity = Sensitivity::Public;

        for (name, value) in headers {
            let sensitivity = classify_header(name, value);

            // Only report findings for non-public headers
            if sensitivity > Sensitivity::Public {
                findings.push(Finding {
                    path: format!("header:{}", name),
                    value: self.mask_value(value, sensitivity),
                    pattern_id: None,
                    sensitivity,
                    finding_type: "http_header".to_string(),
                });

                if sensitivity > max_sensitivity {
                    max_sensitivity = sensitivity;
                }
            }
        }

        HeadersAnalysis {
            findings,
            sensitivity: max_sensitivity,
        }
    }

    /// Classify a single header
    pub fn classify_header(&self, name: &str, value: &str) -> Sensitivity {
        classify_header(name, value)
    }

    /// Mask a value based on sensitivity
    fn mask_value(&self, value: &str, sensitivity: Sensitivity) -> String {
        match sensitivity {
            Sensitivity::Public => value.to_string(),
            Sensitivity::Internal => {
                if value.len() <= 4 {
                    "***".to_string()
                } else {
                    format!("{}...", &value[..value.len().min(4)])
                }
            }
            Sensitivity::Confidential => {
                if value.len() <= 8 {
                    "[REDACTED]".to_string()
                } else {
                    format!("{}...", &value[..value.len().min(8)])
                }
            }
            Sensitivity::Secret => "[CLASSIFIED]".to_string(),
        }
    }

    /// Get all headers that need redaction
    pub fn get_sensitive_headers(&self, headers: &[(String, String)]) -> Vec<(String, Sensitivity)> {
        headers
            .iter()
            .filter_map(|(name, value)| {
                let sensitivity = self.classify_header(name, value);
                if sensitivity > Sensitivity::Public {
                    Some((name.clone(), sensitivity))
                } else {
                    None
                }
            })
            .collect()
    }
}

impl Default for HeaderAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_headers_with_auth() {
        let analyzer = HeaderAnalyzer::new();
        let headers = vec![
            ("Host".to_string(), "example.com".to_string()),
            ("Authorization".to_string(), "Bearer xyz123".to_string()),
            ("User-Agent".to_string(), "curl/7.64.1".to_string()),
        ];

        let analysis = analyzer.analyze_headers(&headers);
        assert_eq!(analysis.sensitivity, Sensitivity::Secret);
        assert_eq!(analysis.findings.len(), 1);
        assert_eq!(analysis.findings[0].sensitivity, Sensitivity::Secret);
    }

    #[test]
    fn test_analyze_headers_with_cookie() {
        let analyzer = HeaderAnalyzer::new();
        let headers = vec![
            ("Host".to_string(), "example.com".to_string()),
            ("Cookie".to_string(), "session=abc123".to_string()),
        ];

        let analysis = analyzer.analyze_headers(&headers);
        assert_eq!(analysis.sensitivity, Sensitivity::Confidential);
        assert_eq!(analysis.findings.len(), 1);
    }

    #[test]
    fn test_analyze_headers_all_public() {
        let analyzer = HeaderAnalyzer::new();
        let headers = vec![
            ("Host".to_string(), "example.com".to_string()),
            ("User-Agent".to_string(), "curl/7.64.1".to_string()),
            ("Accept".to_string(), "application/json".to_string()),
        ];

        let analysis = analyzer.analyze_headers(&headers);
        assert_eq!(analysis.sensitivity, Sensitivity::Public);
        assert!(analysis.findings.is_empty());
    }

    #[test]
    fn test_get_sensitive_headers() {
        let analyzer = HeaderAnalyzer::new();
        let headers = vec![
            ("Host".to_string(), "example.com".to_string()),
            ("Authorization".to_string(), "Bearer xyz".to_string()),
            ("Cookie".to_string(), "session=123".to_string()),
            ("User-Agent".to_string(), "curl".to_string()),
            ("X-API-Key".to_string(), "key123".to_string()),
        ];

        let sensitive = analyzer.get_sensitive_headers(&headers);
        assert_eq!(sensitive.len(), 3);
        assert!(sensitive.iter().any(|(n, _)| n == "Authorization"));
        assert!(sensitive.iter().any(|(n, _)| n == "Cookie"));
        assert!(sensitive.iter().any(|(n, _)| n == "X-API-Key"));
    }

    #[test]
    fn test_mask_value_by_sensitivity() {
        let analyzer = HeaderAnalyzer::new();

        assert_eq!(
            analyzer.mask_value("public", Sensitivity::Public),
            "public"
        );
        assert_eq!(analyzer.mask_value("internal", Sensitivity::Internal), "inte...");
        assert_eq!(
            analyzer.mask_value("confidential", Sensitivity::Confidential),
            "confiden..."
        );
        assert_eq!(
            analyzer.mask_value("secret", Sensitivity::Secret),
            "[CLASSIFIED]"
        );
    }
}

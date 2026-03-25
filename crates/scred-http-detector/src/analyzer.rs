//! Core HTTP content analyzer
//!
//! Provides the main trait and implementation for analyzing HTTP content.

use crate::body_analysis::BodyAnalyzer;
use crate::header_analysis::HeaderAnalyzer;
use crate::models::{AnalysisResult, ContentType};
use std::sync::Arc;

/// Trait for analyzing HTTP content
pub trait ContentAnalyzer {
    /// Analyze request headers
    fn analyze_headers(&self, headers: &[(String, String)]) -> AnalysisResult;

    /// Analyze request/response body
    fn analyze_body(&self, body: &str, content_type: Option<&str>) -> AnalysisResult;

    /// Analyze complete request
    fn analyze_request(
        &self,
        headers: &[(String, String)],
        body: Option<&str>,
    ) -> AnalysisResult;

    /// Analyze complete response
    fn analyze_response(
        &self,
        headers: &[(String, String)],
        body: Option<&str>,
    ) -> AnalysisResult;
}

/// Default HTTP content analyzer implementation
pub struct HttpContentAnalyzer {
    header_analyzer: Arc<HeaderAnalyzer>,
    body_analyzer: Arc<BodyAnalyzer>,
}

impl HttpContentAnalyzer {
    /// Create a new HTTP content analyzer
    pub fn new() -> Self {
        Self {
            header_analyzer: Arc::new(HeaderAnalyzer::new()),
            body_analyzer: Arc::new(BodyAnalyzer::new()),
        }
    }
}

impl Default for HttpContentAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ContentAnalyzer for HttpContentAnalyzer {
    fn analyze_headers(&self, headers: &[(String, String)]) -> AnalysisResult {
        let analysis = self.header_analyzer.analyze_headers(headers);

        let mut result = AnalysisResult::new(ContentType::Unknown);
        result.sensitivity = analysis.sensitivity;

        for finding in analysis.findings {
            result.add_finding(finding);
        }

        result
    }

    fn analyze_body(&self, body: &str, content_type: Option<&str>) -> AnalysisResult {
        let ct = self
            .body_analyzer
            .detect_content_type(body, content_type);
        let analysis = self.body_analyzer.analyze_body(body, ct);

        let mut result = AnalysisResult::new(ct);
        result.sensitivity = analysis.sensitivity;

        for finding in analysis.findings {
            result.add_finding(finding);
        }

        result
    }

    fn analyze_request(
        &self,
        headers: &[(String, String)],
        body: Option<&str>,
    ) -> AnalysisResult {
        // Analyze headers first
        let mut result = self.analyze_headers(headers);

        // Then analyze body if present
        if let Some(body_content) = body {
            // Find Content-Type header
            let content_type = headers
                .iter()
                .find(|(name, _)| name.to_lowercase() == "content-type")
                .map(|(_, value)| value.as_str());

            let body_result = self.analyze_body(body_content, content_type);

            // Merge body findings
            for finding in body_result.findings {
                result.add_finding(finding);
            }

            // Update content type if found
            if body_result.content_type != ContentType::Unknown {
                result.content_type = body_result.content_type;
            }
        }

        result
    }

    fn analyze_response(
        &self,
        headers: &[(String, String)],
        body: Option<&str>,
    ) -> AnalysisResult {
        // Response analysis is similar to request analysis
        self.analyze_request(headers, body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_analyze_headers_only() {
        let analyzer = HttpContentAnalyzer::new();
        let headers = vec![
            ("Host".to_string(), "example.com".to_string()),
            ("Authorization".to_string(), "Bearer token123".to_string()),
        ];

        let result = analyzer.analyze_headers(&headers);
        assert_eq!(result.sensitivity, Sensitivity::Secret);
        assert!(result.needs_redaction);
    }

    #[test]
    fn test_analyze_json_body() {
        let analyzer = HttpContentAnalyzer::new();
        let body = r#"{"name": "John", "password": "secret123"}"#;

        let result = analyzer.analyze_body(body, Some("application/json"));
        assert_eq!(result.content_type, ContentType::Json);
        assert_eq!(result.sensitivity, Sensitivity::Secret);
        assert!(result.needs_redaction);
    }

    #[test]
    fn test_analyze_complete_request() {
        let analyzer = HttpContentAnalyzer::new();
        let headers = vec![("Content-Type".to_string(), "application/json".to_string())];
        let body = r#"{"email": "john@example.com", "api_key": "xyz123"}"#;

        let result = analyzer.analyze_request(&headers, Some(body));
        assert_eq!(result.sensitivity, Sensitivity::Secret);
        assert!(result.has_findings());
    }

    #[test]
    fn test_analyze_request_no_sensitive_data() {
        let analyzer = HttpContentAnalyzer::new();
        let headers = vec![("Content-Type".to_string(), "application/json".to_string())];
        let body = r#"{"name": "John", "title": "Engineer"}"#;

        let result = analyzer.analyze_request(&headers, Some(body));
        assert_eq!(result.sensitivity, Sensitivity::Internal);
    }

    #[test]
    fn test_analyze_request_no_body() {
        let analyzer = HttpContentAnalyzer::new();
        let headers = vec![
            ("Host".to_string(), "example.com".to_string()),
            ("Accept".to_string(), "application/json".to_string()),
        ];

        let result = analyzer.analyze_request(&headers, None);
        assert_eq!(result.sensitivity, Sensitivity::Public);
    }

    #[test]
    fn test_analyze_response_with_sensitive_data() {
        let analyzer = HttpContentAnalyzer::new();
        let headers = vec![
            ("Content-Type".to_string(), "application/json".to_string()),
            ("Set-Cookie".to_string(), "session=abc123".to_string()),
        ];
        let body = r#"{"data": "response"}"#;

        let result = analyzer.analyze_response(&headers, Some(body));
        assert!(result.sensitivity >= Sensitivity::Internal);
    }
}

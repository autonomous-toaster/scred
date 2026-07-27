#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use scred_redactor::{RedactionConfig, RedactionEngine};
    use crate::mitm::config::RedactionMode;
    use crate::mitm::h2_upstream_forwarder::log_detected_secrets;

    #[test]
    fn test_log_detected_secrets_no_secrets() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig { enabled: true }));
        let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello, world!";
        let detect_patterns = scred_http::PatternSelector::default_detect();
        log_detected_secrets(&engine, response, &detect_patterns);
    }

    #[test]
    fn test_log_detected_secrets_with_api_key() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig { enabled: true }));
        let response = b"api_key=sk-test123456789";
        let detect_patterns = scred_http::PatternSelector::default_detect();
        log_detected_secrets(&engine, response, &detect_patterns);
    }

    #[test]
    fn test_log_detected_secrets_none_selector() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig { enabled: true }));
        let response = b"api_key=sk-test123456789";
        let detect_patterns = scred_http::PatternSelector::None;
        log_detected_secrets(&engine, response, &detect_patterns);
    }

    #[test]
    fn test_redact_mode_passthrough() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig { enabled: true }));
        let body = b"Hello, world!".to_vec();
        let mode = RedactionMode::Passthrough;
        let detect_patterns = scred_http::PatternSelector::default_detect();

        let result = if mode.should_redact() {
            let response_str = String::from_utf8_lossy(&body);
            let result = engine.redact(&response_str);
            result.redacted.into_bytes()
        } else if mode.should_detect() {
            log_detected_secrets(&engine, &body, &detect_patterns);
            body.clone()
        } else {
            body.clone()
        };

        assert_eq!(result, b"Hello, world!");
    }

    #[test]
    fn test_redact_mode_redact() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig { enabled: true }));
        let body = b"My API key is sk-test123456789".to_vec();
        let mode = RedactionMode::Redact;
        let detect_patterns = scred_http::PatternSelector::default_detect();

        let result = if mode.should_redact() {
            let response_str = String::from_utf8_lossy(&body);
            let result = engine.redact(&response_str);
            result.redacted.into_bytes()
        } else if mode.should_detect() {
            log_detected_secrets(&engine, &body, &detect_patterns);
            body.clone()
        } else {
            body.clone()
        };

        let result_str = String::from_utf8_lossy(&result);
        assert!(!result_str.contains("sk-test123456789"), "API key should be redacted");
        assert!(result_str.contains("My API key is"), "Prefix should remain");
    }

    #[test]
    fn test_redact_mode_detect() {
        let engine = Arc::new(RedactionEngine::new(RedactionConfig { enabled: true }));
        let body = b"My API key is sk-test123456789".to_vec();
        let mode = RedactionMode::DetectOnly;
        let detect_patterns = scred_http::PatternSelector::default_detect();

        let result = if mode.should_redact() {
            let response_str = String::from_utf8_lossy(&body);
            let result = engine.redact(&response_str);
            result.redacted.into_bytes()
        } else if mode.should_detect() {
            log_detected_secrets(&engine, &body, &detect_patterns);
            body.clone()
        } else {
            body.clone()
        };

        assert_eq!(result, b"My API key is sk-test123456789");
    }
}

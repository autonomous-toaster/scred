#[cfg(test)]
mod h2_forwarder_tests {
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
        let body = b"My API key is sk-test1234567890123456789012345678901234567890".to_vec();
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
        assert!(!result_str.contains("sk-test1234567890123456789012345678901234567890"), "API key should be redacted");
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

    #[test]
    fn test_is_connection_closed_error_eof() {
        let err = std::io::Error::new(std::io::ErrorKind::Other, "EOF");
        assert!(crate::mitm::h2_upstream_forwarder::is_connection_closed_error(&err));
    }

    #[test]
    fn test_is_connection_closed_error_reset() {
        let err = std::io::Error::new(std::io::ErrorKind::ConnectionReset, "connection reset");
        assert!(crate::mitm::h2_upstream_forwarder::is_connection_closed_error(&err));
    }

    #[test]
    fn test_is_connection_closed_error_unexpected_eof() {
        let err = std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "unexpected eof");
        assert!(crate::mitm::h2_upstream_forwarder::is_connection_closed_error(&err));
    }

    #[test]
    fn test_is_connection_closed_error_aborted() {
        let err = std::io::Error::new(std::io::ErrorKind::ConnectionAborted, "connection aborted");
        assert!(crate::mitm::h2_upstream_forwarder::is_connection_closed_error(&err));
    }

    #[test]
    fn test_is_connection_closed_error_message_contains() {
        let err = std::io::Error::new(std::io::ErrorKind::Other, "connection closed by peer");
        assert!(crate::mitm::h2_upstream_forwarder::is_connection_closed_error(&err));
    }

    #[test]
    fn test_is_connection_closed_error_real_error() {
        let err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "permission denied");
        assert!(!crate::mitm::h2_upstream_forwarder::is_connection_closed_error(&err));
    }

    #[test]
    fn test_is_connection_closed_error_timedout() {
        let err = std::io::Error::new(std::io::ErrorKind::TimedOut, "timed out");
        assert!(!crate::mitm::h2_upstream_forwarder::is_connection_closed_error(&err));
    }

    #[tokio::test]
    async fn test_read_response_direct_basic() {
        let data = b"HTTP/1.1 200 OK\r\n\r\n";
        let mut reader = &data[..];
        let result = crate::mitm::h2_upstream_forwarder::read_response_direct(&mut reader).await.unwrap();
        assert!(String::from_utf8_lossy(&result).contains("HTTP/1.1 200 OK"));
    }

    #[tokio::test]
    async fn test_read_response_direct_empty() {
        let data = b"";
        let mut reader = &data[..];
        let result = crate::mitm::h2_upstream_forwarder::read_response_direct(&mut reader).await.unwrap();
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_read_response_direct_multi_chunk() {
        let data = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello";
        let mut reader = &data[..];
        let result = crate::mitm::h2_upstream_forwarder::read_response_direct(&mut reader).await.unwrap();
        let text = String::from_utf8_lossy(&result);
        assert!(text.contains("hello"));
    }

    #[test]
    fn test_parse_status_from_response_200() {
        let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\n";
        let status = crate::mitm::h2_upstream_forwarder::forward::parse_status_from_response(response).unwrap();
        assert_eq!(status, http::StatusCode::OK);
    }

    #[test]
    fn test_parse_status_from_response_401() {
        let response = b"HTTP/1.1 401 Unauthorized\r\nContent-Type: text/plain\r\n\r\n{\"error\":\"unauthorized\"}";
        let status = crate::mitm::h2_upstream_forwarder::forward::parse_status_from_response(response).unwrap();
        assert_eq!(status, http::StatusCode::UNAUTHORIZED);
    }

    #[test]
    fn test_parse_status_from_response_502() {
        let response = b"HTTP/1.1 502 Bad Gateway\r\n\r\n";
        let status = crate::mitm::h2_upstream_forwarder::forward::parse_status_from_response(response).unwrap();
        assert_eq!(status, http::StatusCode::BAD_GATEWAY);
    }

    #[test]
    fn test_parse_status_from_response_empty_fails() {
        let response = b"";
        let result = crate::mitm::h2_upstream_forwarder::forward::parse_status_from_response(response);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_status_from_response_http10() {
        let response = b"HTTP/1.0 404 Not Found\r\n\r\n";
        let status = crate::mitm::h2_upstream_forwarder::forward::parse_status_from_response(response).unwrap();
        assert_eq!(status, http::StatusCode::NOT_FOUND);
    }
}

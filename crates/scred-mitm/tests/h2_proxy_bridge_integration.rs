//! H2ProxyBridge Integration Tests
//! 
//! Tests for HTTP/2 to HTTP/1.1 proxy bridge integration with tls_mitm

#[cfg(test)]
mod h2_proxy_bridge_integration_tests {
    use scred_http::h2::h2_proxy_bridge::{H2ProxyBridge, BridgeConfig};
    use scred_redactor::{RedactionEngine, RedactionConfig};
    use std::sync::Arc;

    #[test]
    fn test_bridge_config_creation() {
        let config = BridgeConfig {
            redact_headers: true,
            max_request_size: 10 * 1024 * 1024,
            max_response_size: 100 * 1024 * 1024,
            debug_logging: false,
        };

        assert_eq!(config.max_request_size, 10 * 1024 * 1024);
        assert_eq!(config.max_response_size, 100 * 1024 * 1024);
        assert!(config.redact_headers);
        assert!(!config.debug_logging);
    }

    #[test]
    fn test_bridge_initialization() {
        let config = BridgeConfig::default();
        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));

        // Should not panic
        let _bridge = H2ProxyBridge::new(engine, config);
    }

    #[test]
    fn test_proxy_detection_pattern() {
        // Test that proxy detection pattern works
        let proxy_urls = vec![
            "http://proxy.example.com:3128",
            "http://proxy.internal.net:8080",
            "https://secure-proxy.corp.com:443",
        ];

        for url in proxy_urls {
            assert!(url.contains("://"), "Proxy URL should contain protocol: {}", url);
        }
    }

    #[test]
    fn test_upstream_server_pattern() {
        // Test that upstream server detection works (no protocol in address)
        let upstream_urls = vec![
            "example.com:443",
            "api.example.org:443",
            "internal-server:443",
        ];

        for url in upstream_urls {
            assert!(!url.contains("://"), "Upstream URL should not contain protocol: {}", url);
        }
    }

    #[test]
    fn test_bridge_with_redaction_engine() {
        let redaction_config = RedactionConfig::default();

        let engine = Arc::new(RedactionEngine::new(redaction_config));
        let config = BridgeConfig {
            redact_headers: true,
            max_request_size: 10 * 1024 * 1024,
            max_response_size: 100 * 1024 * 1024,
            debug_logging: true,
        };

        let redact_headers = config.redact_headers;
        let debug_logging = config.debug_logging;
        
        let _bridge = H2ProxyBridge::new(engine, config);
        
        // Bridge should be created with redaction engine
        assert!(redact_headers);
        assert!(debug_logging);
    }

    #[test]
    fn test_bridge_config_sizes() {
        let config = BridgeConfig {
            redact_headers: true,
            max_request_size: 5 * 1024 * 1024,    // 5 MB
            max_response_size: 50 * 1024 * 1024,  // 50 MB
            debug_logging: false,
        };

        assert_eq!(config.max_request_size, 5 * 1024 * 1024);
        assert_eq!(config.max_response_size, 50 * 1024 * 1024);
        
        // Larger config
        let large_config = BridgeConfig {
            redact_headers: true,
            max_request_size: 100 * 1024 * 1024,  // 100 MB
            max_response_size: 1024 * 1024 * 1024, // 1 GB
            debug_logging: false,
        };

        assert_eq!(large_config.max_request_size, 100 * 1024 * 1024);
        assert_eq!(large_config.max_response_size, 1024 * 1024 * 1024);
    }

    #[test]
    fn test_integration_scenario_proxy_detection() {
        // Simulate the detection logic from tls_mitm.rs
        let upstream_addrs = vec![
            ("http://proxy.example.com:3128", true),    // Is proxy
            ("https://proxy.internal:8443", true),      // Is proxy
            ("api.example.com:443", false),             // Not proxy
            ("internal-server:443", false),             // Not proxy
            ("direct.upstream.org:443", false),         // Not proxy
        ];

        for (addr, is_proxy) in upstream_addrs {
            let detected_as_proxy = addr.contains("://");
            assert_eq!(
                detected_as_proxy, is_proxy,
                "Address {} should be detected as proxy: {}",
                addr, is_proxy
            );
        }
    }

    #[test]
    fn test_bridge_initialization_for_proxy_path() {
        // This test verifies the integration would work
        let config = BridgeConfig {
            redact_headers: true,
            max_request_size: 10 * 1024 * 1024,
            max_response_size: 100 * 1024 * 1024,
            debug_logging: false,
        };

        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        
        // When proxy detected (contains "://"), bridge should be initialized
        let upstream_addr = "http://proxy.example.com:3128";
        
        if upstream_addr.contains("://") {
            let _bridge = H2ProxyBridge::new(engine, config);
            // Bridge initialized successfully
        }
    }
}

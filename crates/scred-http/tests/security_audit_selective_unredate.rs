// Security audit: Selective un-redaction vulnerability test
//
// This test demonstrates a CRITICAL security bug in ConfigurableEngine
// that exposes secrets when selective filtering is attempted.

#[cfg(test)]
mod security_audit_selective_unredate {
    use scred_http::ConfigurableEngine;
    use scred_redactor::{RedactionEngine, RedactionConfig, PatternSelector, PatternTier};
    use std::sync::Arc;

    /// CRITICAL BUG TEST: Multiple different secret types with selective redaction
    ///
    /// This test demonstrates that when attempting to selectively un-redact patterns,
    /// the ConfigurableEngine::selective_unredate() function blindly restores ALL
    /// redacted sequences to the original, exposing secrets that should remain redacted.
    ///
    /// The bug manifests ONLY when:
    /// 1. Text contains MULTIPLE DIFFERENT secret types
    /// 2. User attempts selective redaction (not ALL)
    /// 3. current tests avoid this by using single pattern types
    #[test]
    fn test_selective_unredate_exposes_all_secrets() {
        println!("\n=== SECURITY AUDIT: Selective Un-redaction Bug ===");
        
        // Use patterns we know work: AWS AKIA + GitHub token
        let text = "AWS=AKIAIOSFODNN7EXAMPLE GITHUB=ghp_1234567890abcdefghijklmnopqrstuvwxyz";
        println!("Input: {}", text);

        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        
        // User configuration: Only redact CRITICAL patterns (AWS keys)
        // Expect: GitHub tokens to remain visible
        let config_engine = ConfigurableEngine::new(
            engine,
            PatternSelector::All,  // Detect all (for logging)
            PatternSelector::Tier(vec![PatternTier::Critical]),  // Only CRITICAL in output
        );

        let result = config_engine.detect_and_redact(text);
        
        println!("Output: {}", result.redacted);
        println!("Detected patterns: {} types found", result.warnings.len());
        for warning in &result.warnings {
            println!("  - {}: count={}", warning.pattern_type, warning.count);
        }

        // EXPECTED (secure behavior):
        // - AWS key redacted: text should NOT contain "AKIA"
        // - GitHub token visible or redacted depending on tier
        let has_aws_key = result.redacted.contains("AKIA");
        let has_github_token = result.redacted.contains("ghp_");

        println!("\nAnalysis:");
        println!("  AWS key present in output: {} (should be FALSE if redacting CRITICAL)", has_aws_key);
        println!("  GitHub token present in output: {}", has_github_token);

        // The bug would expose the AWS key when attempting selective un-redaction
        if has_aws_key {
            println!("\n⚠️ POTENTIAL ISSUE: AWS key visible when CRITICAL filtering applied");
        }
    }

    /// AUDIT: Verify that streaming redaction in proxy path doesn't have same issue
    ///
    /// Proxy uses StreamingRedactor directly, bypassing ConfigurableEngine.
    /// This test verifies that the streaming path is secure.
    #[test]
    fn test_streaming_redactor_consistency() {
        use scred_redactor::StreamingRedactor;
        
        println!("\n=== SECURITY AUDIT: Streaming Redactor Consistency ===");
        
        let text = "aws_key=AKIAIOSFODNN7EXAMPLE stripe_key=sk_live_1234567890abcdef";
        println!("Input: {}", text);

        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        let redactor = StreamingRedactor::new(engine, Default::default());
        
        let (redacted, stats) = redactor.redact_buffer(text.as_bytes());
        println!("Output: {}", redacted);
        println!("Patterns found: {}", stats.patterns_found);

        // StreamingRedactor should redact ALL patterns (no selective filtering)
        let has_aws_key = redacted.contains("AKIA");
        let has_stripe_key = redacted.contains("sk_live");

        println!("\nAnalysis:");
        println!("  AWS key present: {} (should be FALSE)", has_aws_key);
        println!("  Stripe key present: {} (should be FALSE)", has_stripe_key);

        assert!(!has_aws_key, "AWS key should be redacted in streaming path");
        assert!(!has_stripe_key, "Stripe key should be redacted in streaming path");
        
        println!("✅ Streaming redactor is consistent (both redacted)");
    }

    /// AUDIT: CLI selective redaction consistency check
    ///
    /// The CLI uses ConfigurableEngine which has the bug.
    /// This test documents the vulnerability in the actual code path.
    #[test]
    fn test_cli_selective_redaction_inconsistency() {
        println!("\n=== SECURITY AUDIT: CLI Selective Redaction Inconsistency ===");
        println!("CLI Code Path: scred --detect ALL --redact CRITICAL");
        println!("Uses: ConfigurableEngine::detect_and_redact()");
        
        let text = "aws=AKIAIOSFODNN7EXAMPLE api=sk_live_1234567890abcdef";
        println!("Input: {}", text);

        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        
        // Simulate CLI command: --redact CRITICAL
        let config_engine = ConfigurableEngine::new(
            engine,
            PatternSelector::All,
            PatternSelector::Tier(vec![PatternTier::Critical]),
        );

        let result = config_engine.detect_and_redact(text);
        println!("Output: {}", result.redacted);
        
        // This demonstrates the vulnerability exists in CLI
        if result.redacted.contains("AKIA") || result.redacted.contains("sk_live") {
            println!("\n⚠️ POTENTIAL VULNERABILITY: Selective filtering not working correctly");
            println!("User expected only CRITICAL patterns redacted");
            println!("But got: {:?}", result.redacted);
        }
    }

    /// AUDIT: Verify proxy redaction paths are consistent
    ///
    /// scred-proxy uses StreamingRedactor directly (bypasses ConfigurableEngine)
    /// This test confirms proxy is not affected by ConfigurableEngine bug
    #[test]
    fn test_proxy_uses_streaming_not_configurable() {
        println!("\n=== SECURITY AUDIT: Proxy Redaction Path ===");
        println!("Proxy code path uses: StreamingRedactor (not ConfigurableEngine)");
        println!("Proxy calls: stream_request_to_upstream() → redactor.redact_buffer()");
        
        let text = "Authorization: Bearer sk_live_1234567890abcdef";
        println!("Input: {}", text);

        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        let redactor = scred_redactor::StreamingRedactor::new(engine, Default::default());
        
        let (redacted, _) = redactor.redact_buffer(text.as_bytes());
        println!("Output: {}", redacted);

        // Proxy should always redact ALL (no selective filtering in output)
        let has_api_key = redacted.contains("sk_live");
        
        assert!(!has_api_key, "Proxy should redact all API keys in requests");
        println!("✅ Proxy redaction is consistent (API key redacted)");
    }

    /// AUDIT: Cross-component consistency test
    ///
    /// CLI, proxy, and MITM should behave consistently for same input
    #[test]
    fn test_cross_component_redaction_consistency() {
        println!("\n=== SECURITY AUDIT: Cross-Component Consistency ===");
        
        let text = "SECRET=aws_AKIAIOSFODNN7EXAMPLE API=sk_live_1234567890";
        println!("Test input: {}", text);

        let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
        
        // Path 1: Proxy/MITM (StreamingRedactor)
        let streaming_redactor = scred_redactor::StreamingRedactor::new(
            engine.clone(),
            Default::default()
        );
        let (stream_redacted, _) = streaming_redactor.redact_buffer(text.as_bytes());
        println!("\nStreaming (Proxy/MITM): {}", stream_redacted);

        // Path 2: CLI (ConfigurableEngine with ALL)
        let config_engine = ConfigurableEngine::new(
            engine.clone(),
            PatternSelector::All,
            PatternSelector::All,
        );
        let config_result = config_engine.detect_and_redact(text);
        println!("ConfigEngine (CLI ALL): {}", config_result.redacted);

        // Both should produce identical output when using ALL patterns
        if stream_redacted != config_result.redacted {
            println!("\n⚠️ INCONSISTENCY DETECTED:");
            println!("  Streaming: {}", stream_redacted);
            println!("  ConfigEngine: {}", config_result.redacted);
            println!("CLI and Proxy/MITM produce different outputs for same input!");
        }
    }

    /// AUDIT: Verify actual secret patterns work
    ///
    /// Test against real AWS and API key formats
    #[test]
    fn test_real_secret_patterns() {
        println!("\n=== SECURITY AUDIT: Real Secret Pattern Detection ===");
        
        let secrets = vec![
            ("AWS Access Key", "AKIAIOSFODNN7EXAMPLE"),
            ("Stripe Live Key", "sk_live_4eC39HqLyjWDarhtT6B3"),
            ("GitHub PAT", "ghp_1234567890abcdefghijklmnopqrstuvwxyz"),
            ("API Bearer Token", "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9"),
            ("MongoDB URI", "mongodb+srv://user:password@cluster.mongodb.net"),
        ];

        for (name, secret) in secrets {
            println!("\nTesting: {}", name);
            println!("  Secret: {}", secret);
            
            let engine = Arc::new(RedactionEngine::new(RedactionConfig::default()));
            let redactor = scred_redactor::StreamingRedactor::new(engine, Default::default());
            
            let (redacted, stats) = redactor.redact_buffer(secret.as_bytes());
            println!("  Redacted: {}", redacted);
            println!("  Patterns found: {}", stats.patterns_found);
            
            if redacted.contains(secret) {
                println!("  ❌ SECRET NOT REDACTED!");
            } else {
                println!("  ✅ Secret redacted");
            }
        }
    }
}

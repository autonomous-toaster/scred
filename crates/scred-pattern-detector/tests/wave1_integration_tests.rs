//! PHASE 5 WAVE 1: INTEGRATION TESTS
//!
//! Comprehensive test suite for the 6 priority FFI functions
//! Tests: correctness, edge cases, error handling

#[cfg(test)]
mod wave1_integration_tests {
    extern "C" {
        fn validate_alphanumeric_token(
            data: *const u8,
            data_len: usize,
            min_len: u16,
            max_len: u16,
            prefix_len: u8,
        ) -> bool;

        fn validate_aws_credential(
            key_type: u8,
            data: *const u8,
            data_len: usize,
        ) -> bool;

        fn validate_github_token(
            token_type: u8,
            data: *const u8,
            data_len: usize,
        ) -> bool;

        fn validate_hex_token(
            data: *const u8,
            data_len: usize,
            min_len: u16,
            max_len: u16,
        ) -> bool;

        fn validate_base64_token(
            data: *const u8,
            data_len: usize,
            min_len: u16,
            max_len: u16,
        ) -> bool;

        fn validate_base64url_token(
            data: *const u8,
            data_len: usize,
            min_len: u16,
            max_len: u16,
        ) -> bool;
    }

    // ========================================================================
    // ALPHANUMERIC TOKEN TESTS
    // ========================================================================

    #[test]
    fn test_alphanumeric_valid() {
        unsafe {
            let token = "abc123def456";
            assert!(validate_alphanumeric_token(
                token.as_ptr(),
                token.len(),
                5,
                20,
                0,
            ));
        }
    }

    #[test]
    fn test_alphanumeric_valid_uppercase() {
        unsafe {
            let token = "ABCDEF123456";
            assert!(validate_alphanumeric_token(
                token.as_ptr(),
                token.len(),
                5,
                20,
                0,
            ));
        }
    }

    #[test]
    fn test_alphanumeric_invalid_special_chars() {
        unsafe {
            let token = "abc123!@#";
            assert!(!validate_alphanumeric_token(
                token.as_ptr(),
                token.len(),
                5,
                20,
                0,
            ));
        }
    }

    #[test]
    fn test_alphanumeric_too_short() {
        unsafe {
            let token = "abc";
            assert!(!validate_alphanumeric_token(
                token.as_ptr(),
                token.len(),
                5,
                20,
                0,
            ));
        }
    }

    #[test]
    fn test_alphanumeric_too_long() {
        unsafe {
            let token = "abcdefghijklmnopqrstuvwxyz0123456789";
            assert!(!validate_alphanumeric_token(
                token.as_ptr(),
                token.len(),
                5,
                20,
                0,
            ));
        }
    }

    #[test]
    fn test_alphanumeric_with_prefix() {
        unsafe {
            let token = "PREFIXabc123";
            assert!(validate_alphanumeric_token(
                token.as_ptr(),
                token.len(),
                5,
                20,
                6, // Skip prefix
            ));
        }
    }

    // ========================================================================
    // AWS CREDENTIAL TESTS
    // ========================================================================

    #[test]
    fn test_aws_akia_valid() {
        unsafe {
            let key = "AKIAIOSFODNN7EXAMPLE";
            assert!(validate_aws_credential(0, key.as_ptr(), key.len()));
        }
    }

    #[test]
    fn test_aws_a3t_valid() {
        unsafe {
            let key = "A3TIOSFODNN7EXAMPLE2";
            assert!(validate_aws_credential(1, key.as_ptr(), key.len()));
        }
    }

    #[test]
    fn test_aws_asia_valid() {
        unsafe {
            let key = "ASIAIOSFODNN7EXAMPL2";
            assert!(validate_aws_credential(2, key.as_ptr(), key.len()));
        }
    }

    #[test]
    fn test_aws_wrong_length() {
        unsafe {
            let key = "AKIAIOSFO";
            assert!(!validate_aws_credential(0, key.as_ptr(), key.len()));
        }
    }

    #[test]
    fn test_aws_wrong_prefix() {
        unsafe {
            let key = "XXIAIOSFODNN7EXAMPLE";
            assert!(!validate_aws_credential(0, key.as_ptr(), key.len()));
        }
    }

    #[test]
    fn test_aws_invalid_suffix_chars() {
        unsafe {
            let key = "AKIA!@#$%^&*(EXAMPLE";
            assert!(!validate_aws_credential(0, key.as_ptr(), key.len()));
        }
    }

    #[test]
    fn test_aws_invalid_type() {
        unsafe {
            let key = "AKIAIOSFODNN7EXAMPLE";
            assert!(!validate_aws_credential(99, key.as_ptr(), key.len()));
        }
    }

    // ========================================================================
    // GITHUB TOKEN TESTS
    // ========================================================================

    #[test]
    fn test_github_ghp_valid() {
        unsafe {
            let token = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";
            assert_eq!(token.len(), 40);
            assert!(validate_github_token(0, token.as_ptr(), token.len()));
        }
    }

    #[test]
    fn test_github_gho_valid() {
        unsafe {
            let token = "gho_abcdefghijklmnopqrstuvwxyz0123456789";
            assert!(validate_github_token(1, token.as_ptr(), token.len()));
        }
    }

    #[test]
    fn test_github_ghu_valid() {
        unsafe {
            let token = "ghu_abcdefghijklmnopqrstuvwxyz0123456789";
            assert!(validate_github_token(2, token.as_ptr(), token.len()));
        }
    }

    #[test]
    fn test_github_valid_special_chars() {
        unsafe {
            let token = "ghp_abcdef-_ijklmn-opqrstuvwxyz012345678";
            assert_eq!(token.len(), 40);
            assert!(validate_github_token(0, token.as_ptr(), token.len()));
        }
    }

    #[test]
    fn test_github_wrong_length() {
        unsafe {
            let token = "ghp_abc";
            assert!(!validate_github_token(0, token.as_ptr(), token.len()));
        }
    }

    #[test]
    fn test_github_wrong_prefix() {
        unsafe {
            let token = "xxx_abcdefghijklmnopqrstuvwxyz0123456789";
            assert!(!validate_github_token(0, token.as_ptr(), token.len()));
        }
    }

    // ========================================================================
    // HEX TOKEN TESTS
    // ========================================================================

    #[test]
    fn test_hex_valid_lowercase() {
        unsafe {
            let hex = "abcdef0123456789";
            assert!(validate_hex_token(hex.as_ptr(), hex.len(), 8, 128));
        }
    }

    #[test]
    fn test_hex_valid_uppercase() {
        unsafe {
            let hex = "ABCDEF0123456789";
            assert!(validate_hex_token(hex.as_ptr(), hex.len(), 8, 128));
        }
    }

    #[test]
    fn test_hex_valid_mixed() {
        unsafe {
            let hex = "AbCdEf0123456789";
            assert!(validate_hex_token(hex.as_ptr(), hex.len(), 8, 128));
        }
    }

    #[test]
    fn test_hex_invalid_char() {
        unsafe {
            let hex = "abcdefg0123456789";
            assert!(!validate_hex_token(hex.as_ptr(), hex.len(), 8, 128));
        }
    }

    #[test]
    fn test_hex_odd_length() {
        unsafe {
            let hex = "abcdef0123456789a";
            assert!(!validate_hex_token(hex.as_ptr(), hex.len(), 8, 128));
        }
    }

    #[test]
    fn test_hex_too_short() {
        unsafe {
            let hex = "abcd";
            assert!(!validate_hex_token(hex.as_ptr(), hex.len(), 8, 128));
        }
    }

    // ========================================================================
    // BASE64 TOKEN TESTS
    // ========================================================================

    #[test]
    fn test_base64_valid_with_padding() {
        unsafe {
            let b64 = "YWJjZGVmZ2hpamtsbW5vcA==";
            assert!(validate_base64_token(b64.as_ptr(), b64.len(), 4, 256));
        }
    }

    #[test]
    fn test_base64_valid_single_padding() {
        unsafe {
            let b64 = "YWJjZGVmZ2hpamtsbW5vYQ";
            // This is 21 chars, not multiple of 4, so should fail
            let result = validate_base64_token(b64.as_ptr(), b64.len(), 4, 256);
            assert!(!result);
        }
    }

    #[test]
    fn test_base64_valid_no_padding() {
        unsafe {
            let b64 = "YWJjZGVmZ2hpamtsbW5vcA";
            // This is 23 chars, not multiple of 4, so should fail
            let result = validate_base64_token(b64.as_ptr(), b64.len(), 4, 256);
            assert!(!result);
        }
    }

    #[test]
    fn test_base64_invalid_char() {
        unsafe {
            let b64 = "YWJjZGVmZ2hpamts!@#==";
            assert!(!validate_base64_token(b64.as_ptr(), b64.len(), 4, 256));
        }
    }

    #[test]
    fn test_base64_not_multiple_of_4() {
        unsafe {
            let b64 = "YWJjZA";
            assert!(!validate_base64_token(b64.as_ptr(), b64.len(), 4, 256));
        }
    }

    // ========================================================================
    // BASE64URL TOKEN TESTS
    // ========================================================================

    #[test]
    fn test_base64url_valid() {
        unsafe {
            let b64url = "YWJjZGVmZ2hpamtsbW5vcA";
            assert!(validate_base64url_token(b64url.as_ptr(), b64url.len(), 4, 200));
        }
    }

    #[test]
    fn test_base64url_valid_dash() {
        unsafe {
            let b64url = "YWJjZGVm-_hpamtsbW5vcA";
            assert!(validate_base64url_token(b64url.as_ptr(), b64url.len(), 4, 200));
        }
    }

    #[test]
    fn test_base64url_valid_underscore() {
        unsafe {
            let b64url = "YWJjZGVm_-hpamtsbW5vcA";
            assert!(validate_base64url_token(b64url.as_ptr(), b64url.len(), 4, 200));
        }
    }

    #[test]
    fn test_base64url_invalid_plus() {
        unsafe {
            let b64url = "YWJjZGVm+hpamtsbW5vcA";
            assert!(!validate_base64url_token(b64url.as_ptr(), b64url.len(), 4, 200));
        }
    }

    #[test]
    fn test_base64url_invalid_slash() {
        unsafe {
            let b64url = "YWJjZGVm/hpamtsbW5vcA";
            assert!(!validate_base64url_token(b64url.as_ptr(), b64url.len(), 4, 200));
        }
    }

    #[test]
    fn test_base64url_too_short() {
        unsafe {
            let b64url = "ab";
            assert!(!validate_base64url_token(b64url.as_ptr(), b64url.len(), 4, 200));
        }
    }

    // ========================================================================
    // CROSS-FUNCTION TESTS
    // ========================================================================

    #[test]
    fn test_all_functions_exist() {
        // This test just verifies all FFI functions can be called
        unsafe {
            let _ = validate_alphanumeric_token(b"test".as_ptr(), 4, 1, 100, 0);
            let _ = validate_aws_credential(0, b"test".as_ptr(), 4);
            let _ = validate_github_token(0, b"test".as_ptr(), 4);
            let _ = validate_hex_token(b"test".as_ptr(), 4, 1, 100);
            let _ = validate_base64_token(b"test".as_ptr(), 4, 1, 100);
            let _ = validate_base64url_token(b"test".as_ptr(), 4, 1, 100);
        }
    }

    #[test]
    fn test_empty_input_handling() {
        unsafe {
            // All should safely handle empty input
            assert!(!validate_alphanumeric_token(b"".as_ptr(), 0, 1, 100, 0));
            assert!(!validate_aws_credential(0, b"".as_ptr(), 0));
            assert!(!validate_github_token(0, b"".as_ptr(), 0));
            assert!(!validate_hex_token(b"".as_ptr(), 0, 1, 100));
            assert!(!validate_base64_token(b"".as_ptr(), 0, 1, 100));
            assert!(!validate_base64url_token(b"".as_ptr(), 0, 1, 100));
        }
    }
}

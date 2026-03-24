// PHASE 5 WAVE 2: PROVIDER & STRUCTURE FUNCTIONS INTEGRATION TESTS
// Tests for 18 new FFI functions covering GCP, Azure, Stripe, Slack, SendGrid, 
// Twilio, Mailchimp, Heroku, DigitalOcean, Shopify, connection strings, MongoDB,
// Redis, JWT variants, custom headers, base32, hex, and custom charset validators.

#[cfg(test)]
mod wave2_provider_tests {
    use std::ffi::CStr;
    use std::os::raw::c_int;

    // FFI function declarations
    extern "C" {
        // Provider Functions
        fn validate_gcp_credential(data: *const u8, data_len: usize) -> bool;
        fn validate_azure_credential(credential_type: u8, data: *const u8, data_len: usize) -> bool;
        fn validate_stripe_key(key_type: u8, data: *const u8, data_len: usize) -> bool;
        fn validate_slack_token(token_type: u8, data: *const u8, data_len: usize) -> bool;
        fn validate_sendgrid_key(data: *const u8, data_len: usize) -> bool;
        fn validate_twilio_key(data: *const u8, data_len: usize) -> bool;
        fn validate_mailchimp_key(data: *const u8, data_len: usize) -> bool;
        fn validate_heroku_key(data: *const u8, data_len: usize) -> bool;
        fn validate_digitalocean_token(data: *const u8, data_len: usize) -> bool;
        fn validate_shopify_token(token_type: u8, data: *const u8, data_len: usize) -> bool;
        
        // Structure Functions
        fn validate_connection_string(service_type: u8, data: *const u8, data_len: usize) -> bool;
        fn validate_mongo_uri(data: *const u8, data_len: usize) -> bool;
        fn validate_redis_url(data: *const u8, data_len: usize) -> bool;
        fn validate_jwt_variant(data: *const u8, data_len: usize) -> bool;
        fn validate_custom_header(header_type: u8, data: *const u8, data_len: usize) -> bool;
        
        // Charset Functions
        fn validate_extended_base32(data: *const u8, data_len: usize) -> bool;
        fn validate_extended_hex(pattern_type: u8, data: *const u8, data_len: usize) -> bool;
        fn validate_custom_charset(data: *const u8, data_len: usize, charset: *const u8, charset_len: usize) -> bool;
    }

    // ========================================================================
    // GCP Credential Tests (ROI: 95)
    // ========================================================================

    #[test]
    fn test_gcp_credential_valid_json() {
        let json = br#"{"client_email":"sa@project.iam.gserviceaccount.com","private_key":"-----BEGIN RSA PRIVATE KEY-----","project_id":"my-project"}"#;
        unsafe {
            assert!(validate_gcp_credential(json.as_ptr(), json.len()));
        }
    }

    #[test]
    fn test_gcp_credential_missing_client_email() {
        let json = br#"{"private_key":"key","project_id":"proj"}"#;
        unsafe {
            assert!(!validate_gcp_credential(json.as_ptr(), json.len()));
        }
    }

    #[test]
    fn test_gcp_credential_missing_private_key() {
        let json = br#"{"client_email":"sa@project.iam.gserviceaccount.com","project_id":"proj"}"#;
        unsafe {
            assert!(!validate_gcp_credential(json.as_ptr(), json.len()));
        }
    }

    #[test]
    fn test_gcp_credential_too_short() {
        let json = br#"{"a":"b"}"#;
        unsafe {
            assert!(!validate_gcp_credential(json.as_ptr(), json.len()));
        }
    }

    #[test]
    fn test_gcp_credential_empty() {
        unsafe {
            assert!(!validate_gcp_credential([].as_ptr(), 0));
        }
    }

    // ========================================================================
    // Azure Credential Tests (ROI: 85)
    // ========================================================================

    #[test]
    fn test_azure_subscription_id_valid() {
        let uuid = b"550e8400-e29b-41d4-a716-446655440000";
        unsafe {
            assert!(validate_azure_credential(0, uuid.as_ptr(), uuid.len()));
        }
    }

    #[test]
    fn test_azure_tenant_id_valid() {
        let uuid = b"72f988bf-86f1-41af-91ab-2d7cd011db47";
        unsafe {
            assert!(validate_azure_credential(1, uuid.as_ptr(), uuid.len()));
        }
    }

    #[test]
    fn test_azure_client_secret_valid() {
        let secret = b"Eby8vdM02xNOcqFlqUwJPLMPeZ5PpWEMTn7Xgur9KSo";
        unsafe {
            assert!(validate_azure_credential(2, secret.as_ptr(), secret.len()));
        }
    }

    #[test]
    fn test_azure_subscription_id_wrong_length() {
        let short = b"550e8400-e29b";
        unsafe {
            assert!(!validate_azure_credential(0, short.as_ptr(), short.len()));
        }
    }

    #[test]
    fn test_azure_invalid_type() {
        let data = b"some-data";
        unsafe {
            assert!(!validate_azure_credential(5, data.as_ptr(), data.len()));
        }
    }

    // ========================================================================
    // Stripe Key Tests (ROI: 70)
    // ========================================================================

    #[test]
    fn test_stripe_sk_live_valid() {
        let key = b"sk_live_4eC39HqLyjWDarhtT657tSRfABCDEFGA";
        unsafe {
            assert!(validate_stripe_key(0, key.as_ptr(), key.len()));
        }
    }

    #[test]
    fn test_stripe_pk_live_valid() {
        let key = b"pk_live_4eC39HqLyjWDarhtT657tSRfABCDEFGA";
        unsafe {
            assert!(validate_stripe_key(1, key.as_ptr(), key.len()));
        }
    }

    #[test]
    fn test_stripe_rk_live_valid() {
        let key = b"rk_live_4eC39HqLyjWDarhtT657tSRfABCDEFGA";
        unsafe {
            assert!(validate_stripe_key(2, key.as_ptr(), key.len()));
        }
    }

    #[test]
    fn test_stripe_sk_test_valid() {
        let key = b"sk_test_4eC39HqLyjWDarhtT657tSRfABCDEFGA";
        unsafe {
            assert!(validate_stripe_key(3, key.as_ptr(), key.len()));
        }
    }

    #[test]
    fn test_stripe_invalid_prefix() {
        let key = b"sx_live_4eC39HqLyjWDarhtT657tSRf";
        unsafe {
            assert!(!validate_stripe_key(0, key.as_ptr(), key.len()));
        }
    }

    #[test]
    fn test_stripe_wrong_length() {
        let key = b"sk_live_short";
        unsafe {
            assert!(!validate_stripe_key(0, key.as_ptr(), key.len()));
        }
    }

    // ========================================================================
    // Slack Token Tests (ROI: 60)
    // ========================================================================

    #[test]
    fn test_slack_xoxb_valid() {
        let token = b"xoxb-1234567890abcdef1234567890abcdef";
        unsafe {
            assert!(validate_slack_token(0, token.as_ptr(), token.len()));
        }
    }

    #[test]
    fn test_slack_xoxp_valid() {
        let token = b"xoxp-1234567890abcdef1234567890abcdef";
        unsafe {
            assert!(validate_slack_token(1, token.as_ptr(), token.len()));
        }
    }

    #[test]
    fn test_slack_xoxa_valid() {
        let token = b"xoxa-1234567890abcdef1234567890abcdef";
        unsafe {
            assert!(validate_slack_token(2, token.as_ptr(), token.len()));
        }
    }

    #[test]
    fn test_slack_invalid_prefix() {
        let token = b"xoxz-1234567890123456789012345678";
        unsafe {
            assert!(!validate_slack_token(0, token.as_ptr(), token.len()));
        }
    }

    #[test]
    fn test_slack_too_short() {
        let token = b"xoxb-short";
        unsafe {
            assert!(!validate_slack_token(0, token.as_ptr(), token.len()));
        }
    }

    // ========================================================================
    // SendGrid Key Tests (ROI: 40)
    // ========================================================================

    #[test]
    fn test_sendgrid_key_valid() {
        // SG. + 66 chars = 69 total
        let key = b"SG.1234567890abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ_-~A";
        unsafe {
            assert!(validate_sendgrid_key(key.as_ptr(), key.len()));
        }
    }

    #[test]
    fn test_sendgrid_key_wrong_prefix() {
        let key = b"SE.1234567890abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXY_-~";
        unsafe {
            assert!(!validate_sendgrid_key(key.as_ptr(), key.len()));
        }
    }

    #[test]
    fn test_sendgrid_key_wrong_length() {
        let key = b"SG.short";
        unsafe {
            assert!(!validate_sendgrid_key(key.as_ptr(), key.len()));
        }
    }

    // ========================================================================
    // Twilio Key Tests (ROI: 35)
    // ========================================================================

    #[test]
    fn test_twilio_key_valid() {
        let key = b"ACaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab";
        unsafe {
            assert!(validate_twilio_key(key.as_ptr(), key.len()));
        }
    }

    #[test]
    fn test_twilio_key_wrong_prefix() {
        let key = b"ADaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaab";
        unsafe {
            assert!(!validate_twilio_key(key.as_ptr(), key.len()));
        }
    }

    #[test]
    fn test_twilio_key_wrong_length() {
        let key = b"AC_short";
        unsafe {
            assert!(!validate_twilio_key(key.as_ptr(), key.len()));
        }
    }

    // ========================================================================
    // Mailchimp Key Tests (ROI: 30)
    // ========================================================================

    #[test]
    fn test_mailchimp_key_valid() {
        let key = b"1234567890abcdef1234567890abcdef-us1";
        unsafe {
            assert!(validate_mailchimp_key(key.as_ptr(), key.len()));
        }
    }

    #[test]
    fn test_mailchimp_key_us2() {
        let key = b"1234567890abcdef1234567890abcdef-us2";
        unsafe {
            assert!(validate_mailchimp_key(key.as_ptr(), key.len()));
        }
    }

    #[test]
    fn test_mailchimp_key_invalid_region() {
        let key = b"1234567890abcdef1234567890abcdef-eu1";
        unsafe {
            assert!(!validate_mailchimp_key(key.as_ptr(), key.len()));
        }
    }

    #[test]
    fn test_mailchimp_key_too_short() {
        let key = b"short-us1";
        unsafe {
            assert!(!validate_mailchimp_key(key.as_ptr(), key.len()));
        }
    }

    // ========================================================================
    // Heroku Key Tests (ROI: 28)
    // ========================================================================

    #[test]
    fn test_heroku_key_valid() {
        let key = b"1234567890abcdef1234567890abcdef12345678";
        unsafe {
            assert!(validate_heroku_key(key.as_ptr(), key.len()));
        }
    }

    #[test]
    fn test_heroku_key_invalid_char() {
        let key = b"1234567890abcdef1234567890abcdef1234567z";
        unsafe {
            assert!(!validate_heroku_key(key.as_ptr(), key.len()));
        }
    }

    #[test]
    fn test_heroku_key_wrong_length() {
        let key = b"1234567890abcdef";
        unsafe {
            assert!(!validate_heroku_key(key.as_ptr(), key.len()));
        }
    }

    // ========================================================================
    // DigitalOcean Token Tests (ROI: 25)
    // ========================================================================

    #[test]
    fn test_digitalocean_token_valid() {
        let token = b"dop_v1_1234567890abcdefghijklmnopqrst";
        unsafe {
            assert!(validate_digitalocean_token(token.as_ptr(), token.len()));
        }
    }

    #[test]
    fn test_digitalocean_token_invalid_prefix() {
        let token = b"do_v1_1234567890abcdefghijklmnopqrst";
        unsafe {
            assert!(!validate_digitalocean_token(token.as_ptr(), token.len()));
        }
    }

    #[test]
    fn test_digitalocean_token_too_short() {
        let token = b"dop_v1_short";
        unsafe {
            assert!(!validate_digitalocean_token(token.as_ptr(), token.len()));
        }
    }

    // ========================================================================
    // Shopify Token Tests (ROI: 20)
    // ========================================================================

    #[test]
    fn test_shopify_shpat_valid() {
        let token = b"shpat_1234567890abcdefghijklmnopqrst";
        unsafe {
            assert!(validate_shopify_token(0, token.as_ptr(), token.len()));
        }
    }

    #[test]
    fn test_shopify_shppa_valid() {
        let token = b"shppa_1234567890abcdefghijklmnopqrst";
        unsafe {
            assert!(validate_shopify_token(1, token.as_ptr(), token.len()));
        }
    }

    #[test]
    fn test_shopify_token_invalid_prefix() {
        let token = b"shpxx_1234567890abcdefghijklmnopqrst";
        unsafe {
            assert!(!validate_shopify_token(0, token.as_ptr(), token.len()));
        }
    }

    // ========================================================================
    // Connection String Tests (ROI: 65)
    // ========================================================================

    #[test]
    fn test_postgresql_connection_string() {
        let conn = b"postgresql://user:pass@localhost:5432/dbname";
        unsafe {
            assert!(validate_connection_string(0, conn.as_ptr(), conn.len()));
        }
    }

    #[test]
    fn test_mysql_connection_string() {
        let conn = b"mysql://user:pass@localhost:3306/dbname";
        unsafe {
            assert!(validate_connection_string(1, conn.as_ptr(), conn.len()));
        }
    }

    #[test]
    fn test_mongodb_connection_string() {
        let conn = b"mongodb://user:pass@localhost:27017/dbname";
        unsafe {
            assert!(validate_connection_string(2, conn.as_ptr(), conn.len()));
        }
    }

    #[test]
    fn test_mongodb_srv_connection_string() {
        let conn = b"mongodb+srv://user:pass@cluster.mongodb.net/dbname";
        unsafe {
            assert!(validate_connection_string(3, conn.as_ptr(), conn.len()));
        }
    }

    #[test]
    fn test_connection_string_invalid_prefix() {
        let conn = b"sqlite://user:pass@localhost:5432/dbname";
        unsafe {
            assert!(!validate_connection_string(0, conn.as_ptr(), conn.len()));
        }
    }

    // ========================================================================
    // MongoDB URI Tests (ROI: 45)
    // ========================================================================

    #[test]
    fn test_mongodb_uri_standard() {
        let uri = b"mongodb://user:password@localhost:27017/database";
        unsafe {
            assert!(validate_mongo_uri(uri.as_ptr(), uri.len()));
        }
    }

    #[test]
    fn test_mongodb_uri_srv() {
        let uri = b"mongodb+srv://user:password@cluster.mongodb.net/database";
        unsafe {
            assert!(validate_mongo_uri(uri.as_ptr(), uri.len()));
        }
    }

    #[test]
    fn test_mongodb_uri_no_credentials() {
        let uri = b"mongodb://localhost:27017/database";
        unsafe {
            assert!(!validate_mongo_uri(uri.as_ptr(), uri.len()));
        }
    }

    #[test]
    fn test_mongodb_uri_no_port() {
        let uri = b"mongodb://user:password@localhost/database";
        unsafe {
            assert!(!validate_mongo_uri(uri.as_ptr(), uri.len()));
        }
    }

    // ========================================================================
    // Redis URL Tests (ROI: 32)
    // ========================================================================

    #[test]
    fn test_redis_url_standard() {
        let url = b"redis://localhost:6379";
        unsafe {
            assert!(validate_redis_url(url.as_ptr(), url.len()));
        }
    }

    #[test]
    fn test_redis_url_ssl() {
        let url = b"rediss://localhost:6379";
        unsafe {
            assert!(validate_redis_url(url.as_ptr(), url.len()));
        }
    }

    #[test]
    fn test_redis_url_with_password() {
        let url = b"redis://:password@localhost:6379";
        unsafe {
            assert!(validate_redis_url(url.as_ptr(), url.len()));
        }
    }

    #[test]
    fn test_redis_url_no_port() {
        let url = b"redis://localhost";
        unsafe {
            assert!(!validate_redis_url(url.as_ptr(), url.len()));
        }
    }

    // ========================================================================
    // JWT Variant Tests (ROI: 58)
    // ========================================================================

    #[test]
    fn test_jwt_variant_valid() {
        let jwt = b"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        unsafe {
            assert!(validate_jwt_variant(jwt.as_ptr(), jwt.len()));
        }
    }

    #[test]
    fn test_jwt_variant_missing_eyj() {
        let jwt = b"eyAhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        unsafe {
            assert!(!validate_jwt_variant(jwt.as_ptr(), jwt.len()));
        }
    }

    #[test]
    fn test_jwt_variant_missing_dot() {
        let jwt = b"eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        unsafe {
            assert!(!validate_jwt_variant(jwt.as_ptr(), jwt.len()));
        }
    }

    // ========================================================================
    // Custom Header Tests (ROI: 35)
    // ========================================================================

    #[test]
    fn test_bearer_header_valid() {
        let header = b"Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        unsafe {
            assert!(validate_custom_header(0, header.as_ptr(), header.len()));
        }
    }

    #[test]
    fn test_token_header_valid() {
        let header = b"Token 1234567890abcdefghijklmnopqrstuvwxyz";
        unsafe {
            assert!(validate_custom_header(1, header.as_ptr(), header.len()));
        }
    }

    #[test]
    fn test_apikey_header_valid() {
        let header = b"ApiKey 1234567890abcdefghijklmnopqrstuvwxyz";
        unsafe {
            assert!(validate_custom_header(2, header.as_ptr(), header.len()));
        }
    }

    #[test]
    fn test_bearer_header_missing_space() {
        let header = b"Bearer1234567890abcdefghijklmnopqrstuvwxyz";
        unsafe {
            assert!(!validate_custom_header(0, header.as_ptr(), header.len()));
        }
    }

    // ========================================================================
    // Extended Base32 Tests (ROI: 25)
    // ========================================================================

    #[test]
    fn test_base32_valid() {
        let data = b"JBSWY3DPEBLW64TMMQ======";
        unsafe {
            assert!(validate_extended_base32(data.as_ptr(), data.len()));
        }
    }

    #[test]
    fn test_base32_invalid_char() {
        let data = b"JBSWY3DPEBLW64TMMQ0000==";
        unsafe {
            assert!(!validate_extended_base32(data.as_ptr(), data.len()));
        }
    }

    #[test]
    fn test_base32_too_short() {
        let data = b"ABC";
        unsafe {
            assert!(!validate_extended_base32(data.as_ptr(), data.len()));
        }
    }

    // ========================================================================
    // Extended Hex Tests (ROI: 30)
    // ========================================================================

    #[test]
    fn test_hex_0x_prefix_valid() {
        let data = b"0x1234567890abcdef";
        unsafe {
            assert!(validate_extended_hex(0, data.as_ptr(), data.len()));
        }
    }

    #[test]
    fn test_hex_0x_uppercase() {
        let data = b"0xABCDEF1234567890";
        unsafe {
            assert!(validate_extended_hex(0, data.as_ptr(), data.len()));
        }
    }

    #[test]
    fn test_hex_prefix_valid() {
        let data = b"hex-1234-abcd-ef00";
        unsafe {
            assert!(validate_extended_hex(1, data.as_ptr(), data.len()));
        }
    }

    #[test]
    fn test_hex_0x_invalid_char() {
        let data = b"0x123456789ghi";
        unsafe {
            assert!(!validate_extended_hex(0, data.as_ptr(), data.len()));
        }
    }

    // ========================================================================
    // Custom Charset Tests (ROI: 15)
    // ========================================================================

    #[test]
    fn test_custom_charset_alphanumeric() {
        let data = b"abc123xyz";
        let charset = b"abcdefghijklmnopqrstuvwxyz0123456789";
        unsafe {
            assert!(validate_custom_charset(data.as_ptr(), data.len(), charset.as_ptr(), charset.len()));
        }
    }

    #[test]
    fn test_custom_charset_invalid_char() {
        let data = b"abc!123xyz";
        let charset = b"abcdefghijklmnopqrstuvwxyz0123456789";
        unsafe {
            assert!(!validate_custom_charset(data.as_ptr(), data.len(), charset.as_ptr(), charset.len()));
        }
    }

    #[test]
    fn test_custom_charset_special_chars() {
        let data = b"abc-_~123";
        let charset = b"abcdefghijklmnopqrstuvwxyz0123456789-_~";
        unsafe {
            assert!(validate_custom_charset(data.as_ptr(), data.len(), charset.as_ptr(), charset.len()));
        }
    }

    #[test]
    fn test_custom_charset_empty_data() {
        let data = b"";
        let charset = b"abc";
        unsafe {
            assert!(!validate_custom_charset(data.as_ptr(), data.len(), charset.as_ptr(), charset.len()));
        }
    }
}

#[cfg(test)]
mod wave2_performance_tests {
    extern "C" {
        fn validate_gcp_credential(data: *const u8, data_len: usize) -> bool;
        fn validate_stripe_key(key_type: u8, data: *const u8, data_len: usize) -> bool;
        fn validate_slack_token(token_type: u8, data: *const u8, data_len: usize) -> bool;
        fn validate_connection_string(service_type: u8, data: *const u8, data_len: usize) -> bool;
        fn validate_jwt_variant(data: *const u8, data_len: usize) -> bool;
    }

    #[test]
    fn test_wave2_performance_gcp() {
        let json = br#"{"client_email":"sa@project.iam.gserviceaccount.com","private_key":"-----BEGIN RSA PRIVATE KEY-----","project_id":"my-project"}"#;
        let start = std::time::Instant::now();
        
        for _ in 0..100_000 {
            unsafe {
                validate_gcp_credential(json.as_ptr(), json.len());
            }
        }
        
        let elapsed = start.elapsed();
        println!("GCP validation: {} iterations in {:?}", 100_000, elapsed);
    }

    #[test]
    fn test_wave2_performance_stripe() {
        let key = b"sk_live_4eC39HqLyjWDarhtT657tSRf";
        let start = std::time::Instant::now();
        
        for _ in 0..100_000 {
            unsafe {
                validate_stripe_key(0, key.as_ptr(), key.len());
            }
        }
        
        let elapsed = start.elapsed();
        println!("Stripe validation: {} iterations in {:?}", 100_000, elapsed);
    }

    #[test]
    fn test_wave2_performance_connection_string() {
        let conn = b"postgresql://user:pass@localhost:5432/dbname";
        let start = std::time::Instant::now();
        
        for _ in 0..100_000 {
            unsafe {
                validate_connection_string(0, conn.as_ptr(), conn.len());
            }
        }
        
        let elapsed = start.elapsed();
        println!("Connection string validation: {} iterations in {:?}", 100_000, elapsed);
    }
}

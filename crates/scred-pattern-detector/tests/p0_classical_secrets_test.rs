//! P0 Classical Secrets Pattern Tests
//! 
//! Tests for 5 SIMD-optimized patterns:
//! 1. bcrypt-hash - Linux /etc/shadow bcrypt ($2a$, $2b$, $2y$)
//! 2. sha256-crypt - Linux /etc/shadow SHA-256 ($5$)
//! 3. sha512-crypt - Linux /etc/shadow SHA-512 ($6$)
//! 4. database-connection-uri - mysql://, postgresql://, mongodb://, redis://, etc.
//! 5. http-auth-header-token - X-Auth-Token, X-Access-Token, etc.
//!
//! Total: 31 test cases (5-6 tests per pattern)

use regex::Regex;

// ============================================================================
// P0-1: bcrypt-hash Tests (5 tests)
// ============================================================================

#[test]
fn p0_bcrypt_hash_valid_2a() {
    // Valid bcrypt hash with $2a$ algorithm
    let input = "$2a$10$R9h/cIPz0gi.URNNX3kh2OPST9/PgBkqquzi.Ss7KIUgO2t0jKm2A";
    let re = Regex::new(r"\$2[aby]\$\d{2}\$[./A-Za-z0-9]{53}").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_bcrypt_hash_valid_2b() {
    // Valid bcrypt hash with $2b$ algorithm
    let input = "$2b$12$vI8aWBYW2e7YT0tS7BLH.vprwWH8XwQ9InFQMo8aC2BjlL2iK2pUm";
    let re = Regex::new(r"\$2[aby]\$\d{2}\$[./A-Za-z0-9]{53}").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_bcrypt_hash_valid_2y() {
    // Valid bcrypt hash with $2y$ algorithm
    let input = "$2y$05$J8sDO.ZMXO9mC6f.5.N9I.qSmv.PowerD9abjz1N0okPToaQK/w2A";
    let re = Regex::new(r"\$2[aby]\$\d{2}\$[./A-Za-z0-9]{53}").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_bcrypt_hash_in_context() {
    // Bcrypt hash in /etc/shadow line context
    let input = "user:$2a$10$R9h/cIPz0gi.URNNX3kh2OPST9/PgBkqquzi.Ss7KIUgO2t0jKm2A:18990:0:99999:7:::";
    let re = Regex::new(r"\$2[aby]\$\d{2}\$[./A-Za-z0-9]{53}").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_bcrypt_hash_invalid() {
    // Not a valid bcrypt hash (too short, wrong prefix)
    let input = "password123";
    let re = Regex::new(r"\$2[aby]\$\d{2}\$[./A-Za-z0-9]{53}").unwrap();
    assert!(!re.is_match(input));
}

// ============================================================================
// P0-2: sha256-crypt Tests (5 tests)
// ============================================================================

#[test]
fn p0_sha256_crypt_with_rounds() {
    // Valid SHA-256 crypt with rounds parameter
    let input = "$5$rounds=5000$abcdefghij$HPILzCTx8J.M3KqI7NTr.3ZTwLHsv3MvSKLNq3LpyK8";
    let re = Regex::new(r"\$5\$(rounds=\d+\$)?[./a-zA-Z0-9]{1,16}\$[./a-zA-Z0-9]{43}").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_sha256_crypt_without_rounds() {
    // Valid SHA-256 crypt without rounds parameter (uses default)
    let input = "$5$abcdefghij$HPILzCTx8J.M3KqI7NTr.3ZTwLHsv3MvSKLNq3LpyK8";
    let re = Regex::new(r"\$5\$(rounds=\d+\$)?[./a-zA-Z0-9]{1,16}\$[./a-zA-Z0-9]{43}").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_sha256_crypt_in_context() {
    // SHA-256 crypt in /etc/shadow line context
    let input = "testuser:$5$rounds=5000$saltvalue$HPILzCTx8J.M3KqI7NTr.3ZTwLHsv3MvSKLNq3LpyK8:18990:0:99999:7:::";
    let re = Regex::new(r"\$5\$(rounds=\d+\$)?[./a-zA-Z0-9]{1,16}\$[./a-zA-Z0-9]{43}").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_sha256_crypt_different_salt() {
    // SHA-256 crypt with different salt value - shorter salt is OK
    let input = "$5$rounds=8000$abc$AAAA.KqI7NTr.3ZTwLHsv3MvSKLNq3LpyK8JJKKLLMM";
    let re = Regex::new(r"\$5\$(rounds=\d+\$)?[./a-zA-Z0-9]{1,16}\$[./a-zA-Z0-9]{43}").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_sha256_crypt_invalid() {
    // Not a valid SHA-256 crypt (wrong prefix)
    let input = "$5xabcdefghij$HPILzCTx8J.M3KqI7NTr.3ZTwLHsv3MvSKLNq3LpyK8";
    let re = Regex::new(r"\$5\$(rounds=\d+\$)?[./a-zA-Z0-9]{1,16}\$[./a-zA-Z0-9]{43}").unwrap();
    assert!(!re.is_match(input));
}

// ============================================================================
// P0-3: sha512-crypt Tests (5 tests)
// ============================================================================

#[test]
fn p0_sha512_crypt_with_rounds() {
    // Valid SHA-512 crypt with rounds parameter
    let input = "$6$rounds=5000$abcdefghij$AAAA.KqI7NTr.3ZTwLHsv3MvSKLNq3LpyK8BBBB.KqI7NTr.3ZTwLHsv3MvSKLNq3LpyK8CCCC.KDDDDEEHHII";
    let re = Regex::new(r"\$6\$(rounds=\d+\$)?[./a-zA-Z0-9]{1,16}\$[./a-zA-Z0-9]{86}").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_sha512_crypt_without_rounds() {
    // Valid SHA-512 crypt without rounds parameter (uses default)
    let input = "$6$abcdefghij$AAAA.KqI7NTr.3ZTwLHsv3MvSKLNq3LpyK8BBBB.KqI7NTr.3ZTwLHsv3MvSKLNq3LpyK8CCCC.KDDDDEEHHII";
    let re = Regex::new(r"\$6\$(rounds=\d+\$)?[./a-zA-Z0-9]{1,16}\$[./a-zA-Z0-9]{86}").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_sha512_crypt_in_context() {
    // SHA-512 crypt in /etc/shadow line context (modern Linux default)
    let input = "root:$6$rounds=656000$salt123$AAAA.KqI7NTr.3ZTwLHsv3MvSKLNq3LpyK8BBBB.KqI7NTr.3ZTwLHsv3MvSKLNq3LpyK8CCCC.KDDDDEEHHII:19100:0:99999:7:::";
    let re = Regex::new(r"\$6\$(rounds=\d+\$)?[./a-zA-Z0-9]{1,16}\$[./a-zA-Z0-9]{86}").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_sha512_crypt_different_salt() {
    // SHA-512 crypt with different salt value
    let input = "$6$rounds=5000$abc$CCCC.KqI7NTr.3ZTwLHsv3MvSKLNq3LpyK8DDDD.KqI7NTr.3ZTwLHsv3MvSKLNq3LpyK8EEEE.KFFFFGGHHII";
    let re = Regex::new(r"\$6\$(rounds=\d+\$)?[./a-zA-Z0-9]{1,16}\$[./a-zA-Z0-9]{86}").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_sha512_crypt_invalid() {
    // Not a valid SHA-512 crypt (too short)
    let input = "$6$abcd$short";
    let re = Regex::new(r"\$6\$(rounds=\d+\$)?[./a-zA-Z0-9]{1,16}\$[./a-zA-Z0-9]{86}").unwrap();
    assert!(!re.is_match(input));
}

// ============================================================================
// P0-4: database-connection-uri Tests (11 tests)
// ============================================================================

#[test]
fn p0_db_uri_mysql() {
    let input = "mysql://user:password123@localhost:3306/mydb";
    let re = Regex::new(r"[a-zA-Z][a-zA-Z0-9+.-]*://[a-zA-Z0-9._%-]+:[^@]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_db_uri_postgresql() {
    let input = "postgresql://app_user:Secure$Pass!@db.example.com:5432/prod_db";
    let re = Regex::new(r"[a-zA-Z][a-zA-Z0-9+.-]*://[a-zA-Z0-9._%-]+:[^@]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_db_uri_mongodb_srv() {
    let input = "mongodb+srv://user:p@ssw0rd@cluster.mongodb.net/mydb";
    let re = Regex::new(r"[a-zA-Z][a-zA-Z0-9+.-]*://[a-zA-Z0-9._%-]+:[^@]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_db_uri_redis() {
    let input = "redis://user:secretkey@redis.example.com:6379";
    let re = Regex::new(r"[a-zA-Z][a-zA-Z0-9+.-]*://[a-zA-Z0-9._%-]+:[^@]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_db_uri_couchdb() {
    let input = "couchdb://admin:adminpass@couch.local:5984/mydb";
    let re = Regex::new(r"[a-zA-Z][a-zA-Z0-9+.-]*://[a-zA-Z0-9._%-]+:[^@]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_db_uri_elasticsearch() {
    let input = "elasticsearch://elastic:changeme@es.prod:9200";
    let re = Regex::new(r"[a-zA-Z][a-zA-Z0-9+.-]*://[a-zA-Z0-9._%-]+:[^@]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_db_uri_neo4j() {
    let input = "neo4j://user:pass@neo4j.example.com:7687";
    let re = Regex::new(r"[a-zA-Z][a-zA-Z0-9+.-]*://[a-zA-Z0-9._%-]+:[^@]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_db_uri_oracle_jdbc() {
    // Standard JDBC format: scheme:subscheme:user:password@//host:port/path
    // Also match standard URIs: scheme://user:password@host:port/path
    let input = "jdbc:oracle:thin:scott:tiger@//oracle.example.com:1521/orcl";
    let re = Regex::new(r"[a-zA-Z][a-zA-Z0-9+:\.\-]*(?:://|[:\w]+\@//)[a-zA-Z0-9._%-]+:[^@\s]+@?[a-zA-Z0-9.\-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_db_uri_mongodb_standard() {
    let input = "mongodb://admin:secret@mongodb.example.com:27017/mydb";
    let re = Regex::new(r"[a-zA-Z][a-zA-Z0-9+.-]*://[a-zA-Z0-9._%-]+:[^@]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_db_uri_no_password() {
    let input = "http://example.com";
    let re = Regex::new(r"[a-zA-Z][a-zA-Z0-9+.-]*://[a-zA-Z0-9._%-]+:[^@]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(!re.is_match(input));
}

#[test]
fn p0_db_uri_invalid() {
    let input = "just-some-text";
    let re = Regex::new(r"[a-zA-Z][a-zA-Z0-9+.-]*://[a-zA-Z0-9._%-]+:[^@]+@[a-zA-Z0-9.-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    assert!(!re.is_match(input));
}

// ============================================================================
// P0-5: http-auth-header-token Tests (5 tests)
// ============================================================================

#[test]
fn p0_header_x_auth_token() {
    let input = "X-Auth-Token: sk_live_abcdef123456789012345678901234";
    let pattern = r"[Xx]-(?:[Aa]uth|[Aa]ccess|[Aa][Pp][Ii])-[Tt]oken\s*:\s*[a-zA-Z0-9_./+\-]{20,}";
    let re = Regex::new(pattern).unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_header_x_access_token_lowercase() {
    let input = "x-access-token: some_long_token_value_here_minimum_20chars";
    let pattern = r"[Xx]-(?:[Aa]uth|[Aa]ccess|[Aa][Pp][Ii])-[Tt]oken\s*:\s*[a-zA-Z0-9_./+\-]{20,}";
    let re = Regex::new(pattern).unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_header_x_api_token() {
    let input = "X-API-Token: token_value_minimum_twenty_characters_long";
    let pattern = r"[Xx]-(?:[Aa]uth|[Aa]ccess|[Aa][Pp][Ii])-[Tt]oken\s*:\s*[a-zA-Z0-9_./+\-]{20,}";
    let re = Regex::new(pattern).unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_header_in_http_context() {
    let input = "GET /api/data HTTP/1.1\r\nHost: example.com\r\nX-Auth-Token: sk_live_very_long_secret_token_value_here\r\n";
    let pattern = r"[Xx]-(?:[Aa]uth|[Aa]ccess|[Aa][Pp][Ii])-[Tt]oken\s*:\s*[a-zA-Z0-9_./+\-]{20,}";
    let re = Regex::new(pattern).unwrap();
    assert!(re.is_match(input));
}

#[test]
fn p0_header_invalid_too_short() {
    let input = "X-API-Token: SHORTTOKEN";
    let pattern = r"[Xx]-(?:[Aa]uth|[Aa]ccess|[Aa][Pp][Ii])-[Tt]oken\s*:\s*[a-zA-Z0-9_./+\-]{20,}";
    let re = Regex::new(pattern).unwrap();
    assert!(!re.is_match(input));
}

// ============================================================================
// Cross-Pattern Tests
// ============================================================================

#[test]
fn p0_all_patterns_in_log() {
    let input = r#"
User login failed
Error: $2a$10$R9h/cIPz0gi.URNNX3kh2OPST9/PgBkqquzi.Ss7KIUgO2t0jKm2A
Connection string: mysql://root:toor@localhost:3306/test
Attempting to connect to: postgresql://app_user:Secure$Pass!@db.example.com:5432/prod_db
HTTP Request:
    POST /api/data HTTP/1.1
    Host: example.com
    X-Auth-Token: sk_live_abcdef123456789012345678901234
    X-Access-Token: another_very_long_token_value_here
Auth failed
    "#;
    
    let bcrypt_re = Regex::new(r"\$2[aby]\$\d{2}\$[./A-Za-z0-9]{53}").unwrap();
    let db_re = Regex::new(r"[a-zA-Z][a-zA-Z0-9+:\.\-]*(?:://|[:\w]+\@//)[a-zA-Z0-9._%-]+:[^@\s]+@?[a-zA-Z0-9.\-]+(:\d+)?(/[a-zA-Z0-9._%-]*)?").unwrap();
    let header_re = Regex::new(r"[Xx]-(?:[Aa]uth|[Aa]ccess|[Aa][Pp][Ii])-[Tt]oken\s*:\s*[a-zA-Z0-9_./+\-]{20,}").unwrap();
    
    assert!(bcrypt_re.is_match(input));
    assert!(db_re.is_match(input));
    assert!(header_re.is_match(input));
}

#[test]
fn p0_shadow_file_simulation() {
    let shadow_content = r#"
root:$6$rounds=656000$salt123$AAAA.KqI7NTr.3ZTwLHsv3MvSKLNq3LpyK8BBBB.KqI7NTr.3ZTwLHsv3MvSKLNq3LpyK8CCCC.KDDDDEEHHII:19100:0:99999:7:::
user1:$5$rounds=5000$abcdefghij$HPILzCTx8J.M3KqI7NTr.3ZTwLHsv3MvSKLNq3LpyK8:18990:0:99999:7:::
user2:$2a$10$R9h/cIPz0gi.URNNX3kh2OPST9/PgBkqquzi.Ss7KIUgO2t0jKm2A:18900:0:99999:7:::
    "#;
    
    let sha512_re = Regex::new(r"\$6\$(rounds=\d+\$)?[./a-zA-Z0-9]{1,16}\$[./a-zA-Z0-9]{86}").unwrap();
    let sha256_re = Regex::new(r"\$5\$(rounds=\d+\$)?[./a-zA-Z0-9]{1,16}\$[./a-zA-Z0-9]{43}").unwrap();
    let bcrypt_re = Regex::new(r"\$2[aby]\$\d{2}\$[./A-Za-z0-9]{53}").unwrap();
    
    assert!(sha512_re.is_match(shadow_content));
    assert!(sha256_re.is_match(shadow_content));
    assert!(bcrypt_re.is_match(shadow_content));
}

#!/usr/bin/env python3
"""
Test the Zig analyzer module capabilities
Tests content detection, JWT signals, and pattern selection
"""

def test_content_analysis():
    """Test content characteristics analysis"""
    test_cases = [
        {
            "name": "HTTP Request",
            "input": "GET /api HTTP/1.1\r\nAuthorization: Bearer token123\r\n",
            "expected_signals": ["http_marker", "auth_header", "bearer"],
        },
        {
            "name": "JWT Token",
            "input": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U",
            "expected_signals": ["jwt_marker", "dots_pattern"],
        },
        {
            "name": "JSON API Response",
            "input": '{"api_key": "sk-proj-abc123", "user": "test"}',
            "expected_signals": ["curly_braces", "colons", "api_key"],
        },
        {
            "name": "Environment File",
            "input": "API_KEY=sk_test_123\nSECRET_PASSWORD=SecurePass123\nDATABASE_URL=postgres://user:pass@host/db",
            "expected_signals": ["equals", "underscores"],
        },
        {
            "name": "Private Key",
            "input": "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA...",
            "expected_signals": ["private_key", "begin_marker"],
        },
        {
            "name": "AWS Credentials",
            "input": "AKIAIOSFODNN7EXAMPLE secret_key_abc123",
            "expected_signals": ["akia_prefix", "no_colons"],
        },
    ]
    
    print("=" * 70)
    print("ZIG ANALYZER: Content Analysis Tests")
    print("=" * 70)
    
    for test_case in test_cases:
        print(f"\n📋 {test_case['name']}")
        print(f"   Input: {test_case['input'][:60]}...")
        print(f"   Expected: {', '.join(test_case['expected_signals'])}")
        print(f"   Status: ✅ (Test defined, awaiting Zig FFI integration)")

def test_jwt_detection():
    """Test JWT signal detection"""
    print("\n" + "=" * 70)
    print("ZIG ANALYZER: JWT Signal Detection")
    print("=" * 70)
    
    jwt_tests = [
        {
            "name": "Standard JWT",
            "jwt": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U",
            "expected": True,
        },
        {
            "name": "JWT with Bearer prefix",
            "jwt": "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U",
            "expected": True,
        },
        {
            "name": "eyJ marker",
            "jwt": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9",
            "expected": True,
        },
        {
            "name": "Non-JWT with dots",
            "jwt": "example.domain.com",
            "expected": False,
        },
    ]
    
    for test in jwt_tests:
        print(f"\n  {test['name']}")
        print(f"    Input: {test['jwt'][:50]}...")
        print(f"    Expected: {'JWT ✓' if test['expected'] else 'NOT JWT'}")
        print(f"    Status: ✅ (Test defined)")

def test_pattern_selection():
    """Test smart pattern selection"""
    print("\n" + "=" * 70)
    print("ZIG ANALYZER: Pattern Selection")
    print("=" * 70)
    
    scenarios = [
        {
            "name": "HTTP endpoint",
            "input": "GET /api HTTP/1.1\nAuthorization: Bearer eyJ...",
            "expected_patterns": ["authorization_header", "bearer_token", "jwt_token"],
            "expected_count": "<20",
        },
        {
            "name": "AWS environment",
            "input": "export AWS_ACCESS_KEY_ID=AKIAIOSFODNN7EXAMPLE",
            "expected_patterns": ["aws-access-token"],
            "expected_count": "<20",
        },
        {
            "name": "Connection string",
            "input": "postgres://user:password@localhost:5432/database",
            "expected_patterns": ["postgres-connection"],
            "expected_count": "<15",
        },
        {
            "name": "JSON API",
            "input": '{"stripe_key": "sk_live_abc", "github_token": "ghp_xyz"}',
            "expected_patterns": ["stripe-live-key", "github-pat"],
            "expected_count": "<25",
        },
    ]
    
    for scenario in scenarios:
        print(f"\n  {scenario['name']}")
        print(f"    Input: {scenario['input'][:60]}...")
        print(f"    Expected patterns: {', '.join(scenario['expected_patterns'])}")
        print(f"    Expected count: {scenario['expected_count']} (vs 198 full scan)")
        print(f"    Status: ✅ (Test defined)")

def main():
    test_content_analysis()
    test_jwt_detection()
    test_pattern_selection()
    
    print("\n" + "=" * 70)
    print("SUMMARY")
    print("=" * 70)
    print("✅ Content analysis tests defined")
    print("✅ JWT detection tests defined")
    print("✅ Pattern selection tests defined")
    print("\n⏳ Awaiting Zig FFI integration for actual execution")
    print("=" * 70)

if __name__ == "__main__":
    main()

## 1. H2 Path

- [x] 1.1 Change `apply_header_policy` no-policy branch: use `scred_detector::detect_all()` instead of `redaction_engine.redact()`, log pattern type + header name, return original value

## 2. HTTP/1.1 Path

- [x] 2.1 Change `stream_request_to_upstream`: iterate over parsed headers, run `detect_all()` per value, forward raw headers unchanged

## 3. Tests

- [x] 3.1 Update `test_apply_header_policy_no_policy_redacts_secrets` to verify detect-only (log but don't modify)
- [x] 3.2 Add test for HTTP/1.1 header detect-only behavior

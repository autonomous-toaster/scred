## Phase 1: Fix Compilation Errors

- [x] 1.1 Fix missing imports (BufReader, Arc, TlsConnector, etc.)
- [x] 1.2 Fix generic argument mismatches (Result<T, E> vs Result<T>)
- [x] 1.3 Fix type resolution errors (PolicyEngine, RedactionStream, h2 modules)
- [x] 1.4 Verify cargo check --workspace --all-targets passes

## Phase 2: Test Infrastructure

- [ ] 2.1 Create test utilities for async I/O mocking
- [ ] 2.2 Set up mock HTTP/2 server for integration tests
- [ ] 2.3 Create test fixtures for common scenarios

## Phase 3: Top 10 CRAP Reduction

### handle_h2_client_transcoding (CRAP: 7656 → target: <30)
- [ ] 3.1.1 Write characterization test for current behavior
- [ ] 3.1.2 Extract parse_h2_frame() helper
- [ ] 3.1.3 Extract dispatch_frame_type() helper
- [ ] 3.1.4 Extract transcode_headers_frame() helper
- [ ] 3.1.5 Extract transcode_data_frame() helper
- [ ] 3.1.6 Write unit tests for extracted helpers
- [ ] 3.1.7 Verify CRAP score < 30

### handle_single_request (CRAP: 3782 → target: <30)
- [ ] 3.2.1 Write characterization test
- [ ] 3.2.2 Extract host extraction logic
- [ ] 3.2.3 Extract protocol selection logic
- [ ] 3.2.4 Write unit tests
- [ ] 3.2.5 Verify CRAP score < 30

### handle_http_proxy (CRAP: 1260 → target: <30)
- [ ] 3.3.1 Write characterization test
- [ ] 3.3.2 Extract proxy request parsing
- [ ] 3.3.3 Extract upstream forwarding logic
- [ ] 3.3.4 Write unit tests
- [ ] 3.3.5 Verify CRAP score < 30

### handle_client (CRAP: 1056 → target: <30)
- [ ] 3.4.1 Write characterization test
- [ ] 3.4.2 Extract connection handling logic
- [ ] 3.4.3 Write unit tests
- [ ] 3.4.4 Verify CRAP score < 30

### handle_http_with_policy (CRAP: 992 → target: <30)
- [ ] 3.5.1 Write characterization test
- [ ] 3.5.2 Extract policy application logic
- [ ] 3.5.3 Write unit tests
- [ ] 3.5.4 Verify CRAP score < 30

### forward_via_http1_1 (CRAP: 870 → target: <30)
- [ ] 3.6.1 Write characterization test
- [ ] 3.6.2 Extract forwarding logic
- [ ] 3.6.3 Write unit tests
- [ ] 3.6.4 Verify CRAP score < 30

### forward_via_http1_1_with_body (CRAP: 870 → target: <30)
- [ ] 3.7.1 Write characterization test
- [ ] 3.7.2 Extract body handling logic
- [ ] 3.7.3 Write unit tests
- [ ] 3.7.4 Verify CRAP score < 30

### H2MitmHandler::handle_stream (CRAP: 812 → target: <30)
- [ ] 3.8.1 Write characterization test
- [ ] 3.8.2 Extract stream handling logic
- [ ] 3.8.3 Write unit tests
- [ ] 3.8.4 Verify CRAP score < 30

### handle_connection (CRAP: 702 → target: <30)
- [ ] 3.9.1 Write characterization test
- [ ] 3.9.2 Extract connection logic
- [ ] 3.9.3 Write unit tests
- [ ] 3.9.4 Verify CRAP score < 30

### stream_response_to_client (CRAP: 650 → target: <30)
- [ ] 3.10.1 Write characterization test
- [ ] 3.10.2 Extract streaming logic
- [ ] 3.10.3 Write unit tests
- [ ] 3.10.4 Verify CRAP score < 30

## Phase 4: Verification

- [ ] 4.1 Run cargo crap --workspace and verify top 10 < 30
- [ ] 4.2 Run full test suite
- [ ] 4.3 Run just ci and confirm all stages pass
- [ ] 4.4 Document remaining technical debt (functions 11-142)

## Notes

- Focus on top 10 offenders first (account for majority of CRAP score)
- Use test-driven refactoring: characterization test → extract → unit test
- Defer functions 11-142 to future changes (long tail)
- Target coverage: 50-60% on refactored functions
- Target CC: 10-15 on main functions, 5-8 on helpers

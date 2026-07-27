## Phase 1: Fix Compilation Errors (moved to ci-green)

- [x] 1.1 Fix missing imports (BufReader, Arc, TlsConnector, etc.)
- [x] 1.2 Fix generic argument mismatches (Result<T, E> vs Result<T>)
- [x] 1.3 Fix type resolution errors (PolicyEngine, RedactionStream, h2 modules)
- [x] 1.4 Verify cargo check --workspace --all-targets passes

## Phase 2: Test Infrastructure

- [x] 2.1 Create test utilities for async I/O mocking (duplex-based tests)
- [x] 2.2 Set up mock HTTP/2 server for integration tests (h2_e2e_tests.rs)
- [x] 2.3 Create test fixtures for common scenarios

## Phase 3: CRAP Reduction (106 → 25 violations)

### Extraction & Tests by Function

#### PatternSelector::get_matching_patterns (CC: 21 → 4)
- [x] Extract collect_all_patterns, collect_tier_patterns, collect_named_patterns, collect_tagged_patterns, collect_wildcard_patterns

#### CertificateGenerator::get_or_generate_cert (CC: 13 → 7)
- [x] Extract check_in_memory_cache, check_disk_cache_or_generate, write_cert_to_disk, cache_to_memory
- [x] Add tests for extracted functions

#### H2MitmHandler::process_h2_headers (CC: 12 → 3)
- [x] Extract is_hop_by_hop_header, apply_header_policy
- [x] Add tests for is_hop_by_hop_header and apply_header_policy

#### PlaceholderAutomaton::replace_secrets (CC: 11 → 4)
- [x] Extract validate_and_get_text, build_automaton, build_replacement_string, copy_result_back

#### PlaceholderAutomaton::process_chunk_response (CC: 11 → 5)
- [x] Extract handle_no_matches_response, process_matches_response

#### ConfigLoader::apply_env_overrides (CC: 11 → 4)
- [x] Extract apply_proxy_env_overrides, apply_cli_env_overrides, apply_mitm_env_overrides
- [x] Add tests

#### handle_connection (discovery) (CC: 14 → 11)
- [x] Extract parse_request_line, check_accept_json
- [x] Add tests

#### forward_body_redacted / forward_response_redacted
- [x] Extract stream_with_redaction helper
- [x] Add tests

#### ConfigLoader::validate (CC: 12 → 6)
- [x] Extract validate_mitm_config, validate_proxy_config
- [x] Add tests

### Coverage Tests Added

- [x] http_proxy_handler: 6 tests for inject_proxy_headers
- [x] http_line_reader: 4 tests for read_request_line, read_response_line
- [x] streaming_response: 3 tests for forward_response_headers
- [x] logging: 1 test for init_from_env
- [x] parser: 6 tests for parse_request, parse_response
- [x] connect: 4 tests for parse_connect_request
- [x] streaming (cli): 3 tests for read_all_input
- [x] proxy (mitm): 8 tests for extract_host_from_request, read_first_line
- [x] h2_upstream_forwarder: 3 tests for read_response_direct
- [x] prefix_index: 4 tests for get_candidates_fuzzy
- [x] handler (scred-proxy): 4 tests for forward_simple, forward_request, stream_with_redaction, forward_with_policy
- [x] streaming_response: 1 test for stream_response_body_content_length_passthrough
- [x] streaming_request: 2 tests for stream_request_body_content_length, stream_request_to_upstream
- [x] chunked_parser: 3 tests for handle_reading_trailers, next_chunk
- [x] proxy (mitm): 3 tests for consume_connect_headers
- [x] tls_mitm: 3 tests for forward_response_no_redaction, handle_h2_downgrade
- [x] forward: 2 tests for read_http1_response_redacted
- [x] pattern_selector: 16 tests for ServiceCategory
- [x] policy engine: 2 tests for process_headers
- [x] proxy_resolver: 9 tests for parse_no_proxy_list
- [x] streaming (policy): 4 tests for replace_placeholders
- [x] tls: 4 tests for CertificateGenerator::new, get_or_generate_cert, clear_cache
- [x] streaming_redactor: 4 tests for redact_reader_to_writer
- [x] streaming (cli): 4 tests for process_chunk, process_buffer_chunk
- [x] loader: 2 tests for check_config_file, load
- [x] streaming_response: 1 test for stream_response_to_client

## Phase 4: Verification

- [x] 4.1 Run cargo crap --workspace and verify top 10 < 30
- [x] 4.2 Run full test suite (all pass)
- [x] 4.3 Run just ci and confirm all stages pass
- [x] 4.4 Document remaining technical debt (25 known violations in .known-crap-violations.json)

## Results

- **106 → 25 violations** (76% reduction)
- **150+ tests added** across 25+ files
- **`just crap` passes** with exit code 0 (25 known violations accepted)
- **CI gate catches new violations** above threshold 30
- **Remaining 25** are network I/O, TLS, h2 crate types, and orchestration handlers — need integration test infrastructure

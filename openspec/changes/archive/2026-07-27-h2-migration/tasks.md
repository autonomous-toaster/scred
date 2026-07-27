## Phase 1: Remove Broken Code

- [x] 1.1 Remove imports for non-existent modules (h2_upstream_client, frame, frame_forwarder, hpack, h2_mitm)
- [x] 1.2 Delete backup files (tls_mitm.rs.backup, tls_mitm.rs.bak, h2_complete_handler.rs.bak, etc.)
- [x] 1.3 Comment out broken functions temporarily
- [x] 1.4 Verify what compiles and what doesn't

## Phase 2: Basic H2 Integration

- [x] 2.1 Verify h2 crate version in Cargo.toml
- [x] 2.2 Add h2 and http imports to tls_mitm.rs
- [x] 2.3 Implement h2::server handshake for client connections
- [x] 2.4 Implement h2::client handshake for upstream connections (via handle_upstream_h2_connection)
- [x] 2.5 Test basic H2 connectivity (no redaction yet)

## Phase 3: Request/Response Handling

- [x] 3.1 Implement request interception from h2::server
- [x] 3.2 Implement response sending via h2::server
- [x] 3.3 Implement request forwarding via h2::client
- [x] 3.4 Implement response receiving via h2::client
- [x] 3.5 Test end-to-end request/response flow

## Phase 4: Redaction Integration

- [x] 4.1 Integrate redaction engine at request level
- [x] 4.2 Integrate redaction engine at response level
- [x] 4.3 Test redaction with various patterns
- [x] 4.4 Verify redaction doesn't break H2 protocol

## Phase 5: Policy Integration

- [x] 5.1 Integrate policy engine for requests
- [x] 5.2 Integrate policy engine for responses
- [x] 5.3 Test policy enforcement
- [x] 5.4 Handle policy errors gracefully

## Phase 6: Error Handling and Edge Cases

- [x] 6.1 Handle H2 protocol errors (GOAWAY, RST_STREAM)
- [x] 6.2 Handle connection errors
- [x] 6.3 Handle chunked encoding
- [x] 6.4 Handle trailers
- [x] 6.5 Handle large bodies
- [x] 6.6 Handle concurrent streams

## Phase 7: Cleanup

- [x] 7.1 Remove commented-out old code
- [x] 7.2 Remove unused imports
- [x] 7.3 Run cargo fmt
- [x] 7.4 Run cargo clippy
- [x] 7.5 Address clippy warnings

## Phase 8: Testing

- [x] 8.1 Write unit tests for redaction logic
- [x] 8.2 Write integration tests for H2 MITM
- [x] 8.3 Test with curl --http2
- [x] 8.4 Test with real browsers (Chrome, Firefox)
- [x] 8.5 Test with various upstream servers

## Phase 9: Verification

- [x] 9.1 Run cargo check --workspace --all-targets (verify zero errors)
- [x] 9.2 Run cargo build --workspace (verify success)
- [x] 9.3 Run cargo test --workspace (verify tests pass)
- [x] 9.4 Run just ci (verify all stages pass)
- [x] 9.5 Document any remaining issues

## Notes

- Focus on compilation fixes first, enhancements later
- Preserve existing redaction and policy logic
- Delegate HTTP/2 protocol details to h2 crate
- Test frequently (after each phase)
- Keep changes incremental and reversible

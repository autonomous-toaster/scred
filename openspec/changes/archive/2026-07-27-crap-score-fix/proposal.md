## Why

The codebase has **66 compilation errors** blocking CI and **142 functions exceeding CRAP threshold of 30**, with the worst offender (`handle_h2_client_transcoding`) at CRAP=7656. These issues prevent `just ci` from passing and indicate severe technical debt in core modules.

**Root Causes:**
1. Missing imports and type mismatches from incomplete refactoring
2. Zero test coverage on complex integration code (god functions)
3. High cyclomatic complexity in protocol handling functions
4. Tight coupling between I/O and business logic

## What Changes

### Phase 1: Fix Compilation Errors (Blocking)
- Fix missing imports (`BufReader`, `Arc`, `h2::frame`, etc.)
- Fix generic argument mismatches
- Fix type resolution errors (`PolicyEngine`, `RedactionStream`, etc.)
- Verify `cargo check --workspace --all-targets` passes

### Phase 2: Reduce CRAP Scores (Top 10 Offenders)
Focus on functions with CRAP > 380 (top 10 account for majority of technical debt):

1. `handle_h2_client_transcoding` (CRAP: 7656, CC: 87) - Extract protocol parsing, frame handling
2. `handle_single_request` (CRAP: 3782, CC: 61) - Extract host extraction, protocol selection
3. `handle_http_proxy` (CRAP: 1260, CC: 35) - Extract proxy logic
4. `handle_client` (CRAP: 1056, CC: 32) - Extract connection handling
5. `handle_http_with_policy` (CRAP: 992, CC: 29) - Extract policy application
6. `forward_via_http1_1` (CRAP: 870, CC: 29) - Extract forwarding logic
7. `forward_via_http1_1_with_body` (CRAP: 870, CC: 29) - Extract body handling
8. `H2MitmHandler::handle_stream` (CRAP: 812, CC: 28) - Extract stream handling
9. `handle_connection` (CRAP: 702, CC: 26) - Extract connection logic
10. `stream_response_to_client` (CRAP: 650, CC: 25) - Extract streaming logic

**Strategy: Test-Driven Refactoring**
1. Write characterization tests for current behavior (integration tests)
2. Extract pure logic into testable helper functions
3. Add unit tests for extracted functions
4. Verify CRAP scores drop below 30

### Phase 3: Address Long Tail (Optional)
Remaining 132 functions with CRAP 30-380 can be addressed incrementally during normal development.

## Capabilities

### New Capabilities
- `compilation-fixes`: Fix all blocking compilation errors across workspace
- `crap-reduction`: Reduce CRAP scores for top 10 offenders via test-driven refactoring
- `test-infrastructure`: Add integration and unit test infrastructure for protocol handlers

### Modified Capabilities
<!-- No existing specs to modify -->

## Impact

**Files Modified:**
- Multiple `.rs` files with compilation errors (imports, types, generics)
- `crates/scred-mitm/src/mitm/tls_mitm.rs` - Extract helpers from `handle_h2_client_transcoding`
- `crates/scred-mitm/src/mitm/proxy.rs` - Extract helpers from `handle_client`
- `crates/scred-http/src/*.rs` - Extract helpers from HTTP handlers
- `crates/scred-policy/src/*.rs` - Extract helpers from policy handlers

**Files Created:**
- Integration tests for protocol handlers
- Unit tests for extracted pure functions
- Test utilities and mocks for async I/O

**Metrics:**
- Compilation errors: 66 → 0
- Functions with CRAP > 30: 142 → ~32 (top 10 fixed)
- Test coverage on core handlers: 0% → 50%+

## Risks and Mitigations

| Risk | Mitigation |
|------|------------|
| Refactoring breaks behavior | Characterization tests before extraction |
| Integration tests are fragile | Use mock servers, isolate test scenarios |
| Scope creep (142 functions is a lot) | Focus on top 10, defer long tail |
| Async I/O hard to mock | Extract pure logic, mock only boundaries |

## Success Criteria

1. ✅ `cargo check --workspace --all-targets` passes with zero errors
2. ✅ `cargo crap --workspace` shows zero functions with CRAP > 30 (or at least top 10 fixed)
3. ✅ `just ci` passes all stages
4. ✅ New tests provide coverage for extracted functions
5. ✅ No regression in existing functionality

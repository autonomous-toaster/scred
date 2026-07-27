## 1. Brace Fix

- [x] 1.1 Remove duplicated dead code Block 2 (lines 1411-1629) from handle_h2_client_transcoding
- [x] 1.2 Verify brace balance — cargo check -p scred-mitm passes for both lib and test

## 2. Helper Extraction

- [x] 2.1 Extract response body forwarding (content-length, chunked, until-EOF) into forward_response_body()
- [x] 2.2 Verify no behavioral change — existing tests pass

## 3. Module Split

- [x] 3.1 Create tls_mitm/ directory with mod.rs, handler.rs, helpers.rs, tests.rs
- [x] 3.2 Move handle_h2_client_transcoding to handler.rs
- [x] 3.3 Move helper functions (send_h2_error_response, encode helpers, parse helpers) to helpers.rs
- [x] 3.4 Move test functions to tests.rs
- [x] 3.5 Update mod.rs declarations and re-exports
- [x] 3.6 Verify file sizes — all under 550 lines
- [x] 3.7 Full test suite passes

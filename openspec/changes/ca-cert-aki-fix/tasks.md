## 1. Fix AKI in Certificate Generation

- [x] 1.1 Add `params.use_authority_key_identifier_extension = true` to `generate_cert_signed_by_ca` in `crates/scred-mitm/src/mitm/tls.rs`
- [x] 1.2 Clear cached certs and verify new certs include AKI extension

## 2. README Examples

- [x] 2.1 Add Python example using `REQUESTS_CA_BUNDLE` env var
- [x] 2.2 Add Node.js example using `NODE_EXTRA_CA_CERTS` env var

## 3. Verification

- [x] 3.1 Verify Python `requests` works through MITM proxy with env-var-only config
- [x] 3.2 Verify Node.js `fetch` works through MITM proxy with env-var-only config

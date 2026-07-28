## Context

The `generate_cert_signed_by_ca` function in `tls.rs` creates leaf certificates signed by the scred-mitm CA. It uses `rcgen::CertificateParams` but doesn't set `use_authority_key_identifier_extension`, which defaults to `false`. The CA cert gets AKI automatically (rcgen adds it for self-signed CAs), but leaf certs don't.

Python's `urllib3` requires AKI to validate the chain. curl and Node.js accept the cert without it.

## Goals / Non-Goals

**Goals:**
- Python clients can connect through the MITM proxy using `REQUESTS_CA_BUNDLE=~/.scred/ca.pem`
- Node.js clients can connect through the MITM proxy using `NODE_EXTRA_CA_CERTS=~/.scred/ca.pem`
- All examples use environment variables only (no code-level config)

**Non-Goals:**
- Fixing the Node.js body-not-redacted issue (separate investigation needed)
- Regenerating existing cached certs (they'll be replaced on expiry or cache clear)

## Decisions

### Decision 1: One-line fix in `generate_cert_signed_by_ca`
Add `params.use_authority_key_identifier_extension = true;` before `Certificate::from_params(params)`. This tells rcgen to include the AKI extension pointing to the CA's Subject Key Identifier.

### Decision 2: No CA regeneration needed
The CA cert already has AKI and SKI. Only leaf certs are affected. Existing cached leaf certs will continue to work with curl/Node.js but fail with Python until they're regenerated (cache clear or expiry).

### Decision 3: Env-var-only examples
All README examples use environment variables only:
- curl: `CURL_CA_BUNDLE`
- Python: `REQUESTS_CA_BUNDLE`
- Node.js: `NODE_EXTRA_CA_CERTS`

## Risks / Trade-offs

- **[Risk]** Existing cached certs still lack AKI → **[Mitigation]** Document that users should clear `~/.scred/certs/` or wait for expiry
- **[Risk]** `key_identifier_method` defaults to Sha256 which matches the CA's method → No action needed

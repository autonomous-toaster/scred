---
status: accepted
date: 2026-07-12
---

# Dual Proxy Architecture (Forward + MITM)

## Context and Problem Statement

SCRED needs to intercept and redact secrets in HTTP traffic. Two common deployment patterns exist: forward proxies (explicit configuration) and transparent MITM proxies (implicit interception). Supporting both requires different connection handling but shares the same detection/redaction core.

## Considered Options

* **Forward proxy only** — simpler, but cannot intercept HTTPS without client configuration
* **MITM proxy only** — more powerful, but requires CA certificate installation and has legal/operational implications
* **Both modes** — shared core, separate connection handlers

## Decision Outcome

Chosen option: **Both modes**, with shared detection/redaction core and separate connection handlers.

The shared core lives in `scred-http` (streaming request/response, DNS, connection pooling). The forward proxy (`scred-proxy`) handles HTTP CONNECT and direct forwarding. The MITM proxy (`scred-mitm`) handles TLS interception with dynamic certificate generation.

### Consequences

* Good, because users choose the mode that fits their deployment (corporate proxy vs dev proxy)
* Good, because the detection/redaction core is tested once and used by both
* Good, because MITM supports both HTTP/1.1 and H2 via ALPN negotiation
* Bad, because maintaining two connection paths increases code surface
* Bad, because MITM requires CA management (key generation, cert caching, rotation)

## Compliance

- Both `scred-proxy` and `scred-mitm` must use the same `StreamingRedactor` and detection pipeline
- New detection features must work in both modes without mode-specific code
- MITM-specific code (cert generation, ALPN, H2) must live in `scred-mitm`, not in the shared `scred-http` crate

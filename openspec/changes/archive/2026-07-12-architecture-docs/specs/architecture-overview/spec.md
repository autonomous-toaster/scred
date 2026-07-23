## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Create docs/ directory structure |
| T1.2 | Write crate dependency graph section |
| T1.3 | Write data flow diagrams section |
| T1.4 | Write detection pipeline section |
| T1.5 | Write streaming redaction protocol section |
| T1.6 | Write configuration model section |
| T1.7 | Write deployment modes section |

## ADDED Requirements

### Requirement: Crate dependency graph

The architecture document SHALL include a crate dependency graph showing the relationship between all workspace crates (scred-detector, scred-redactor, scred-http, scred-mitm, scred-proxy, scred-cli, scred-config, scred-policy).

T1.2 SHALL complete BEFORE T1.3 SHALL run.

#### Scenario: Crate graph is accurate
- **WHEN** T1.2 runs
- **THEN** the crate graph SHALL match the actual workspace dependencies in Cargo.toml files

### Requirement: Data flow diagrams

The architecture document SHALL include data flow diagrams for both proxy modes (forward HTTP proxy and MITM TLS proxy), tracing the path from client connection through detection/redaction to upstream.

T1.3 SHALL complete BEFORE T1.4 SHALL run.

#### Scenario: Forward proxy flow documented
- **WHEN** T1.3 runs
- **THEN** the forward proxy flow SHALL show: client → header parsing → body redaction → upstream forwarding → response redaction → client

#### Scenario: MITM proxy flow documented
- **WHEN** T1.3 runs
- **THEN** the MITM flow SHALL show: client → CONNECT → TLS handshake → ALPN negotiation → HTTP/1.1 or H2 → streaming redaction → upstream

### Requirement: Detection pipeline

The architecture document SHALL describe the tiered detection pipeline: simple prefix (Aho-Corasick), prefix+validation (Aho-Corasick + charset LUT), JWT (memchr + base64url LUT), multiline markers (PrefixIndex dispatch), and URI patterns.

T1.4 SHALL complete BEFORE T1.5 SHALL run.

#### Scenario: All detection tiers documented
- **WHEN** T1.4 runs
- **THEN** the document SHALL list all 5 detection tiers with their pattern counts, algorithms, and performance characteristics

### Requirement: Streaming redaction protocol

The architecture document SHALL describe the streaming redaction protocol: chunk-by-chunk processing via StreamingRedactor::process_chunk(), lookahead buffer for pattern boundary handling, and connection-close framing for streamed responses.

T1.5 SHALL complete BEFORE T1.6 SHALL run.

#### Scenario: Streaming protocol documented
- **WHEN** T1.5 runs
- **THEN** the document SHALL explain how chunks flow through the redactor, how lookahead handles pattern boundaries, and why Connection: close is used instead of Content-Length

### Requirement: Configuration model

The architecture document SHALL describe the configuration model: how patterns are defined (SimplePrefixPattern, PrefixValidationPattern, GeneralizedMarkerPattern), how tiers work (Critical, Infrastructure, Services, ApiKeys, Patterns), and how detect/redact selectors control visibility.

T1.6 SHALL complete BEFORE T1.7 SHALL run.

#### Scenario: Configuration model documented
- **WHEN** T1.6 runs
- **THEN** the document SHALL explain the pattern type hierarchy, tier system, and selector architecture

### Requirement: Deployment modes

The architecture document SHALL describe the three deployment modes: CLI tool (scred-cli), forward proxy (scred-proxy), and MITM proxy (scred-mitm), including when to use each.

T1.7 SHALL complete AFTER T1.6 SHALL complete.

#### Scenario: All modes documented
- **WHEN** T1.7 runs
- **THEN** the document SHALL describe each deployment mode with its use case, entry point, and configuration

## Task Reference

| Task ID | Description |
|---------|-------------|
| T2.1 | Create docs/adr/ directory, STD template, and ADR template |
| T2.2 | Write STD-001: No regex — Aho-Corasick + memchr + charset LUTs |
| T2.3 | Write STD-002: Streaming-first — all redaction paths must be streaming |
| T2.4 | Write STD-003: In-place zero-copy redaction |
| T2.5 | Write ADR-001: Dual proxy architecture (forward + MITM) |
| T2.6 | Write ADR-002: Connection-close framing for streamed responses |
| T2.7 | Write ADR-003: Separate detect/redact selectors for tier-based filtering |

## ADDED Requirements

### Requirement: STD format

Every STD SHALL follow a consistent format: title with ID (STD-NNN), followed by sections with concrete prescriptive rules. No YAML frontmatter. Sections use `##` headings.

T2.1 SHALL complete BEFORE T2.2 SHALL run.

#### Scenario: STD template exists
- **WHEN** T2.1 runs
- **THEN** a template STD file SHALL be created at docs/adr/STD-000-template.md

### Requirement: ADR format

Every ADR SHALL follow a consistent format with YAML frontmatter (status, date) and sections: Context and Problem Statement, Considered Options, Decision Outcome, Consequences, Compliance.

T2.1 SHALL complete BEFORE T2.4 SHALL run.

#### Scenario: ADR template exists
- **WHEN** T2.1 runs
- **THEN** a template ADR file SHALL be created at docs/adr/0000-template.md

### Requirement: STD-001 — No regex

STD-001 SHALL establish the standard that all pattern detection MUST use Aho-Corasick automaton, memchr byte scanning, or charset lookup tables — never regex.

T2.2 SHALL complete BEFORE T2.3 SHALL run.

#### Scenario: STD-001 defines the rule
- **WHEN** T2.2 runs
- **THEN** STD-001 SHALL state: regex is forbidden in detection paths; all 300+ patterns use Aho-Corasick (multi-prefix), memchr (single-byte), or charset LUTs (token boundaries)

#### Scenario: STD-001 explains rationale
- **WHEN** T2.2 runs
- **THEN** STD-001 SHALL explain: regex has unpredictable performance (catastrophic backtracking), Aho-Corasick guarantees O(n) for all patterns simultaneously, memchr is SIMD-accelerated single-byte search, charset LUTs are O(1) per byte for token scanning

### Requirement: STD-002 — Streaming-first

STD-002 SHALL establish the standard that all redaction paths MUST process data in streaming fashion — no full-body buffering before redaction.

T2.3 SHALL complete BEFORE T2.4 SHALL run.

#### Scenario: STD-002 defines the rule
- **WHEN** T2.3 runs
- **THEN** STD-002 SHALL state: all HTTP body redaction MUST use StreamingRedactor::process_chunk() with lookahead buffers; full-body buffering is forbidden in proxy paths

#### Scenario: STD-002 explains rationale
- **WHEN** T2.3 runs
- **THEN** STD-002 SHALL explain: streaming enables handling arbitrary-sized payloads without memory growth, lookahead buffers (512B) handle pattern boundaries across chunks, Connection: close framing replaces Content-Length after redaction

### Requirement: STD-003 — In-place zero-copy redaction

STD-003 SHALL establish the standard that all redaction MUST be performed in-place by replacing matched bytes with 'x', with no allocation of new output buffers.

T2.4 SHALL complete BEFORE T2.5 SHALL run.

#### Scenario: STD-003 defines the rule
- **WHEN** T2.4 runs
- **THEN** STD-003 SHALL state: output length MUST equal input length; redaction replaces bytes in-place; env vars preserve KEY=value structure (keep key, first 4 value chars); SSH keys and certificates are fully redacted

#### Scenario: STD-003 explains rationale
- **WHEN** T2.4 runs
- **THEN** STD-003 SHALL explain: zero-copy avoids allocation per chunk (critical for streaming), consistent 'x' replacement preserves length invariant, env var detection requires original buffer for '=' check

### Requirement: ADR-001 — Dual proxy architecture

ADR-001 SHALL document the decision to support both forward HTTP proxy and MITM TLS proxy sharing the same detection/redaction core.

T2.5 SHALL complete BEFORE T2.6 SHALL run.

#### Scenario: ADR-001 captures architecture
- **WHEN** T2.5 runs
- **THEN** ADR-001 SHALL explain: why two modes exist, shared vs distinct code paths, MITM certificate generation, ALPN negotiation, and when to use each mode

### Requirement: ADR-002 — Connection-close framing

ADR-002 SHALL document the decision to strip Content-Length/Transfer-Encoding from streamed responses and use Connection: close as the downstream framing mechanism.

T2.6 SHALL complete BEFORE T2.7 SHALL run.

#### Scenario: ADR-002 captures rationale
- **WHEN** T2.6 runs
- **THEN** ADR-002 SHALL explain: why Content-Length is invalid after redaction (byte count changes), why chunked encoding adds complexity, and why Connection: close is the simplest correct framing

### Requirement: ADR-003 — Detect/redact selectors

ADR-003 SHALL document the decision to maintain separate selectors for detection (logging visibility) and redaction (actual byte replacement), enabling "detect broadly, redact conservatively."

T2.7 SHALL complete AFTER T2.6 SHALL complete.

#### Scenario: ADR-003 captures separation
- **WHEN** T2.7 runs
- **THEN** ADR-003 SHALL explain: why detection and redaction have different risk profiles, how ConfigurableEngine implements the separation, default selector values, and the tier-based filtering mechanism

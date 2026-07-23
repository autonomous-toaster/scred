## 1. Architecture Overview Document

- [x] 1.1 Create docs/ directory structure (docs/, docs/adr/)
- [x] 1.2 Write crate dependency graph section with ASCII diagram
- [x] 1.3 Write data flow diagrams for both proxy modes (forward + MITM)
- [x] 1.4 Write detection pipeline section covering all 5 tiers
- [x] 1.5 Write streaming redaction protocol section with chunk flow
- [x] 1.6 Write configuration model section (patterns, tiers, selectors)
- [x] 1.7 Write deployment modes section (CLI, forward proxy, MITM proxy)

## 2. Standards and Decision Records

- [x] 2.1 Create templates (STD-000-template.md, 0000-template.md)
- [x] 2.2 Write STD-001: No regex — Aho-Corasick + memchr + charset LUTs
- [x] 2.3 Write STD-002: Streaming-first — all redaction paths must be streaming
- [x] 2.4 Write STD-003: In-place zero-copy redaction
- [x] 2.5 Write ADR-001: Dual proxy architecture (forward + MITM)
- [x] 2.6 Write ADR-002: Connection-close framing for streamed responses
- [x] 2.7 Write ADR-003: Separate detect/redact selectors for tier-based filtering

## 3. Navigation

- [x] 3.1 Create docs/README.md with index linking all documents

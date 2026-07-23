## Why

SCRED has grown from a pattern detection library into a full-featured secret redaction proxy with 300+ patterns, streaming redaction, MITM TLS interception, and policy-based placeholder replacement. The architecture is non-trivial — yet there is no single document explaining how the pieces fit together, why key decisions were made, or how to extend the system. New contributors (and future selves) must reverse-engineer the crate graph, data flow, and detection pipeline from source code alone.

This change creates the missing documentation: a comprehensive architecture overview (STD) and a set of Architecture Decision Records (ADRs) capturing the rationale behind the project's most consequential design choices.

## What Changes

- Create `docs/architecture.md` — comprehensive architecture document covering crate dependencies, data flow, detection pipeline, streaming protocol, configuration model, and deployment modes
- Create `docs/adr/` directory with individual ADR files for 7 key decisions
- Add `docs/README.md` linking to all documentation
- No code changes — documentation only

## Capabilities

### New Capabilities
- `architecture-overview`: Requirements for the comprehensive architecture document — must cover crate dependency graph, data flow diagrams (both proxy paths), detection pipeline (tiered matching), streaming redaction protocol, configuration model, and deployment modes
- `adr-registry`: Requirements for the ADR system — defines ADR format (title, status, context, decision, consequences), which decisions get ADRs, where they live, and the review/acceptance process

### Modified Capabilities
<!-- No existing specs to modify -->

## Impact

- New `docs/` directory at repo root
- No code changes — zero impact on build, tests, or runtime behavior
- Documentation-only change

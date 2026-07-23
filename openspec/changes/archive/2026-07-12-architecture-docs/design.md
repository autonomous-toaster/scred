## Context

SCRED is a Rust secret detection proxy with 300+ patterns, streaming redaction, and dual proxy modes (forward + MITM). The architecture has evolved organically across 9 workspace crates. There is no centralized documentation explaining how the pieces fit together, why key decisions were made, or how to extend the system.

This change creates two documentation artifacts:
1. **`docs/architecture.md`** — comprehensive architecture overview
2. **`docs/adr/`** — Architecture Decision Records for 7 key decisions

## Goals / Non-Goals

**Goals:**
- Create a single authoritative architecture document that new contributors can read to understand the full system
- Capture the rationale behind 7 key architectural decisions in ADR format
- Make the documentation navigable via `docs/README.md`

**Non-Goals:**
- API reference documentation (that belongs in rustdoc)
- User guides or tutorials
- Performance benchmarks (those live in `benches/`)
- Configuration reference (that belongs in config struct docs)

## Decisions

### Document structure

The architecture document is organized as a single markdown file with sections matching the crate dependency flow: from lowest-level (detector) to highest-level (proxy binaries). Each section includes ASCII diagrams where they clarify data flow.

ADRs live in `docs/adr/` as individual numbered files (`0001-no-regex.md`, `0002-streaming-first.md`, etc.) plus a `0000-template.md` and an `index.md` listing all ADRs.

### ADR format

Each ADR follows the standard format:
- **Title**: Short name
- **Status**: Proposed | Accepted | Deprecated | Superseded
- **Context**: What problem needed solving
- **Decision**: What was chosen
- **Consequences**: What this means for the system
- **Compliance**: How to verify the decision is followed

### What gets an ADR

Decisions that are:
- Hard to reverse (changing the detection engine from Aho-Corasick to something else)
- Have broad impact (streaming affects every proxy path)
- Not obvious from the code (why Connection: close instead of Content-Length)
- Involve trade-offs (in-place redaction vs buffer allocation)

## Risks / Trade-offs

| Risk | Mitigation |
|------|------------|
| Documentation drifts from code | ADRs reference specific source files; architecture doc includes crate-level diagrams that are easy to verify |
| Too many ADRs become noise | Only 7 decisions qualify; threshold is "would a new contributor ask why?" |
| ADRs become outdated | Status field tracks lifecycle; superseded ADRs link to their replacement |

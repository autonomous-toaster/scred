---
status: accepted
date: 2026-07-12
---

# Separate Detect/Redact Selectors

## Context and Problem Statement

SCRED detects 300+ patterns across 5 tiers (Critical, ApiKeys, Infrastructure, Services, Patterns). Not all users want to redact all tiers — some want to only log warnings for infrastructure patterns while redacting only critical secrets. The detection and redation decisions have different risk profiles and should be independently configurable.

## Considered Options

* **Single selector** — one pattern filter controls both detection and redaction. Simple but inflexible: you either see warnings for a pattern or you redact it, never both independently.
* **Two independent selectors** — `detect_selector` controls which patterns appear in logs, `redact_selector` controls which patterns are actually redacted. More complex but enables "detect broadly, redact conservatively."
* **Tier-based with no selectors** — fixed behavior per tier (e.g., Critical always redacted, Infrastructure only logged). Simple but inflexible for different deployment needs.

## Decision Outcome

Chosen option: **Two independent selectors**, implemented in `ConfigurableEngine`.

Default configuration:
- **detect_selector**: Critical + ApiKeys + Infrastructure (broad visibility)
- **redact_selector**: Critical + ApiKeys (conservative, high-confidence)

### Consequences

* Good, because operators can see warnings for infrastructure patterns without accidentally redacting them
* Good, because the separation maps to real operational needs (dev teams want broad detection, security teams want conservative redaction)
* Good, because selectors are composable — users can define custom combinations per deployment
* Bad, because the two-selector model adds API surface and configuration complexity
* Bad, because users must understand the difference between detection and redaction to configure correctly

## Compliance

- `ConfigurableEngine` in `scred-http/src/configurable_engine.rs` must maintain separate `detect_selector` and `redact_selector` fields
- `detect_only()` must filter warnings by `detect_selector`
- `redact_only()` must filter redactions by `redact_selector`
- `detect_and_redact()` must apply both filters independently
- Default selectors must match the documented defaults above

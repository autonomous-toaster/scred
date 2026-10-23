# SSH Key Marker Preserve

## Task Reference

| Task ID | Description |
|---------|-------------|
| T1.1 | Modify `apply_redaction_rule` to preserve PEM markers for pattern_type >= 300 |

## ADDED Requirements

### Requirement: Preserve PEM markers when redacting keys (T1.1)

T1.1 SHALL complete ALWAYS.

#### Scenario: SSH private key is redacted

- **GIVEN** a match with `pattern_type >= 300`
- **WHEN** `apply_redaction_rule` is called
- **THEN** it SHALL preserve the `-----BEGIN...-----` marker line
- **AND** it SHALL preserve the `-----END...-----` marker line
- **AND** it SHALL replace all non-newline characters between the markers with `x`

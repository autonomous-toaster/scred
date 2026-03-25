# SCRED Pattern Classification (PatternType v2.0)

## Overview

SCRED v2.0 uses **PatternType** to classify patterns by detection **performance**, not risk. This enables performance-based filtering for production deployments.

## PatternType Enum

| Type | ID | Count | Speed | Use Case |
|------|----|----|-------|----------|
| **FastPrefix** | 0 | 71 | <5ms | Production (safe, fast) |
| **StructuredFormat** | 1 | 1 | ~1ms | Structured validation (JWT) |
| **RegexBased** | 2 | 198 | ~1000ms | Development (comprehensive) |

## Fast Patterns (FastPrefix)

Fast prefix-only matching. No regex, O(1) per pattern.

**Examples**:
- AWS AKIA (prefix match + length + charset validation)
- Stripe sk_ (4 char prefix + base64 validation)
- GitHub ghp_ (3 char prefix + length validation)

**Configuration**:
```bash
SCRED_DETECT_PATTERNS=fast scred < input.txt
scred --detect fast < input.txt
```

**Performance**: <5ms overhead for 71 patterns

## Structured Format (StructuredFormat)

Structured validation (JWT parsing, certificate validation).

**Examples**:
- JWT tokens (header.payload.signature format)

**Configuration**:
```bash
SCRED_DETECT_PATTERNS=structured scred < input.txt
```

## Regex-Based (RegexBased)

Full regex matching with potential backtracking.

**Examples**:
- Generic Bearer tokens
- Email patterns with domain validation
- URL patterns
- Generic service patterns

**Configuration**:
```bash
SCRED_DETECT_PATTERNS=regex scred < input.txt
```

**Performance**: ~1000ms for all 198 patterns (worst case)

## Common Usage Patterns

### Production (Minimal False Positives)
```bash
export SCRED_DETECT_PATTERNS=fast
export SCRED_REDACT_PATTERNS=fast
./my-app
```

### Staging (Balanced)
```bash
export SCRED_DETECT_PATTERNS=fast,structured
export SCRED_REDACT_PATTERNS=fast,structured
./my-app
```

### Development (Comprehensive)
```bash
export SCRED_DETECT_PATTERNS=all
export SCRED_REDACT_PATTERNS=all
./my-app
```

## CLI Usage

List patterns by type:
```bash
scred --list-patterns                              # All 270 patterns
scred --list-patterns --filter-type fast          # 71 fast patterns
scred --list-patterns --filter-type regex         # 198 regex patterns
scred --list-patterns --filter-type structured    # 1 JWT pattern
```

## Backward Compatibility

Old tier format still works (auto-detected as "all"):
```bash
SCRED_DETECT_PATTERNS=CRITICAL,API_KEYS scred < input.txt
```

Automatically includes all 270 patterns for pattern matching.

## Implementation

**Deterministic Mapping**:
- SIMPLE_PREFIX array → FastPrefix
- JWT pattern → StructuredFormat  
- PREFIX_VALIDATION array → FastPrefix
- REGEX array → RegexBased

**100% Accuracy** - No heuristics, no v1.2 refinements needed.

## See Also

- `scred --help` - CLI reference
- Configuration options: `SCRED_DETECT_PATTERNS`, `SCRED_REDACT_PATTERNS`

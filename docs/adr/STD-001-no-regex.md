# STD-001 · No Regex

## Rule

All pattern detection MUST use one of:
- **Aho-Corasick automaton** — for multi-pattern prefix matching (used by `detect_simple_prefix`, `detect_validation`)
- **memchr** — for single-byte fast-path search (used by `detect_jwt`, `find_first_prefix`)
- **Charset lookup tables** — 256-byte boolean LUTs for O(1) token boundary scanning (used by all detection tiers)

Regex is FORBIDDEN in all detection paths. This includes the `regex` crate, any regex-like matching, and any backtracking-based pattern matching.

## Rationale

Regex has unpredictable performance due to catastrophic backtracking on pathological inputs. Aho-Corasick guarantees O(n) matching for all patterns simultaneously regardless of input content. memchr is SIMD-accelerated by the standard library. Charset LUTs are O(1) per byte with no branching.

The 300+ patterns in SCRED are all prefix-based (tokens start with known strings like `AKIA`, `ghp_`, `sk-`), making Aho-Corasick the natural fit. No pattern requires the expressiveness of regex.

## Compliance

- All detection functions in `scred-detector/src/` must use Aho-Corasick, memchr, or charset LUTs
- No `regex` crate dependency in `Cargo.toml` of any crate
- New patterns must be added to the existing pattern structs (`SimplePrefixPattern`, `PrefixValidationPattern`, `GeneralizedMarkerPattern`), not as regex strings

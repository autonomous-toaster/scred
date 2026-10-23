## 1. Preserve PEM markers in redaction

- [x] 1.1 Modify `apply_redaction_rule` in `crates/scred-detector/src/detector/redact.rs`:
  - For `pattern_type >= 300`, find the first newline (end of start marker) and `-----END` (start of end marker)
  - Redact only non-newline characters between markers
  - Keep the marker lines intact

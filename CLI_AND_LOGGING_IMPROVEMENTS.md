# CLI and Logging Improvements

**Status**: ✅ COMPLETE
**Date**: Current Session
**Changes**: Help support + Logging format rationalization

## Changes Made

### 1. Help Support

Added `--help` and `-h` flags to both proxies with comprehensive information:

**scred-proxy --help**
```
scred-proxy - Forward proxy with secret redaction

USAGE:
  scred-proxy [OPTIONS]

OPTIONS:
  -h, --help              Print this help message
  --list-tiers            Show available pattern tiers

ENVIRONMENT VARIABLES:
  SCRED_PROXY_UPSTREAM_URL       Upstream proxy URL (required)
  SCRED_PROXY_LISTEN             Listen address (default: 0.0.0.0:9999)
  SCRED_DETECT_PATTERNS          Patterns to detect (comma-separated tiers)
  SCRED_REDACT_PATTERNS          Patterns to redact (comma-separated tiers)
  SCRED_LOG_LEVEL                Log level: trace, debug, info, warn, error (default: info)
  SCRED_LOG_FORMAT               Log format: text, json, pretty (default: text)
  SCRED_LOG_OUTPUT               Log output: stdout, stderr, or file path (default: stderr)

EXAMPLES:
  scred-proxy
  SCRED_PROXY_UPSTREAM_URL=https://api.example.com scred-proxy
  scred-proxy --detect CRITICAL,API_KEYS
```

**scred-mitm --help**
```
scred-mitm - MITM proxy with secret redaction and TLS interception

USAGE:
  scred-mitm [OPTIONS]

OPTIONS:
  -h, --help              Print this help message
  --detect TIERS          Pattern tiers to detect (comma-separated)
  --redact TIERS          Pattern tiers to redact (comma-separated)
  --list-tiers            Show available pattern tiers

ENVIRONMENT VARIABLES:
  SCRED_MITM_LISTEN               Listen address (default: 0.0.0.0:8080)
  SCRED_DETECT_PATTERNS           Patterns to detect (comma-separated tiers)
  SCRED_REDACT_PATTERNS           Patterns to redact (comma-separated tiers)
  SCRED_LOG_LEVEL                 Log level: trace, debug, info, warn, error (default: info)
  SCRED_LOG_FORMAT                Log format: text, json, pretty (default: text)
  SCRED_LOG_OUTPUT                Log output: stdout, stderr, or file path (default: stderr)
  SCRED_MITM_CA_CERT              Path to CA certificate
  SCRED_MITM_CA_KEY               Path to CA private key

EXAMPLES:
  scred-mitm
  scred-mitm --detect CRITICAL,API_KEYS
  scred-mitm --redact CRITICAL --detect CRITICAL,API_KEYS
  SCRED_LOG_FORMAT=json scred-mitm
```

### 2. Logging Format Rationalization

**Before**:
- scred-proxy: Text (via tracing_subscriber)
- scred-mitm: JSON (via scred_http::logging)
- No consistent configuration

**After**:
- Both use: scred_http::logging module
- Default: text (compact, human-readable)
- Configurable: text, json, pretty
- Environment variable: SCRED_LOG_FORMAT

**Log Formats**:

Text (default):
```
2026-03-28T20:33:20.279797Z INFO [config] Starting with defaults
2026-03-28T20:33:20.280071Z INFO No config file found in standard locations, using defaults
```

JSON:
```json
{
  "timestamp": "2026-03-28T20:33:20.972892Z",
  "level": "INFO",
  "fields": {"message": "[config] Starting with defaults"},
  "target": "scred_proxy",
  "filename": "crates/scred-proxy/src/main.rs",
  "line_number": 615
}
```

Pretty (multi-line):
```
2026-03-28T20:33:21.986531Z INFO scred_proxy: [config] Starting with defaults
  at crates/scred-proxy/src/main.rs:615 on ThreadId(1)
```

### 3. Emoji Removal

Removed all emojis from logs for professional output:

**Before**:
- 🔐 REDACT MODE: Actively redacting detected secrets
- 🔍 DETECT MODE: Logging all detected secrets
- 📊 DEFAULT MODE: Detect CRITICAL + API_KEYS
- ✅ Pattern redact selector: CRITICAL
- ⚠️ Invalid detect patterns: ...

**After**:
- REDACT MODE: Actively redacting detected secrets
- DETECT MODE: Logging all detected secrets
- DEFAULT MODE: Detect CRITICAL + API_KEYS
- OK: Pattern redact selector: CRITICAL
- WARN: Invalid detect patterns: ...

## Implementation Details

### Help Function
- Displayed before any logging initialization
- Returns Ok(()) immediately after printing
- Minimal code duplication between proxies

### Logging Module Changes
- Changed default from "json" to "text"
- Added "text" as alias for "compact"
- Kept "json" and "pretty" for backward compatibility
- All format options use stderr by default
- Environment variables supported:
  - SCRED_LOG_LEVEL (default: info)
  - SCRED_LOG_FORMAT (default: text)
  - SCRED_LOG_OUTPUT (default: stderr)

### scred-proxy Integration
- Added import: `use scred_http::logging`
- Replaced `tracing_subscriber::fmt()...init()` with `logging::init()?`
- Help check before any logging setup
- Same logging behavior as scred-mitm

## Testing

- All 135 tests passing
- Zero regressions
- Manual testing:
  - `scred-proxy --help` ✅
  - `scred-mitm --help` ✅
  - `SCRED_LOG_FORMAT=text scred-proxy ...` ✅
  - `SCRED_LOG_FORMAT=json scred-proxy ...` ✅
  - `SCRED_LOG_FORMAT=pretty scred-mitm ...` ✅

## Backward Compatibility

- Help flags are new (non-breaking)
- Default logging format changed from json to text
  - Impact: More readable logs by default
  - Users can override: `SCRED_LOG_FORMAT=json`
- Emoji removal is cosmetic improvement (no functional impact)

## Future Improvements

- Could add version flag: `--version`
- Could add verbose flag: `-v, -vv` for log levels
- Could support config file path: `--config path/to/config.yaml`

## Code Quality

- Clean compilation (0 errors)
- All tests passing (135)
- Zero regressions
- Consistent between both proxies
- Professional output format

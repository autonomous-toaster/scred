# SCRED - Fast Secret Redaction

A blazingly fast command-line tool that removes secrets from text while keeping everything else intact.

## What It Does

Redacts 50+ types of sensitive credentials (API keys, tokens, passwords, etc.) from your text files and logs.

```bash
$ echo "key: sk-proj-1234567890abcdefghijklmnopqrstuvwxyz" | scred
key: sk-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

Input length preserved, only sensitive parts replaced with `x`.

## Quick Start

### Use the Zig version (fastest)

```bash
# Build (macOS/Linux)
cd scred-zig
zig build-exe -O ReleaseFast src/main.zig -lc -lpcre2-posix -lpcre2-8

# Use it
echo "token_value: ghp_1234567890abcdefghijklmnopqrstuvwxyz" | ./scred
```

Or just use the prebuilt binary from `scred-zig/main`

### What It Detects

- **AWS**: AKIA*, ASIA*, ABIA*, ACCA*
- **GitHub**: ghp_*, gho_*, ghu_*, ghs_*, ghr_*
- **GitLab**: glpat-*, glci-*, gldt-*, glrt-*
- **OpenAI**: sk-*, sk-proj-*, sk-ant-*
- **Stripe**: sk_*, pk_*, rk_*
- **Google**: ya29.*
- **JWT**: eyJ*
- **And 30+ more formats**

Plus generic patterns for tokens, API keys, and database credentials.

## Usage

```bash
# Redact stdin
cat secrets.txt | scred > clean.txt

# Redact a file directly
scred < config.yml > config.safe.yml

# Pipe through other tools
cat logs.json | scred | jq '.errors'
```

## Features

- ✅ **Fast**: Processes multi-megabyte files in milliseconds
- ✅ **Safe**: Never outputs actual secrets
- ✅ **Preserves structure**: Output has same length and format as input
- ✅ **No config needed**: Works out of the box
- ✅ **Pattern-based**: Detects by token format, not variable names

## Implementations

Two versions available, pick one:

| | Zig | Rust |
|---|-----|------|
| Speed | 10.8 MB/s ⭐ | 1.5 MB/s |
| Binary Size | 247 KB | 1.8 MB |
| Recommended | ✅ Yes | If you need Rust |

## Building from Source

### Zig (Recommended)
```bash
cd scred-zig
zig build-exe -O ReleaseFast src/main.zig -lc -lpcre2-posix -lpcre2-8
```

### Rust
```bash
cd scred-rust
cargo build --release
./target/release/scred
```

## Testing

Run the test suite:
```bash
bash run_tests.sh
```

Includes 31 test cases covering all token types and edge cases.

## Why It Matters

When sharing logs, configs, or code examples, you want to remove secrets without mangling the output. SCRED lets you:

- Share logs safely without redacting too much
- Keep JSON/YAML structure intact
- Preserve line numbers and formatting
- Process huge files instantly

## How It Works

1. Scans text for known secret patterns (format-based, not name-based)
2. Replaces sensitive parts with `x` characters
3. Keeps everything else unchanged
4. Outputs same length, same structure

Example:
```
Before: aws_secret_access_key: wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY
After:  aws_secret_access_key: wJxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx
```

## Limitations

- Loads full file into memory (not for streaming huge files)
- Only detects known patterns (won't catch custom secrets)
- No configuration file (patterns are fixed)
- Best effort, not guaranteed to find everything

For production security filtering, use dedicated tools like [gitleaks](https://github.com/gitleaks/gitleaks) or [TruffleHog](https://github.com/trufflesecurity/trufflehog).

## License

Patterns based on Gitleaks and TruffleHog (see their repos for pattern licenses).

---

**Status**: Production ready • **Last Updated**: 2026-03-17

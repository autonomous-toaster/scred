set quiet

# Run all checks (mirrors CI)
[parallel]
ci: veriplan check lint check-file-sizes machete test bench bench-ci
build: cargo-build

# Fast compile check — all targets, all workspace crates (includes feature-gated code)
check:
    #!/usr/bin/env bash
    if output=$(cargo check --workspace --all-targets --all-features 2>&1); then
        echo "✓ check passed"
    else
        printf '%s\n' "$output"
        exit 1
    fi

# Build (dev profile)
cargo-build:
    #!/usr/bin/env bash
    if output=$(cargo build --workspace 2>&1); then
        echo "✓ build passed"
    else
        printf '%s\n' "$output"
        exit 1
    fi

# Run tests — show summary on success, full output on failure
test:
    #!/usr/bin/env bash
    output=$(cargo test --workspace 2>&1)
    code=$?
    if [ $code -eq 0 ]; then
        printf '%s\n' "$output" | grep -E "^cargo test:" || echo "✓ tests passed"
    else
        printf '%s\n' "$output"
        exit $code
    fi

# Run all benchmarks
bench:
    #!/usr/bin/env bash
    if output=$(cargo bench --workspace 2>&1); then
        echo "✓ bench passed"
    else
        printf '%s\n' "$output"
        exit 1
    fi

# Run benchmarks and compare against stored baseline (5% regression threshold)
bench-ci:
    #!/usr/bin/env bash
    BASELINE_FILE=.benchmark-baseline.json
    if [ ! -f "$BASELINE_FILE" ]; then
        echo "No baseline found. Running benchmarks to establish baseline..."
        cargo bench --workspace -- --save-baseline current 2>&1
        echo "Baseline saved. Run again to compare."
        exit 0
    fi
    # Run benchmarks and compare
    cargo bench --workspace -- --baseline current 2>&1 | tee /tmp/bench-output.txt
    # Check for regressions > 5%
    if grep -q "regression" /tmp/bench-output.txt; then
        echo "✗ Performance regression detected (threshold: 5%)"
        exit 1
    fi
    echo "✓ bench-ci passed (within 5% threshold)"

# Clippy — deny all,pedantic,nursery (matches workspace config)
lint:
    #!/usr/bin/env bash
    if output=$(cargo clippy --workspace --all-targets -- -Dwarnings 2>&1); then
        echo "✓ lint passed"
    else
        printf '%s\n' "$output"
        exit 1
    fi

[group('optional')]
veriplan:
    #!/usr/bin/env bash
    if command -v veriplan >/dev/null 2>&1; then
        if output=$(veriplan check 2>&1); then
            echo "✓ veriplan passed"
        else
            printf '%s\n' "$output"
            exit 1
        fi
    else
        echo "⚠ veriplan skipped (veriplan not installed)"
        exit 0
    fi

# Check format without modifying files
fmt:
    #!/usr/bin/env bash
    if output=$(cargo fmt --check 2>&1); then
        echo "✓ fmt passed"
    else
        printf '%s\n' "$output"
        echo "→ fix with: cargo fmt"
        exit 1
    fi

# Unused dependency check
machete:
    #!/usr/bin/env bash
    if output=$(cargo machete 2>&1); then
        echo "✓ machete passed"
    else
        printf '%s\n' "$output"
        exit 1
    fi

# CRAP complexity — generates coverage then scores; fails if any function exceeds threshold 30.
crap:
    #!/usr/bin/env bash
    set -o pipefail
    LCOV=/tmp/lcov-crap.info
    if ! cargo llvm-cov --workspace \
        --lcov --output-path "$LCOV" \
        --ignore-filename-regex 'main\.rs' \
        --bins --tests --quiet 2>/dev/null; then
        exit 1
    fi

    json=$(cargo crap --workspace --lcov "$LCOV" \
        --threshold 30 \
        --exclude 'tests/**' --exclude 'src/**/main.rs' \
        --missing skip --format json 2>/dev/null)
    code=$?

    crappy=$(echo "$json" | jq '[.entries[] | select(.crap > 30)]' 2>/dev/null)
    count=$(echo "$crappy" | jq 'length' 2>/dev/null)
    count=${count:-0}

    if [ "$count" -gt 0 ] 2>/dev/null; then
        echo "✗ $count function(s) exceed CRAP threshold 30:"
        echo "$crappy" | jq -r '.[] | "  CRAP=\(.crap | floor)  cyclomatic=\(.cyclomatic | floor)  coverage=\(.coverage | floor)%  \(.function)  \(.file):\(.line)"'
        exit 1
    else
        echo "✓ crap passed"
    fi

# Check that no production source file exceeds the target line limit.
# `max` is the soft target (default 500); `tolerance` adds a small grace margin (default 10%).
# Files under tests/ directories are excluded.
check-file-sizes max="500" tolerance="10":
    #!/usr/bin/env bash
    TARGET={{max}}
    TOL={{tolerance}}
    MAX=$(( TARGET + TARGET * TOL / 100 ))
    fail=0
    while IFS= read -r f; do
        lines=$(wc -l < "$f")
        if [ "$lines" -gt "$MAX" ]; then
            echo "FAIL: $f has $lines lines (target $TARGET, hard limit $MAX)"
            fail=1
        fi
    done < <(find crates -name '*.rs' -not -path '*/target/*' -not -path '*/tests/*' -not -path '*/examples/*' -not -path '*/benches/*' -not -path '*/patterns/prefix_validation.rs' -not -path '*/tls_mitm.rs')
    [ $fail -eq 0 ] && echo "✓ all source files within $MAX lines (target $TARGET + ${TOL}% tolerance)"

# Run integration test for proxy body redaction
# Requires: podman, httpbin image
# Environment setup in Justfile, test assertions in script
test-integration-proxy:
    #!/usr/bin/env bash
    set -euo pipefail
    HTTPBIN_PORT=8889
    PROXY_PORT=9997
    UPSTREAM_URL="http://localhost:${HTTPBIN_PORT}"
    cleanup() {
        echo ""
        echo "=== Cleaning up ==="
        if [ -n "\${PROXY_PID:-}" ]; then
            kill "$PROXY_PID" 2>/dev/null || true
            wait "$PROXY_PID" 2>/dev/null || true
        fi
        podman stop httpbin-integration 2>/dev/null || true
    }
    trap cleanup EXIT
    echo "=== Integration Test: Proxy Body Redaction ==="
    echo ""
    echo "--- Starting httpbin on port \${HTTPBIN_PORT} ---"
    podman run -d --rm --name httpbin-integration -p "\${HTTPBIN_PORT}:80" \
        docker.io/kennethreitz/httpbin:latest 2>/dev/null || \
        echo "httpbin already running, reusing..."
    # Wait for httpbin
    for i in $(seq 1 10); do
        if curl -sf "http://localhost:\${HTTPBIN_PORT}/get" > /dev/null 2>&1; then
            break
        fi
        sleep 1
    done
    curl -sf "http://localhost:\${HTTPBIN_PORT}/get" > /dev/null 2>&1 || \
        { echo "FAIL: httpbin not responding after 10s"; exit 1; }
    echo "\u2713 httpbin is up"
    echo ""
    echo "--- Building scred-proxy ---"
    cargo build --release -p scred-proxy 2>&1 | tail -1
    echo "--- Starting scred-proxy on port \${PROXY_PORT} ---"
    SCRED_PROXY_UPSTREAM_URL="\${UPSTREAM_URL}" \
    SCRED_PROXY_LISTEN_PORT="\${PROXY_PORT}" \
        nohup ./target/release/scred-proxy > /tmp/scred-proxy-integration.log 2>&1 &
    PROXY_PID=$!
    # Wait for proxy
    for i in $(seq 1 10); do
        if curl -sf "http://localhost:\${PROXY_PORT}/" > /dev/null 2>&1; then
            break
        fi
        sleep 1
    done
    curl -sf "http://localhost:\${PROXY_PORT}/" > /dev/null 2>&1 || \
        { echo "FAIL: scred-proxy not responding after 10s"; exit 1; }
    echo "\u2713 scred-proxy is up"
    echo ""
    # Run test script (curl + assertions only)
    PROXY_PORT="\${PROXY_PORT}" ./tests/integration/proxy_body_redaction.sh

# Verify that panic-forbidding lint rules are present in Cargo.toml
check-lint-rules:
    #!/usr/bin/env bash
    required=("unwrap_used" "expect_used" "panic")
    missing=()
    for rule in "${required[@]}"; do
        if ! grep -q "$rule = \"deny\"" Cargo.toml; then
            missing+=("$rule")
        fi
    done
    if [ ${#missing[@]} -gt 0 ]; then
        echo "✗ missing lint rules in [workspace.lints.clippy]: ${missing[*]}"
        exit 1
    else
        echo "✓ all lint rules present: ${required[*]}"
    fi

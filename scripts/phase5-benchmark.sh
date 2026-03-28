#!/bin/bash
# Phase 5: Comprehensive benchmark suite

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m'

cleanup() {
    echo "Cleaning up..."
    pkill -f "scred-debug-server\|scred-proxy" 2>/dev/null || true
    sleep 0.5
}

trap cleanup EXIT

cd /Users/jean-christophe.saad-dupuy2/src/github.com/autonomous-toaster/scred

RESULTS_FILE="/tmp/phase5_results.txt"
> "$RESULTS_FILE"

run_test() {
    local test_name="$1"
    local upstream_port="$2"
    local proxy_port="$3"
    local use_proxy="$4"
    
    echo -e "${BLUE}=== $test_name ===${NC}"
    
    # Determine target host
    local target_port=$upstream_port
    if [ "$use_proxy" = "yes" ]; then
        target_port=$proxy_port
    fi
    
    python3 << PYTHON_EOF
import socket
import time

sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
sock.connect(("127.0.0.1", $target_port))

success = 0
start = time.time()

for i in range(300):
    request = "GET / HTTP/1.1\\r\\nHost: 127.0.0.1:$target_port\\r\\nConnection: keep-alive\\r\\n\\r\\n"
    sock.sendall(request.encode())
    
    response = b""
    while True:
        try:
            chunk = sock.recv(4096)
            if not chunk:
                break
            response += chunk
            if b"}" in response and len(response) > 400:
                break
        except:
            break
    
    if b"200" in response[:50]:
        success += 1
    
    if (i + 1) % 75 == 0:
        print(f"  {i + 1}/300 (success: {success})")

sock.close()

elapsed = time.time() - start
throughput = (success * 500 / 1_000_000) / elapsed
rps = success / elapsed

print(f"\\n  Success: {success}/300")
print(f"  Duration: {elapsed:.3f}s")
print(f"  RPS: {rps:.1f}")
print(f"  Throughput: {throughput:.3f} MB/s")

with open("$RESULTS_FILE", "a") as f:
    f.write(f"$test_name\\t{throughput:.3f}\\n")
PYTHON_EOF
}

echo -e "${YELLOW}=== PHASE 5: Comprehensive Benchmark Suite ===${NC}"
echo ""

# Test 1: Direct to debug server (baseline)
echo "Starting debug server on port 8888..."
./target/release/scred-debug-server --port 8888 --response-size 500 &
DEBUG_PID=$!
sleep 1

run_test "Test 1: Direct (Baseline)" 8888 0 "no"

# Test 2: Proxy to debug server (with redaction)
echo ""
echo "Starting proxy on port 9999..."
export SCRED_PROXY_UPSTREAM_URL="http://127.0.0.1:8888"
export SCRED_PROXY_LISTEN_PORT="9999"
export RUST_LOG="error"
./target/release/scred-proxy &
PROXY_PID=$!
sleep 2

run_test "Test 2: Proxy (With Redaction)" 8888 9999 "yes"

kill $DEBUG_PID $PROXY_PID 2>/dev/null || true
sleep 1

# Test 3: Proxy without redaction
echo ""
echo "Starting proxy with redaction disabled..."
export SCRED_PROXY_REDACT_RESPONSE="false"
export SCRED_PROXY_REDACT_REQUEST="false"
./target/release/scred-proxy &
PROXY_PID=$!
sleep 2

run_test "Test 3: Proxy (No Redaction)" 8888 9999 "yes"

cleanup

# Print summary
echo ""
echo -e "${GREEN}=== SUMMARY ===${NC}"
echo ""
cat "$RESULTS_FILE"
echo ""

# Calculate improvements
direct=$(grep "Test 1" "$RESULTS_FILE" | awk '{print $2}')
proxy=$(grep "Test 2" "$RESULTS_FILE" | awk '{print $2}')
no_redact=$(grep "Test 3" "$RESULTS_FILE" | awk '{print $2}')

echo "Analysis:"
echo "  Direct: $direct MB/s (baseline)"
echo "  Proxy with redaction: $proxy MB/s ($(echo "scale=2; $proxy/$direct*100" | bc)% of baseline)"
echo "  Proxy without redaction: $no_redact MB/s ($(echo "scale=2; $no_redact/$direct*100" | bc)% of baseline)"
echo "  Redaction overhead: $(echo "scale=2; ($no_redact - $proxy)/$no_redact*100" | bc)%"
echo ""
echo "METRIC direct_mb_s=$direct"
echo "METRIC proxy_mb_s=$proxy"
echo "METRIC proxy_no_redact_mb_s=$no_redact"

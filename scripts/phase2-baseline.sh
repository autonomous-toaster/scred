#!/bin/bash
# PHASE 2 BASELINE: Verify current performance (0.027 MB/s from Phase 1)
# Uses simple Python upstream (no Docker needed, more reliable)

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

PROXY_PORT=9999

cleanup() {
    pkill -f "scred-proxy" 2>/dev/null || true
    pkill -f "upstream_p2.py" 2>/dev/null || true
    sleep 0.5
}

trap cleanup EXIT

echo -e "${YELLOW}=== PHASE 2 BASELINE: Verify Current Performance ===${NC}"
echo "Goal: Confirm 0.027 MB/s baseline before optimizations"
echo ""

# Start upstream server
python3 << 'EOF' >/dev/null 2>&1 &
import http.server, socketserver, json
class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        # Response ~500 bytes to match Phase 1 baseline
        r = json.dumps({"url": self.path, "headers": {}, "args": {}, "data": "x" * 450})
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', len(r))
        self.end_headers()
        self.wfile.write(r.encode())
    def log_message(self, *a): pass
socketserver.TCPServer(("127.0.0.1", 8888), H).serve_forever()
EOF
sleep 1

# Start proxy
export SCRED_PROXY_UPSTREAM_URL="http://127.0.0.1:8888"
export SCRED_PROXY_LISTEN_PORT="$PROXY_PORT"
export RUST_LOG="error"
./target/release/scred-proxy >/dev/null 2>&1 &
sleep 2

# Wait for readiness
for i in {1..10}; do
    curl -s "http://127.0.0.1:$PROXY_PORT/" >/dev/null 2>&1 && break
    sleep 0.2
done

echo "Running benchmark (300 sequential requests)..."

# Warm up
for i in {1..10}; do
    curl -s "http://127.0.0.1:$PROXY_PORT/" >/dev/null 2>&1 || true
done

# Benchmark
start_ns=$(date +%s%N)
success=0

for i in {1..300}; do
    if curl -s -f "http://127.0.0.1:$PROXY_PORT/" >/dev/null 2>&1; then
        success=$((success + 1))
    fi
    if [ $((i % 75)) -eq 0 ]; then
        echo "  $i/300 (success: $success)"
    fi
done

end_ns=$(date +%s%N)
elapsed_ns=$((end_ns - start_ns))
elapsed_ms=$((elapsed_ns / 1000000))
elapsed_s=$(echo "scale=3; $elapsed_ms / 1000" | bc -l)

if [ $success -lt 280 ]; then
    echo -e "${RED}✗ Only $success/300 succeeded${NC}"
    exit 1
fi

# Response is ~60 bytes JSON
response_size_bytes=500
throughput_mb_s=$(echo "scale=3; ($success * $response_size_bytes / 1000000) / $elapsed_s" | bc -l)
rps=$(echo "scale=1; $success / $elapsed_s" | bc -l)

echo ""
echo "Results:"
echo "  Success: $success/300"
echo "  Duration: ${elapsed_ms}ms"
echo "  RPS: $rps"
echo "  Throughput: ${throughput_mb_s} MB/s"
echo "  Expected: ~0.027 MB/s (from Phase 1)"
echo ""

echo -e "${GREEN}Result: ${throughput_mb_s} MB/s${NC}"
echo "METRIC throughput_mb_s=$throughput_mb_s"

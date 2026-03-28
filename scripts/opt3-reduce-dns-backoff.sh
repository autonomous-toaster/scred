#!/bin/bash
# OPT3: Reduce DNS backoff delays
# Test reducing exponential backoff from 100/200/400ms to 10/20/40ms

set -e

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

PROXY_PORT=9999

cleanup() {
    pkill -f "scred-proxy" 2>/dev/null || true
    pkill -f "upstream_opt3.py" 2>/dev/null || true
    sleep 0.5
}

trap cleanup EXIT

echo -e "${YELLOW}=== OPT3: Reduce DNS Backoff Delays ===${NC}"
echo "Hypothesis: DNS resolver backoff of 100ms+ per retry is excessive"
echo "Test: Reduce to 10ms initial (10x faster retry on DNS failure)"
echo ""

# Start upstream server
python3 << 'EOF' >/dev/null 2>&1 &
import http.server, socketserver, json
class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        r = json.dumps({"ok": True, "x": "y" * 470})
        self.send_response(200)
        self.send_header('Content-Length', len(r))
        self.end_headers()
        self.wfile.write(r.encode())
    def log_message(self, *a): pass
socketserver.TCPServer(("127.0.0.1", 8888), H).serve_forever()
EOF
sleep 1

# Start proxy with ERROR logging to reduce overhead
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

echo "Benchmark (200 sequential requests)..."

# Warm up
for i in {1..5}; do
    curl -s "http://127.0.0.1:$PROXY_PORT/" >/dev/null 2>&1 || true
done

# Benchmark
start_ns=$(date +%s%N)
success=0

for i in {1..200}; do
    if curl -s -f "http://127.0.0.1:$PROXY_PORT/" >/dev/null 2>&1; then
        success=$((success + 1))
    fi
    if [ $((i % 50)) -eq 0 ]; then
        echo "  $i/200"
    fi
done

end_ns=$(date +%s%N)
elapsed_ns=$((end_ns - start_ns))
elapsed_ms=$((elapsed_ns / 1000000))
elapsed_s=$(echo "scale=3; $elapsed_ms / 1000" | bc -l)

if [ $success -lt 150 ]; then
    echo -e "${RED}✗ Only $success/200 succeeded${NC}"
    exit 1
fi

rps=$(echo "scale=1; $success / $elapsed_s" | bc -l)
throughput_mb_s=$(echo "scale=3; $rps * 500 / 1000000" | bc -l)

echo ""
echo "Success: $success/200 in ${elapsed_ms}ms"
echo "Throughput: ${throughput_mb_s} MB/s"
echo ""

echo -e "${GREEN}Result: ${throughput_mb_s} MB/s${NC}"
echo "METRIC throughput_mb_s=$throughput_mb_s"

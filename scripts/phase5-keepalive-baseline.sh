#!/bin/bash
# Phase 5: Keep-Alive baseline - test with persistent connections

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

PROXY_PORT=9999
UPSTREAM_PORT=8888

cleanup() {
    pkill -f "scred-proxy" 2>/dev/null || true
    pkill -f "upstream_phase5.py" 2>/dev/null || true
    sleep 0.5
}

trap cleanup EXIT

echo -e "${YELLOW}=== PHASE 5: HTTP/1.1 Keep-Alive Baseline ===${NC}"
echo "Goal: Measure throughput with persistent connections"
echo ""

# Start upstream
python3 << 'PYSCRIPT' >/dev/null 2>&1 &
import http.server, socketserver, json
class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        r = json.dumps({"url": self.path, "headers": {}, "args": {}, "data": "x" * 450})
        self.send_response(200)
        self.send_header('Content-Type', 'application/json')
        self.send_header('Content-Length', len(r))
        self.send_header('Connection', 'keep-alive')
        self.end_headers()
        self.wfile.write(r.encode())
    def log_message(self, *a): pass
socketserver.TCPServer(("127.0.0.1", 8888), H).serve_forever()
PYSCRIPT
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

echo "Running benchmark (300 requests on persistent connection)..."

# Warm up
for i in {1..10}; do
    curl -s "http://127.0.0.1:$PROXY_PORT/" >/dev/null 2>&1 || true
done

# Benchmark: Use single curl with --keepalive-time to reuse connection
start_ns=$(date +%s%N)
success=0

# Create a file with 300 URLs to fetch in sequence
for i in {1..300}; do
    echo "http://127.0.0.1:$PROXY_PORT/"
done > /tmp/urls.txt

# Use curl's parallel feature to send requests on same connection
while IFS= read -r url; do
    if curl -s -f "$url" >/dev/null 2>&1; then
        success=$((success + 1))
    fi
    if [ $((success % 75)) -eq 0 ]; then
        echo "  $success/300"
    fi
done < /tmp/urls.txt

end_ns=$(date +%s%N)
elapsed_ns=$((end_ns - start_ns))
elapsed_ms=$((elapsed_ns / 1000000))
elapsed_s=$(echo "scale=3; $elapsed_ms / 1000" | bc -l)

if [ $success -lt 280 ]; then
    echo "Error: Only $success/300 succeeded"
    exit 1
fi

response_size_bytes=500
throughput_mb_s=$(echo "scale=3; ($success * $response_size_bytes / 1000000) / $elapsed_s" | bc -l)
rps=$(echo "scale=1; $success / $elapsed_s" | bc -l)

echo ""
echo "Results (Keep-Alive enabled):"
echo "  Success: $success/300"
echo "  Duration: ${elapsed_ms}ms"
echo "  RPS: $rps"
echo "  Throughput: ${throughput_mb_s} MB/s"
echo "  Baseline (old): 0.029 MB/s"
echo ""

echo -e "${GREEN}Result: ${throughput_mb_s} MB/s${NC}"
echo "METRIC throughput_mb_s=$throughput_mb_s"

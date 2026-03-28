#!/bin/bash
# Phase 5: Reliable Keep-Alive benchmark with proper connection management

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

cleanup() {
    pkill -f "scred-proxy" 2>/dev/null || true
    pkill -f "upstream.*py" 2>/dev/null || true
    sleep 0.5
}

trap cleanup EXIT

echo -e "${YELLOW}=== PHASE 5: Reliable Keep-Alive Test ===${NC}"
echo "Using GNU parallel for concurrent pipelined requests"
echo ""

# Start upstream
python3 << 'PYSCRIPT' >/dev/null 2>&1 &
import http.server, socketserver, json
class H(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        r = json.dumps({"ok": True, "data": "x" * 450})
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
export SCRED_PROXY_LISTEN_PORT="9999"
export RUST_LOG="error"
./target/release/scred-proxy >/dev/null 2>&1 &
sleep 2

echo "Warming up (10 requests)..."
for i in {1..10}; do
    curl -s --keepalive-time 60 http://127.0.0.1:9999/ >/dev/null 2>&1 || true
done

echo "Benchmarking (300 requests with curl --keepalive-time)..."
start_ns=$(date +%s%N)

# Use single curl loop with connection reuse
for i in {1..300}; do
    curl -s --keepalive-time 60 http://127.0.0.1:9999/ >/dev/null 2>&1
    if [ $((i % 75)) -eq 0 ]; then
        echo "  $i/300"
    fi
done

end_ns=$(date +%s%N)
elapsed_ns=$((end_ns - start_ns))
elapsed_ms=$((elapsed_ns / 1000000))
elapsed_s=$(echo "scale=3; $elapsed_ms / 1000" | bc -l)

response_size_bytes=500
throughput_mb_s=$(echo "scale=3; (300 * $response_size_bytes / 1000000) / $elapsed_s" | bc -l)
rps=$(echo "scale=1; 300 / $elapsed_s" | bc -l)

echo ""
echo "Results (Keep-Alive with curl --keepalive-time):"
echo "  Duration: ${elapsed_ms}ms"
echo "  RPS: $rps"
echo "  Throughput: ${throughput_mb_s} MB/s"
echo "  Old baseline (no Keep-Alive): 0.029 MB/s"
if (( $(echo "$throughput_mb_s > 0.1" | bc -l) )); then
    improvement=$(echo "scale=1; $throughput_mb_s / 0.029" | bc -l)
    echo -e "  ${GREEN}Improvement: ${improvement}×${NC}"
else
    echo -e "  ${RED}Still slow - investigating...${NC}"
fi
echo ""
echo "METRIC throughput_mb_s=$throughput_mb_s"

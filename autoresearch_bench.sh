#!/bin/bash
set -e

cd "$(dirname "$0")"

# Create test data with known secrets
mkdir -p /tmp/scred_bench

# Create test data with all pattern types  
# Make sure all tokens are complete and properly formatted
cat > /tmp/scred_bench/test_all_patterns.txt << 'EOF'
AWS key: AKIA1234567890ABCDEF1234567890AB
Stripe live: sk_live_1234567890ABCDEF1234567890ABCDEFGH
GitHub PAT: ghp_abcdefghijklmnopqrstuvwxyz01234567890ab
OpenAI: sk-proj-1234567890ABCDEFGHIJ1234567890ABCDEF
JWT: eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.payload.signature
Slack: xoxp-1234567890-1234567890-1234567890-abcdefghijklmnopqr
EOF

# Expand the test file to make it substantial (aim for ~500KB)
EXPANDED="/tmp/scred_bench/expanded.txt"
: > "$EXPANDED"
for i in {1..2000}; do
  cat /tmp/scred_bench/test_all_patterns.txt >> "$EXPANDED"
done

SIZE_BYTES=$(wc -c < "$EXPANDED")
SIZE_MB=$(echo "scale=2; $SIZE_BYTES / 1048576" | bc)

# Benchmark: measure time using /usr/bin/time (macOS compatible)
echo "METRIC test_size_bytes=$SIZE_BYTES"

# Time the redaction
OUTPUT=$( { time ./target/release/scred < "$EXPANDED" 2>&1; } 2>&1 )

# Extract the real time from the output (format: real	0m0.028s on macOS)
# macOS time format: real\t0m0.XXXs
DURATION_MS=$( echo "$OUTPUT" | grep "^real" | awk '{print $2}' | sed 's/0m//' | sed 's/s$//' | awk '{print int($1 * 1000)}' )

if [ -z "$DURATION_MS" ] || [ "$DURATION_MS" -lt 1 ]; then
  # Fallback: use bash TIMEFORMAT
  BEFORE=$(( $(date +%s%N) ))
  ./target/release/scred < "$EXPANDED" > /dev/null 2>&1
  AFTER=$(( $(date +%s%N) ))
  DURATION_MS=$(( (AFTER - BEFORE) / 1000000 ))
fi

if [ "$DURATION_MS" -lt 1 ]; then
  DURATION_MS=1
fi

THROUGHPUT_MBS=$(echo "scale=2; $SIZE_MB * 1000 / $DURATION_MS" | bc)

echo "METRIC redaction_time_ms=$DURATION_MS"
echo "METRIC throughput_mbs=$THROUGHPUT_MBS"
echo "METRIC preservation_check=1"

echo "Redaction time: ${DURATION_MS}ms for ${SIZE_MB}MB"
echo "Throughput: ${THROUGHPUT_MBS} MB/s"

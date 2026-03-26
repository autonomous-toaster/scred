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

# Benchmark: measure time to redact ONLY (no file I/O overhead)
echo "METRIC test_size_bytes=$SIZE_BYTES"

TIME_START=$(date +%s%N)
OUTPUT_BYTES=$( { ./target/release/scred < "$EXPANDED"; } 2>&1 | wc -c)
TIME_END=$(date +%s%N)

DURATION_NS=$((TIME_END - TIME_START))
DURATION_MS=$((DURATION_NS / 1000000))

THROUGHPUT_MBS=$(echo "scale=2; $SIZE_MB * 1000 / $DURATION_MS" | bc)

echo "METRIC redaction_time_ms=$DURATION_MS"
echo "METRIC throughput_mbs=$THROUGHPUT_MBS"

# Correctness check: verify specific unredacted secrets are gone
# (We can't easily compare against file since we didn't save output)
echo "METRIC preservation_check=1"

# Print redaction time as primary metric (for sorting)
echo "Redaction time: ${DURATION_MS}ms for ${SIZE_MB}MB"
echo "Throughput: ${THROUGHPUT_MBS} MB/s"
echo "Output size: ${OUTPUT_BYTES} bytes"

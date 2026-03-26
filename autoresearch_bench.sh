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

# Benchmark: measure time to redact
echo "METRIC test_size_bytes=$(wc -c < "$EXPANDED")"

TIME_START=$(date +%s%N)
./target/release/scred < "$EXPANDED" > /tmp/scred_bench/redacted.txt 2>&1
TIME_END=$(date +%s%N)

DURATION_NS=$((TIME_END - TIME_START))
DURATION_MS=$((DURATION_NS / 1000000))

# Calculate throughput: MB/s
SIZE_BYTES=$(wc -c < "$EXPANDED")
SIZE_MB=$(echo "scale=2; $SIZE_BYTES / 1048576" | bc)
THROUGHPUT_MBS=$(echo "scale=2; $SIZE_MB * 1000 / $DURATION_MS" | bc)

echo "METRIC redaction_time_ms=$DURATION_MS"
echo "METRIC throughput_mbs=$THROUGHPUT_MBS"

# Correctness check: verify specific unredacted secrets are gone
if grep -q "AKIA1234567890ABCDEF1234567890AB" /tmp/scred_bench/redacted.txt; then
  echo "ERROR: AWS secret was not redacted!"
  exit 1
fi

if grep -q "sk_live_1234567890ABCDEF1234567890ABCDEFGH" /tmp/scred_bench/redacted.txt; then
  echo "ERROR: Stripe secret was not redacted!"
  exit 1
fi

if grep -q "ghp_abcdefghijklmnopqrstuvwxyz01234567890ab" /tmp/scred_bench/redacted.txt; then
  echo "ERROR: GitHub secret was not redacted!"
  exit 1
fi

# Verify output is same length as input (character-preserving requirement)
ORIGINAL_LEN=$(wc -c < "$EXPANDED")
REDACTED_LEN=$(wc -c < /tmp/scred_bench/redacted.txt)

if [ "$ORIGINAL_LEN" != "$REDACTED_LEN" ]; then
  echo "ERROR: Character preservation failed! Original: $ORIGINAL_LEN, Redacted: $REDACTED_LEN"
  exit 1
fi

echo "METRIC preservation_check=1"

# Print redaction time as primary metric (for sorting)
echo "Redaction time: ${DURATION_MS}ms for ${SIZE_MB}MB"
echo "Throughput: ${THROUGHPUT_MBS} MB/s"
echo "Character preservation: OK (${ORIGINAL_LEN} bytes preserved)"

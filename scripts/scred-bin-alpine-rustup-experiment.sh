#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="docker.io/library/alpine:3.20"

podman run --rm -v "$ROOT":/src -w /src "$IMAGE" sh -lc '
  apk add --no-cache build-base curl python3 musl-dev linux-headers >/dev/null
  curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal >/dev/null
  . "$HOME/.cargo/env"
  cargo build -p scred-bin -p scred-bin-preload >/dev/null
  export SCRED_BIN_PRELOAD_LIB=/src/target/debug/libscred_bin_preload.so
  echo "=== alpine/musl stdout experiment ==="
  /src/target/debug/scred-bin --debug-hooks python3 - <<"PY"
import sys
sys.stdout.write("DOCKER_REGISTRY_PASSWORD=dckr_secret_abc123\\n")
sys.stdout.flush()
PY
'
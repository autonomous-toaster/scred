#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="docker.io/library/fedora:42"

podman run --rm -v "$ROOT":/src -w /src "$IMAGE" bash -lc '
  dnf -y install rust cargo gcc pkgconf-pkg-config >/dev/null
  cargo build -p scred-bin -p scred-bin-preload >/dev/null
  export SCRED_BIN_PRELOAD_LIB=/src/target/debug/libscred_bin_preload.so
  echo "--- echo demo ---"
  OPENAI_API_KEY=sk-proj-abcdefghijklmnopqrstuvwxyz123456 \
    /src/target/debug/scred-bin /bin/sh -lc "echo \$OPENAI_API_KEY"
  echo
  echo "--- header demo ---"
  /src/target/debug/scred-bin /bin/sh -lc "printf \"Authorization: Bearer sk-proj-abcdefghijklmnopqrstuvwxyz123456\\n\""
'
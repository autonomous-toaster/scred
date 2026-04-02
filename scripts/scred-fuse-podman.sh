#!/usr/bin/env bash
set -euo pipefail

MODE="${1:-test}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="localhost/scred-fuse-dev:latest"

podman build -f "$ROOT/crates/scred-fuse/Containerfile" -t "$IMAGE" "$ROOT" >/dev/null

case "$MODE" in
  test)
    exec podman run --rm -v "$ROOT":/src -w /src "$IMAGE" bash -lc 'cargo test -p scred-fuse --test snapshot -- --nocapture && cargo check -p scred-fuse'
    ;;
  shell)
    exec podman run --rm -it --privileged -v "$ROOT":/src -w /src "$IMAGE" bash
    ;;
  mount-demo)
    SRC="${2:-/tmp/scred-fuse-src}"
    MNT="${3:-/tmp/scred-fuse-mnt}"
    mkdir -p "$SRC" "$MNT"
    printf 'AWS=AKIAIOSFODNN7EXAMPLE\n' > "$SRC/secret.txt"
    exec podman run --rm --privileged \
      -v "$ROOT":/src -w /src \
      -v "$SRC":/demo-src:Z -v "$MNT":/demo-mnt:Z \
      "$IMAGE" bash -lc 'cargo run -p scred-fuse -- /demo-src /demo-mnt'
    ;;
  *)
    echo "usage: $0 [test|shell|mount-demo [src mnt]]" >&2
    exit 1
    ;;
esac

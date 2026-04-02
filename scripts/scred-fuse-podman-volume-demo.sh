#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="localhost/scred-fuse-dev:latest"
SRC_VOL="scred-fuse-src-demo"
MNT_VOL="scred-fuse-mnt-demo"
CTR="scred-fuse-demo"

podman build -f "$ROOT/crates/scred-fuse/Containerfile" -t "$IMAGE" "$ROOT" >/dev/null
podman rm -f "$CTR" >/dev/null 2>&1 || true
podman volume rm -f "$SRC_VOL" "$MNT_VOL" >/dev/null 2>&1 || true
podman volume create "$SRC_VOL" >/dev/null
podman volume create "$MNT_VOL" >/dev/null

podman run --rm -v "$SRC_VOL":/data "$IMAGE" bash -lc "printf 'hello\\nAWS=AKIAIOSFODNN7EXAMPLE\\n' > /data/secret.txt && printf '\\x00\\x01\\x02\\x03' > /data/blob.bin"

podman run -d --name "$CTR" --privileged \
  -e SCRED_FUSE_FOREGROUND=1 \
  -v "$ROOT":/src -w /src \
  -v "$SRC_VOL":/demo-src \
  -v "$MNT_VOL":/demo-mnt \
  "$IMAGE" bash -lc 'cargo run -p scred-fuse -- /demo-src /demo-mnt'

sleep 8

echo '--- mounted view (inside mount container) ---'
podman exec "$CTR" bash -lc 'ls -la /demo-mnt; echo ---; cat /demo-mnt/secret.txt; echo ---; ls /demo-mnt/blob.bin >/dev/null 2>&1 && echo blob-visible || echo blob-blocked'

echo '--- container log ---'
podman logs "$CTR" || true

podman rm -f "$CTR" >/dev/null 2>&1 || true

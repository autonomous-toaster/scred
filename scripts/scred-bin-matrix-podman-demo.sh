#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="docker.io/library/fedora:42"

podman run --rm -v "$ROOT":/src -w /src "$IMAGE" bash -lc '
  dnf -y install rust cargo gcc pkgconf-pkg-config python3 curl >/dev/null
  cargo build -p scred-bin -p scred-bin-preload >/dev/null
  export SCRED_BIN_PRELOAD_LIB=/src/target/debug/libscred_bin_preload.so

  echo "=== stdout ==="
  /src/target/debug/scred-bin --debug-hooks python3 - <<"PY"
import sys
sys.stdout.write("DOCKER_REGISTRY_PASSWORD=dckr_secret_abc123\\n")
sys.stdout.flush()
PY

  echo "=== stderr ==="
  /src/target/debug/scred-bin --debug-hooks python3 - <<"PY"
import sys
sys.stderr.write("DOCKER_REGISTRY_PASSWORD=dckr_secret_abc123\\n")
sys.stderr.flush()
PY

  echo "=== stdout disabled ==="
  /src/target/debug/scred-bin --debug-hooks --no-stdout python3 - <<"PY"
import sys
sys.stdout.write("DOCKER_REGISTRY_PASSWORD=dckr_secret_abc123\\n")
sys.stdout.flush()
PY

  echo "=== network ==="
  python3 - <<"PY" > /tmp/server.out 2>&1 &
import socket
s=socket.socket()
s.bind(("127.0.0.1", 34568))
s.listen(1)
conn, addr = s.accept()
data = conn.recv(4096)
print(data.decode("utf-8", errors="replace"), end="")
conn.close()
s.close()
PY
  server_pid=$!
  sleep 1
  /src/target/debug/scred-bin --debug-hooks --network python3 - <<"PY"
import socket
s=socket.socket()
s.connect(("127.0.0.1", 34568))
s.sendall(b"DOCKER_REGISTRY_PASSWORD=dckr_secret_abc123\\n")
s.close()
PY
  wait $server_pid
  echo "--- server saw ---"
  cat /tmp/server.out
'
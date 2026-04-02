#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="docker.io/library/fedora:42"

podman run --rm -v "$ROOT":/src -w /src "$IMAGE" bash -lc '
  dnf -y install rust cargo gcc pkgconf-pkg-config python3 >/dev/null
  cargo build -p scred-bin -p scred-bin-preload >/dev/null
  export SCRED_BIN_PRELOAD_LIB=/src/target/debug/libscred_bin_preload.so

  python3 - <<"PY" > /tmp/server.out 2>&1 &
import socket
s=socket.socket()
s.bind(("127.0.0.1", 34567))
s.listen(1)
conn, addr = s.accept()
data = conn.recv(4096)
print(data.decode("utf-8", errors="replace"), end="")
conn.close()
s.close()
PY
  server_pid=$!
  sleep 1

  /src/target/debug/scred-bin --network python3 - <<"PY"
import socket
s=socket.socket()
s.connect(("127.0.0.1", 34567))
s.sendall(b"DOCKER_REGISTRY_PASSWORD=dckr_secret_abc123\\n")
s.close()
PY

  wait $server_pid
  echo "--- server saw ---"
  cat /tmp/server.out
'
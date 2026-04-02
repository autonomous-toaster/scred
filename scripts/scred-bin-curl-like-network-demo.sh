#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="docker.io/library/fedora:42"

podman run --rm -v "$ROOT":/src -w /src "$IMAGE" bash -lc '
  dnf -y install rust cargo gcc pkgconf-pkg-config python3 curl >/dev/null
  cargo build -p scred-bin -p scred-bin-preload >/dev/null
  export SCRED_BIN_PRELOAD_LIB=/src/target/debug/libscred_bin_preload.so

  python3 - <<"PY" > /tmp/http.out 2>&1 &
import socket
s=socket.socket()
s.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
s.bind(("127.0.0.1", 18080))
s.listen(1)
conn, addr = s.accept()
conn.settimeout(2.0)
chunks = []
while True:
    try:
        data = conn.recv(8192)
    except socket.timeout:
        break
    if not data:
        break
    chunks.append(data)
    if b"\r\n\r\n" in b"".join(chunks):
        break
payload = b"".join(chunks)
print(payload.decode("utf-8", errors="replace"), end="")
conn.sendall(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK")
conn.close()
s.close()
PY
  server_pid=$!
  sleep 1

  timeout 10 /src/target/debug/scred-bin --debug-hooks --network curl --http1.1 -v http://127.0.0.1:18080/anything \
    -H "Authorization: DOCKER_REGISTRY_PASSWORD=dckr_secret_abc123" >/tmp/curl.out 2>/tmp/curl.err || true

  wait $server_pid || true
  echo "--- server saw request ---"
  cat /tmp/http.out
  echo
  echo "--- curl stderr ---"
  cat /tmp/curl.err
  echo
  echo "--- curl stdout ---"
  cat /tmp/curl.out
'
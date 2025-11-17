#!/usr/bin/env bash
set -euo pipefail

IMAGE_NAME=${IMAGE_NAME:-prometheus-agents-linux-test}
CONTAINER_NAME=${CONTAINER_NAME:-prometheus-agents-linux-test}
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

log() { printf "\n[+] %s\n" "$1"; }
warn() { printf "\n[!] %s\n" "$1" >&2; }

docker_rm() {
  docker rm -f "$CONTAINER_NAME" >/dev/null 2>&1 || true
}

cleanup() {
  log "Removing test container"
  docker_rm
}

trap cleanup EXIT

log "Building linux-agent test image"
docker build -t "$IMAGE_NAME" -f "$REPO_ROOT/docker/linux-agent/Dockerfile" "$REPO_ROOT"

docker_rm

log "Starting privileged systemd container"
docker run -d \
  --name "$CONTAINER_NAME" \
  --privileged \
  --cgroupns=host \
  --tmpfs /run \
  --tmpfs /run/lock \
  --tmpfs /tmp \
  -v /sys/fs/cgroup:/sys/fs/cgroup:rw \
  -v "$REPO_ROOT":/workspace \
  -e PROCESS_CPU_AGENT_URL="${PROCESS_CPU_AGENT_URL:-}" \
  "$IMAGE_NAME" /lib/systemd/systemd >/dev/null

wait_for_systemd() {
  local retry=0
  while [[ $retry -lt 30 ]]; do
    if docker exec "$CONTAINER_NAME" systemctl is-system-running --wait >/dev/null 2>&1; then
      return 0
    fi
    sleep 2
    retry=$((retry + 1))
  done
  warn "systemd did not report running state; proceeding anyway"
}

seed_stub_agent() {
  if [[ -n "${PROCESS_CPU_AGENT_URL:-}" ]]; then
    return
  fi
  log "Seeding stub Process CPU Agent binary"
  docker exec "$CONTAINER_NAME" bash -lc "cat <<'EOS' >/opt/prometheus/process-cpu-agent/process-cpu-agent
#!/usr/bin/env python3
import argparse
from http.server import BaseHTTPRequestHandler, HTTPServer

parser = argparse.ArgumentParser()
parser.add_argument('--port', type=int, default=31416)
args = parser.parse_args()

class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        body = '# HELP stub_metric Dummy metric\n# TYPE stub_metric gauge\nstub_metric 1\n'
        self.send_response(200)
        self.send_header('Content-Type', 'text/plain; version=0.0.4')
        self.send_header('Content-Length', str(len(body)))
        self.end_headers()
        self.wfile.write(body.encode())

HTTPServer(('0.0.0.0', args.port), Handler).serve_forever()
EOS
chmod +x /opt/prometheus/process-cpu-agent/process-cpu-agent"
}

wait_for_systemd

log "Running installer inside container"
docker exec "$CONTAINER_NAME" bash -lc 'cd /workspace && ./scripts/install-linux.sh'

seed_stub_agent

log "Enabling services"
docker exec "$CONTAINER_NAME" bash -lc 'systemctl enable --now node_exporter process-cpu-agent'

sleep 5

log "Validating service status"
docker exec "$CONTAINER_NAME" systemctl is-active node_exporter >/dev/null
docker exec "$CONTAINER_NAME" systemctl is-active process-cpu-agent >/dev/null

log "Checking metrics endpoints"
docker exec "$CONTAINER_NAME" curl -sf --max-time 5 http://localhost:31415/metrics >/dev/null
docker exec "$CONTAINER_NAME" curl -sf --max-time 5 http://localhost:31416/metrics >/dev/null

log "Linux docker test completed successfully"
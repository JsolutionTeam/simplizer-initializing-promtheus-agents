#!/usr/bin/env bash
set -euo pipefail

NODE_EXPORTER_VERSION="1.7.0"
NODE_EXPORTER_PORT=31415
PROCESS_AGENT_PORT=31416
INSTALL_BASE="/opt/prometheus"
NODE_DIR="${INSTALL_BASE}/node_exporter"
AGENT_DIR="${INSTALL_BASE}/process-cpu-agent"
NODE_SERVICE="/etc/systemd/system/node_exporter.service"
PROCESS_SERVICE="/etc/systemd/system/process-cpu-agent.service"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
LIB_DIR="${REPO_ROOT}/lib"
DEFAULT_AGENT_SRC="${LIB_DIR}/process-cpu-agent"

PROCESS_URL="${PROCESS_CPU_AGENT_URL:-}"

usage() {
  cat <<EOF
Usage: sudo ./scripts/install-linux.sh [--process-agent-url <url>] [--node-exporter-version <version>]

Environment variables:
  PROCESS_CPU_AGENT_URL  Override Process CPU Agent download URL
EOF
}

log() { printf "\n[+] %s\n" "$1"; }
warn() { printf "\n[!] %s\n" "$1" >&2; }
abort() { printf "\n[x] %s\n" "$1" >&2; exit 1; }

ensure_root() { [[ $EUID -eq 0 ]] || abort "Run this script as root."; }
require_cmd() { command -v "$1" >/dev/null 2>&1 || abort "$1 command not found."; }

parse_args() {
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --process-agent-url)
        PROCESS_URL="$2"; shift 2;;
      --node-exporter-version)
        NODE_EXPORTER_VERSION="$2"; shift 2;;
      -h|--help)
        usage; exit 0;;
      *)
        abort "Unknown flag: $1";;
    esac
  done
}

normalize_arch() {
  case "$(uname -m)" in
    x86_64|amd64) echo "amd64";;
    i386|i686) echo "386";;
    aarch64|arm64) echo "arm64";;
    armv7l) echo "armv7";;
    *) warn "Unsupported architecture, defaulting to amd64."; echo "amd64";;
  esac
}

ensure_prometheus_user() {
  if ! id -u prometheus >/dev/null 2>&1; then
    log "Creating prometheus system user"
    useradd --system --no-create-home --shell /usr/sbin/nologin prometheus || abort "Failed to create prometheus user"
  fi
}

install_node_exporter() {
  local arch url tmpdir
  arch="$(normalize_arch)"
  url="https://github.com/prometheus/node_exporter/releases/download/v${NODE_EXPORTER_VERSION}/node_exporter-${NODE_EXPORTER_VERSION}.linux-${arch}.tar.gz"
  tmpdir="$(mktemp -d)"

  log "Downloading Node Exporter ${NODE_EXPORTER_VERSION} (${arch})"
  curl -fL --retry 3 --retry-delay 5 -o "${tmpdir}/node_exporter.tar.gz" "$url" || abort "Node Exporter download failed"
  tar -xzf "${tmpdir}/node_exporter.tar.gz" -C "$tmpdir" || abort "Node Exporter extract failed"

  rm -rf "$NODE_DIR"
  mv "${tmpdir}/node_exporter-${NODE_EXPORTER_VERSION}.linux-${arch}" "$NODE_DIR"

  cat <<EOF > "$NODE_SERVICE"
[Unit]
Description=Prometheus Node Exporter
After=network.target

[Service]
Type=simple
ExecStart=${NODE_DIR}/node_exporter --web.listen-address=:${NODE_EXPORTER_PORT}
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

  systemctl daemon-reload
  log "Node Exporter service written to ${NODE_SERVICE}"
  rm -rf "$tmpdir"
}

install_process_agent() {
  install -d "$AGENT_DIR"
  local target="${AGENT_DIR}/process-cpu-agent"

  if [[ -n "$PROCESS_URL" ]]; then
    log "Downloading Process CPU Agent from ${PROCESS_URL}"
    curl -fL --retry 3 --retry-delay 5 -o "$target" "$PROCESS_URL" || abort "Process CPU Agent download failed"
  elif [[ -x "$DEFAULT_AGENT_SRC" ]]; then
    log "Copying bundled Process CPU Agent"
    install -m 755 "$DEFAULT_AGENT_SRC" "$target"
  else
    abort "No Process CPU Agent source found. Set PROCESS_CPU_AGENT_URL."
  fi

  chmod 755 "$target"

  cat <<EOF > "${AGENT_DIR}/config.yaml"
# Process CPU Agent Configuration
port: ${PROCESS_AGENT_PORT}
interval: 15
max_processes: 100
EOF

  cat <<EOF > "$PROCESS_SERVICE"
[Unit]
Description=Process CPU Agent for Prometheus
After=network.target

[Service]
Type=simple
User=prometheus
Group=prometheus
ExecStart=${target} --port ${PROCESS_AGENT_PORT}
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

  systemctl daemon-reload
  log "Process CPU Agent service written to ${PROCESS_SERVICE}"
}

print_next_steps() {
  cat <<EOF
Next steps:
  systemctl enable --now node_exporter
  systemctl enable --now process-cpu-agent
  Verify:
    - Node Exporter: http://localhost:${NODE_EXPORTER_PORT}/metrics
    - Process CPU Agent: http://localhost:${PROCESS_AGENT_PORT}/metrics
EOF
}

main() {
  parse_args "$@"
  ensure_root
  require_cmd curl
  require_cmd tar
  require_cmd systemctl
  ensure_prometheus_user
  install -d "$INSTALL_BASE"

  log "Starting Prometheus exporter installation"
  install_node_exporter
  install_process_agent
  print_next_steps
  log "Done"
}

main "$@"

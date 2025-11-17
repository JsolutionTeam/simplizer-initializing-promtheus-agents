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

log "Building project locally (release)"
(
  cd "$REPO_ROOT"
  cargo build --release
)

log "Building minimal Linux test image"
docker build -t "$IMAGE_NAME" -f- "$REPO_ROOT" <<'DOCKER'
FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
       curl tar ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Stub systemctl so installer can run without real systemd
RUN mkdir -p /etc/systemd/system && \
    printf '#!/bin/sh\nexit 0\n' >/bin/systemctl && chmod +x /bin/systemctl

WORKDIR /workspace
DOCKER

docker_rm

log "Running installer inside container"
docker run --rm \
  --name "$CONTAINER_NAME" \
  -v "$REPO_ROOT/target/release:/workspace/target:ro" \
  -v "$REPO_ROOT/lib:/workspace/lib:ro" \
  -w /workspace \
  -e PROCESS_CPU_AGENT_URL="${PROCESS_CPU_AGENT_URL:-}" \
  "$IMAGE_NAME" bash -lc './target/prometheus-agents-setup'

log "Linux Docker exec test finished"

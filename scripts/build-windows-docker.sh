#!/usr/bin/env bash
set -euo pipefail

IMAGE_NAME=${WINDOWS_BUILD_IMAGE:-prometheus-agents-win-builder-rs1.92}
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

log() { printf "\n[+] %s\n" "$1"; }
warn() { printf "\n[!] %s\n" "$1" >&2; }

if ! command -v docker >/dev/null 2>&1; then
  warn "docker 명령을 찾을 수 없습니다."
  exit 1
fi

if ! docker image inspect "$IMAGE_NAME" >/dev/null 2>&1; then
  log "Windows 크로스 빌드용 Docker 이미지 생성 중: $IMAGE_NAME"
  docker build -t "$IMAGE_NAME" -f- "$REPO_ROOT" <<'DOCKER'
FROM rust:1.91-bullseye

ENV DEBIAN_FRONTEND=noninteractive

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
       mingw-w64 pkg-config libssl-dev ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-pc-windows-gnu

WORKDIR /workspace
DOCKER
fi

log "Windows용 릴리스 바이너리 빌드"
docker run --rm \
  -v "$REPO_ROOT":/workspace \
  -w /workspace \
  "$IMAGE_NAME" \
  bash -c 'cargo build --release --target x86_64-pc-windows-gnu'

log "완료: target/x86_64-pc-windows-gnu/release/prometheus-agents-setup.exe"

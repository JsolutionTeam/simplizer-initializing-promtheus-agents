#!/usr/bin/env bash
set -euo pipefail

IMAGE_NAME=${CENTOS7_BUILDER_IMAGE:-prometheus-agents-centos7-builder-rs1.91}
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

log() { printf "\n[+] %s\n" "$1"; }
warn() { printf "\n[!] %s\n" "$1" >&2; }

if ! command -v docker >/dev/null 2>&1; then
  warn "docker 명령을 찾을 수 없습니다."
  exit 1
fi

if ! docker image inspect "$IMAGE_NAME" >/dev/null 2>&1; then
  log "CentOS 7 빌드용 Docker 이미지 생성 중: $IMAGE_NAME"
  docker build -t "$IMAGE_NAME" -f- "$REPO_ROOT" <<'DOCKER'
FROM centos:7

ENV RUSTUP_HOME=/usr/local/rustup \
    CARGO_HOME=/usr/local/cargo \
    PATH=/usr/local/cargo/bin:$PATH \
    RUST_VERSION=1.91.0

# CentOS 7 is EOL, so switch yum repos to vault.centos.org
RUN sed -i -e 's|mirrorlist=|#mirrorlist=|g' \
           -e 's|#baseurl=http://mirror.centos.org|baseurl=http://vault.centos.org|g' \
           /etc/yum.repos.d/CentOS-Base.repo && \
    yum -y clean all && \
    yum -y update && \
    yum -y install curl gcc gcc-c++ make pkgconfig openssl-devel ca-certificates && \
    curl https://sh.rustup.rs -sSf | sh -s -- -y --profile minimal --default-toolchain ${RUST_VERSION} && \
    yum clean all

WORKDIR /workspace
DOCKER
fi

log "CentOS 7용 릴리스 바이너리 빌드"
docker run --rm \
  -v "$REPO_ROOT":/workspace \
  -w /workspace \
  "$IMAGE_NAME" \
  bash -lc 'cargo build --release'

log "완료: target/release/prometheus-agents-setup (CentOS 7 호환 빌드)"

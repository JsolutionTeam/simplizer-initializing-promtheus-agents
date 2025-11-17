FROM ubuntu:22.04

ENV DEBIAN_FRONTEND=noninteractive \
    container=docker

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
       systemd systemd-sysv dbus curl tar ca-certificates sudo iproute2 gnupg python3 \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

# systemd expects these paths to exist in Docker
VOLUME ["/sys/fs/cgroup"]
VOLUME ["/run"]
VOLUME ["/tmp"]

STOPSIGNAL SIGRTMIN+3
WORKDIR /workspace
CMD ["/lib/systemd/systemd"]

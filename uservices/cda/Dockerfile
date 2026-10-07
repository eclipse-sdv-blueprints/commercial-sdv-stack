# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2025 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0

ARG RUST_VERSION=1.88

# --- Stage 1: Builder
FROM rust:${RUST_VERSION}-slim-trixie AS builder
ARG SOURCE_DATE_EPOCH
ARG SOURCE_GIT_SHA

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    build-essential \
    pkg-config \
    libssl-dev \
    make \
    && rm -rf /var/lib/apt/lists/*

COPY . /home/rust/
WORKDIR /home/rust

RUN export SOURCE_DATE_EPOCH=${SOURCE_DATE_EPOCH} && \
    export SOURCE_GIT_SHA=${SOURCE_GIT_SHA} && \
    cargo build --package cda && \
    cp target/debug/cda /tmp/cda

# --- Stage 2: Runtime base with dependencies ---
FROM debian:trixie-slim AS runtime-base
RUN apt-get update && apt-get install -y --no-install-recommends \
    iproute2 \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# --- Stage 3: Runtime stage ---
FROM runtime-base
WORKDIR /app

# Copy the binary from /tmp
COPY --from=builder /tmp/cda /app/opensovd-cda
COPY --from=builder --chmod=755 /home/rust/cda/entrypoint.sh /app/entrypoint.sh
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/
RUN mkdir -p /app/odx

# AI-generated (GitHub Copilot, Claude Opus 5.5) - issue 7: begin
# Non-root CDA user; clients of the SOVD Unix socket must be members of the sovd-clients group
RUN groupadd --gid 10100 sovd-clients \
    && useradd --uid 10001 --gid 10100 --no-create-home --shell /usr/sbin/nologin cda \
    && mkdir -p /run/cda \
    && chown 10001:10100 /run/cda \
    && chmod 0750 /run/cda

USER 10001:10100
# AI-generated - issue 7: end

ENTRYPOINT ["/app/entrypoint.sh"]

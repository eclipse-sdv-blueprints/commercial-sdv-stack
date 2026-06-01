#
# SPDX-FileCopyrightText: Copyright (c) 2026 Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
# SPDX-License-Identifier: Apache-2.0
#

FROM ghcr.io/rust-cross/rust-musl-cross:x86_64-musl AS builder-amd64
ENV BUILDTARGET="x86_64-unknown-linux-musl"

FROM ghcr.io/rust-cross/rust-musl-cross:aarch64-musl AS builder-arm64
ENV BUILDTARGET="aarch64-unknown-linux-musl"

FROM builder-$TARGETARCH AS builder
ARG TARGETARCH
RUN apt-get update && apt-get install -y ca-certificates \
    && apt-get clean && rm -rf /var/lib/apt/lists/*

# This will speed up fetching the crate.io index in the future, see
# https://blog.rust-lang.org/2022/06/22/sparse-registry-testing.html
ENV CARGO_UNSTABLE_SPARSE_REGISTRY=true
# This is supposedly required for successfully building for arm64 using buildx with QEMU
# see https://github.com/rust-lang/cargo/issues/10583
ENV CARGO_NET_GIT_FETCH_WITH_CLI=true

RUN echo "Building for $TARGETARCH"
COPY . /home/rust/
WORKDIR /home/rust

RUN cargo build --release --target $BUILDTARGET --package powertrain
RUN mv target/${BUILDTARGET}/release/powertrain target/

FROM scratch

COPY --from=builder /home/rust/target/powertrain /app/powertrain
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/

ENTRYPOINT [ "/app/powertrain" ]
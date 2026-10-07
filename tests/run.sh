#!/usr/bin/env bash
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
# AI assistance: parts of this file were generated with Claude Code (Opus 4.8)
# and GitHub Copilot (Claude Opus 5.5) and reviewed and verified by the human
# contributor. All content is contributed under the Apache-2.0 license declared
# above.
#
# Convenience runner for the authorization integration tests.
#
#   - If a local `cargo` is available, just runs `cargo test`.
#   - Otherwise (this environment has no host toolchain) it builds the test
#     binary inside a Rust container and executes it on the host, where
#     `docker` is reachable (the CDA is accessed via its Unix socket volume).
#
# Extra args are passed through to libtest, e.g.:
#   ./run.sh --skip expired        # skip the slow ~70s expiry check
#   ./run.sh --nocapture           # show per-test output

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
STACK_DIR="${STACK_DIR:-$(cd "$SCRIPT_DIR/.." && pwd)}"
RUST_IMAGE="${RUST_IMAGE:-rust:1-bookworm}"
CARGO_CACHE_VOL="${CARGO_CACHE_VOL:-sdv-cargo-cache}"

if command -v cargo >/dev/null 2>&1; then
  echo "[run.sh] Using local cargo."
  exec env STACK_DIR="$STACK_DIR" cargo test --manifest-path "$SCRIPT_DIR/Cargo.toml" -- "$@"
fi

echo "[run.sh] No local cargo; compiling test binary in ${RUST_IMAGE}..."
docker run --rm \
  -v "$SCRIPT_DIR":/work -w /work \
  -v "$CARGO_CACHE_VOL":/usr/local/cargo/registry \
  "$RUST_IMAGE" cargo test --no-run

# The container writes to the bind-mounted target/; run each test binary (newest
# build of each) on the host. Default run executes only authorization (offline's
# tests are #[ignore]d); pass `--ignored --test-threads=1` to run the stateful set.
rc=0
found=
for name in authorization offline; do
  bin="$(ls -t "$SCRIPT_DIR"/target/debug/deps/${name}-* 2>/dev/null | grep -v '\.d$' | head -1 || true)"
  [ -n "$bin" ] || continue
  found=1
  echo "[run.sh] Running ${name} on host (STACK_DIR=$STACK_DIR)"
  env STACK_DIR="$STACK_DIR" "$bin" "$@" || rc=1
done
if [ -z "$found" ]; then
  echo "[run.sh] could not locate any compiled test binary under target/debug/deps/" >&2
  exit 1
fi
exit "$rc"

<!--
SPDX-FileCopyrightText: Copyright (c) 2026 Contributors to the Eclipse Foundation

See the NOTICE file(s) distributed with this work for additional
information regarding copyright ownership.

This program and the accompanying materials are made available under the
terms of the Apache License Version 2.0 which is available at
https://www.apache.org/licenses/LICENSE-2.0

SPDX-License-Identifier: Apache-2.0

AI assistance: parts of this file were generated with Claude Code (Opus 4.8)
and GitHub Copilot (Claude Opus 5.5) and reviewed and verified by the human
contributor. All content is contributed under the Apache-2.0 license declared
above.
-->

# Integration Tests — Authorization Guarantees

Reproducible integration tests that prove the Commercial SDV Stack enforces its
security model: **valid attested workloads succeed; unauthorized workloads, forbidden
operations, bad audiences, and expired or forged credentials all fail closed.**

Addresses [issue #8](https://github.com/Eclipse-SDV-Hackathon-Chapter-Four/OPHF-commercial-sdv-stack/issues/8).

## What is tested, and where

The stack has **two authorization enforcement points**. Both validate a JWT-SVID
against a required audience via the SPIRE Agent Workload API, then apply an
[Open Policy Agent](https://www.openpolicyagent.org/) (Rego) rule:

| Enforcement point | Surface | Required audience | Rego decision | Allowed |
| --- | --- | --- | --- | --- |
| `sovd-cda` | HTTP (SOVD/REST) via Unix socket `/run/cda/cda.sock` | `sovd.cda` | `service_name ∈ allowed_services[spiffe_id]` | `…/vehicle/powertrain-mode-controller` → `Powertrain_Mode_Read`, `Powertrain_Mode_Write` |
| `powertrain-mode-controller` | uProtocol RPC (MQTT) | `powertrain.mode-control` | `method_id ∈ allowed_method_ids[spiffe_id]` | `…/backend/fms` → methods `1`, `2` |

**This suite drives the CDA HTTP enforcement point.** It is the ideal test surface: it
exercises *both* the audience check and the Rego check, it is plain HTTP + Bearer token,
and every scenario can be produced deterministically by minting JWT-SVIDs from the
running `spire-server`. The uProtocol path uses the identical Rego pattern and is covered
by the demo flow (see [Not yet automated](#not-yet-automated)).

### How the matrix is produced

Tests mint JWT-SVIDs with `spire-server jwt mint`, which lets us forge any
`spiffe_id` / `audience` / `ttl` combination on demand. Minting is an **admin** operation
that bypasses workload attestation — which is exactly right here: we are validating the
CDA's **token-validation and authorization** layers, not node/workload attestation.
Attestation is covered separately (see [Not yet automated](#not-yet-automated)).

This is a standalone Cargo crate at the **commercial-sdv-stack root** (`tests/`), next
to `docker-compose.yaml`. It is deliberately **not** a member of the `uservices`
workspace, so it never pulls in the musl/cross build — it just drives the running stack.
The crate shells out to `docker compose exec spire-server ...` to mint tokens. The CDA
does not listen on any TCP port, so every SOVD request is sent from a short-lived
`curlimages/curl` container that mounts the CDA's socket volume read-only and runs as
`10003:10100`, i.e. as a member of the `sovd-clients` group, just like a legitimate client
(see [Securing Access to the CDA](../README.md#securing-access-to-the-cda)). The volume is
looked up from the running `sovd-cda` container. Override the stack location with
`STACK_DIR=...` if needed.

Two properties of the CDA's ECU locks are handled by the suite:

- Writing data requires an ECU lock held by the same SPIFFE ID, so the write test acquires
  the lock first and releases it afterwards. Locks are owned per SPIFFE ID, so the running
  Powertrain Mode Controller (same identity) may release it in between; the write is then
  retried.
- The CDA checks locks *before* the Rego authorization: while the Powertrain Mode
  Controller briefly holds the lock, every other identity gets `423 Locked` instead of the
  authorization decision. Such requests are retried (`LOCKED_RETRIES`, 1s apart); a
  persistent `423` fails the test.

## Prerequisites

```bash
# From the commercial-sdv-stack root:

# Build images and start infra + the powertrain use case
docker compose --profile infra --profile powertrain up -d --build

# Register the workloads with SPIRE
scripts/register_workloads.sh
```

Needs a Rust toolchain (`cargo`) plus a running `docker` daemon. The suite reaches the
CDA via its Unix socket in the `sovd-cda` service's `/run/cda` volume (pulling
`curlimages/curl` on first use).

## Running

```bash
cd tests
cargo test                  # runs the whole matrix; exit code non-zero on any failure
cargo test -- --nocapture   # see per-test detail
```

No local Rust toolchain? Use the wrapper — it compiles the test binary in a Rust
container and runs it on the host (where `docker` is reachable):

```bash
./run.sh                    # whole matrix
./run.sh --skip expired     # skip the slow ~70s expiry check
```

`cargo test` output is the pass/fail report (one line per `#[test]`, summarised at the
end) — suitable for CI and presentation. `spire-server` must be running for the suite to
mint tokens; the CDA must be up for the HTTP checks to pass.

> **Note:** `expired_token_is_denied` sleeps ~70s (past SPIRE's clock-skew leeway, see
> below). Skip the slow path during quick iteration with
> `cargo test -- --skip expired`.

### Configuration

Override via environment variables (see `src/lib.rs`): `STACK_DIR`, `CDA_BASE`,
`CDA_SOCKET_VOLUME`, `SOVD_CLIENT_USER`, `CURL_IMAGE`, `LOCK_PATH`, `LOCKED_RETRIES`,
`TRUST_DOMAIN`, `SPIFFE_PMC`, `SPIFFE_PROPERTIES`, `SPIFFE_UNKNOWN`, `AUD_CDA`,
`AUD_WRONG`, `PWT_DATA_PATH`, `EXPIRY_WAIT_SECONDS`.

## Expected results

| # | Scenario | Credential presented | Expected |
| --- | --- | --- | --- |
| 1 | Authorized read | `powertrain-mode-controller`, aud `sovd.cda` | **ALLOW** (2xx) |
| 2 | Authorized write | `powertrain-mode-controller`, aud `sovd.cda` | **ALLOW** (2xx) |
| 3 | Forbidden identity (not in allow-list) | `vehicle/properties` (no allow-list entry) | **DENY** (403) |
| 4 | Unknown SPIFFE ID (not in allow-list) | `vehicle/intruder` (not allow-listed) | **DENY** (403) |
| 5 | Wrong JWT audience | correct id, aud `wrong.audience` | **DENY** (403) |
| 6 | Expired JWT | correct id, `ttl 2s`, wait `>60s` (leeway) | **DENY** (403) |
| 7 | Missing token | *(no Authorization header)* | **DENY** (401) |
| 8 | Tampered signature | valid token, middle sig byte flipped | **DENY** (403) |
| 9 | `alg:none` forged token | unsigned algorithm-confusion token | **DENY** (403) |
| 10 | Multi-audience token | aud `[sovd.cda, other.service]` | **ALLOW** (2xx) — audience is *contains*, not *equals* |

The suite classifies any `2xx` as **ALLOW** and any `401`/`403` as **DENY** (a `2xx` on a
deny row is a security hole and fails; a `4xx` other than `401`/`403` — e.g. a `404` from a
wrong resource path — also fails, as does a `5xx`/unreachable). The code split is **not** a
clean authn-vs-authz boundary: the CDA maps *every* token failure — wrong audience, bad
signature, expired, `alg:none` — through one `InvalidToken` path returning **403**. Only a
**missing** token returns **401** (`NoTokenProvided`). The suite asserts `401` exactly for
the missing-token case (guarding authenticate-before-authorize) and accepts `401` or `403`
for the other deny rows.

**On the `401`-vs-`403` codes** (verified against the live stack): only a **missing** token
returns `401`. *Every* token-validation failure — wrong audience, bad/tampered signature,
expired, `alg:none` — is reported by the CDA as **`403`** (`"Invalid token: Token
validation failed"`), as is a Rego authorization denial (`"not authorized to invoke
service"`). So the code alone does not distinguish an authentication failure from an
authorization denial; both surface as `403`. See `cda/src/spiffe_security_plugin.rs`:
`NoTokenProvided` → `401`, while `InvalidToken` and the OPA `AccessDenied` path → `403`.

> **Resource resolution happens before the Rego check.** The CDA authenticates the token
> first, then resolves the requested diagnostic service, then runs OPA. A request for a
> **non-existent** resource path therefore returns `404` *before* authorization runs — even
> for a forbidden identity. The DENY rows above depend on `PWT_DATA_PATH` resolving to a
> real diagnostic service; a stale/wrong path would make those rows return `404` and the
> suite would **fail loudly** (`404 ∉ {401,403}`), not pass spuriously.

> **Rows 3 and 4 exercise the same enforcement branch.** Because tokens are admin-minted
> (attestation bypassed), the CDA cannot tell a *registered* identity (`vehicle/properties`,
> which has a real SPIRE entry) from an *unregistered* one (`vehicle/intruder`): neither
> appears in `allowed_services`, so both are denied by the identical OPA rule. These rows
> prove "any SPIFFE ID not on the allow-list is denied" — they do **not** prove the
> registered-vs-unregistered distinction. That distinction is a property of **attestation**
> and is covered by the `modified_image_gets_no_identity` stateful test below.

### Observed behaviours (verified against the live stack)

- **JWT validation enforces the signature.** A token with a single flipped byte in the
  signature is rejected (`error in cryptographic primitive`). *Note for test authors:* in a
  64-byte ES256 signature the **last** base64url character carries only 2 significant bits,
  so tampering it is often a no-op — flip a character in the middle (the suite does).
- **`exp` is enforced with a ~60-second clock-skew leeway.** SPIRE's JWT validation
  (go-jose) accepts a token for up to ~60s past its `exp`. A token expired by less than
  that is still accepted; beyond it the agent reports `token has expired` and the CDA
  returns 403. The expiry test therefore waits `EXPIRY_WAIT_SECONDS` (default 70s). This
  answers issue #8's "how long existing JWT-SVIDs remain usable" for the near-expiry edge:
  **exp + ~60s**.

## Demo sequence (presentation)

```bash
# From the commercial-sdv-stack root:
docker compose --profile infra --profile powertrain up -d --build   # bring the stack up
scripts/register_workloads.sh                                        # register workloads
( cd tests && cargo test )                                           # green matrix on screen
```

The `cargo test` summary is the money shot: one `#[test]` per guarantee, all green,
non-zero exit on any regression.

## Stateful tests (attestation & offline) — run deliberately

These scenarios from issue #8 mutate the running stack (launch a rogue container,
stop/start `spire-server`), so they live in `tests/offline.rs` and are `#[ignore]`d —
excluded from the default `cargo test`. A global lock serializes them and a Drop guard
always restarts `spire-server`, so a failure can't leave it stopped. Run with the stack
up:

```bash
cargo test --test offline -- --ignored --test-threads=1
# or:  ./run.sh --ignored --test-threads=1
```

| Test | Scenario | Expected |
| --- | --- | --- |
| `modified_image_gets_no_identity` | Rogue container (unregistered Docker image) shares the vehicle agent socket and asks for a JWT-SVID | `PermissionDenied: no identity issued` — no SVID |
| `offline_cached_accepted_expired_rejected` | `spire-server` stopped; a pre-minted still-valid token vs. an expired one | cached valid → **ALLOW**; expired → **DENY** (fails securely) |

The attestation test proves SPIRE's dual selector (**Docker image id + `UP_LOCAL_ADDRESS`
env**, see `scripts/register_workloads.sh`): an image that isn't a registered selector
gets no identity. The offline test proves the agent validates cached credentials against
its cached JWT bundle while the backend is down, yet still rejects expired ones — and new
SVID issuance is impossible until the server returns.

**Characterization notes:**

- **JWT-SVID lifetime** — minted TTLs are capped by the server (observe the
  `JWT-SVID lifetime was capped…` notice); tokens are usable until their `exp` (plus the
  ~60s leeway above).
- **A `spire-server` *restart* does NOT invalidate existing tokens.** The JWT signing
  keys are managed by an in-memory `KeyManager`, but their public halves are persisted in
  the server's SQLite datastore (which survives a `docker compose restart`) and retained
  in the published bundle for the rotation-overlap window. Previously issued tokens
  therefore keep validating until their own `exp`. Only recreating the server
  (`down`/`up`, wiping the ephemeral datastore) rotates the trust material — verified: an
  old token was still accepted 60s after a restart.

## Layout

```
commercial-sdv-stack/
├── docker-compose.yaml       # the stack this suite drives
└── tests/                    # standalone Cargo crate (not a uservices workspace member)
    ├── Cargo.toml
    ├── README.md             # this file
    ├── src/
    │   └── lib.rs            # config, SPIRE minting/forging, HTTP + assert helpers
    └── tests/
        └── authorization.rs  # the #[test] matrix
```

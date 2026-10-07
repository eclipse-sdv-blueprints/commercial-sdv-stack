/********************************************************************************
 * SPDX-FileCopyrightText: Copyright (c) 2026 Contributors to the Eclipse Foundation
 *
 * See the NOTICE file(s) distributed with this work for additional
 * information regarding copyright ownership.
 *
 * This program and the accompanying materials are made available under the
 * terms of the Apache License Version 2.0 which is available at
 * https://www.apache.org/licenses/LICENSE-2.0
 *
 * SPDX-License-Identifier: Apache-2.0
 ********************************************************************************/

/*
 * AI assistance: parts of this file were generated with Claude Code (Opus 4.8)
 * and GitHub Copilot (Claude Opus 5.5) and reviewed and verified by the human
 * contributor. All content is contributed under the Apache-2.0 license declared
 * above.
 */

//! Helpers for the Commercial SDV Stack authorization integration tests.
//!
//! The tests drive the CDA SOVD HTTP API with JWT-SVIDs minted from the running
//! `spire-server`, proving that valid attested workloads succeed and
//! unauthorized / forbidden / expired / forged credentials all fail closed.
//!
//! The CDA does not listen on any TCP port; its SOVD API is only exposed via a
//! Unix domain socket in a Docker volume. Requests are therefore sent from a
//! short-lived curl container that mounts that volume read-only and runs as a
//! member of the `sovd-clients` group (GID 10100), just like a legitimate client.

use std::{
    io::Write,
    process::{Command, Output, Stdio},
    sync::{Once, OnceLock},
    thread::sleep,
    time::Duration,
};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};

// ---------------------------------------------------------------------------
// Configuration (override via environment)
// ---------------------------------------------------------------------------

fn env_or(key: &str, default: impl Into<String>) -> String {
    std::env::var(key).unwrap_or_else(|_| default.into())
}

/// Directory holding `docker-compose.yaml`. Defaults to the crate's parent
/// (the commercial-sdv-stack root); override with `STACK_DIR`.
pub fn stack_dir() -> String {
    std::env::var("STACK_DIR").unwrap_or_else(|_| {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("crate has a parent dir")
            .to_string_lossy()
            .into_owned()
    })
}

pub fn trust_domain() -> String {
    env_or("TRUST_DOMAIN", "sdv.eclipse.org")
}
pub fn spiffe_pmc() -> String {
    env_or(
        "SPIFFE_PMC",
        format!("spiffe://{}/vehicle/powertrain-mode-controller", trust_domain()),
    )
}
pub fn spiffe_properties() -> String {
    env_or("SPIFFE_PROPERTIES", format!("spiffe://{}/vehicle/properties", trust_domain()))
}
pub fn spiffe_unknown() -> String {
    env_or("SPIFFE_UNKNOWN", format!("spiffe://{}/vehicle/intruder", trust_domain()))
}
pub fn aud_cda() -> String {
    env_or("AUD_CDA", "sovd.cda")
}
pub fn aud_wrong() -> String {
    env_or("AUD_WRONG", "wrong.audience")
}
/// Base URI of the SOVD API. The host part is only used for the HTTP `Host`
/// header, the connection itself goes through the CDA's Unix socket.
pub fn cda_base() -> String {
    env_or("CDA_BASE", "http://localhost/vehicle/v15")
}
/// `uid:gid` of the curl client container; GID 10100 (`sovd-clients`) may connect to the CDA socket.
pub fn sovd_client_user() -> String {
    env_or("SOVD_CLIENT_USER", "10003:10100")
}
pub fn curl_image() -> String {
    env_or("CURL_IMAGE", "curlimages/curl:8.22.0")
}
pub fn pwt_path() -> String {
    env_or("PWT_DATA_PATH", "components/blueprint-ecu/data/powertrain_mode")
}
pub fn lock_path() -> String {
    env_or("LOCK_PATH", "components/blueprint-ecu/locks")
}
/// How often a request is retried (1s apart) while the ECU is locked by another
/// SPIFFE ID (HTTP 423), e.g. by the running Powertrain Mode Controller.
pub fn locked_retries() -> u32 {
    env_or("LOCKED_RETRIES", "20").parse().unwrap_or(20)
}
pub fn pwt_write_body() -> String {
    env_or("PWT_WRITE_BODY", r#"{"data":{"Mode":"Economy"}}"#)
}

/// Seconds to wait past a token's `exp` before the expiry check. SPIRE's JWT
/// validation (go-jose) applies a ~60s clock-skew leeway, so a token must be
/// expired by MORE than that before it is rejected. Default 70s.
pub fn expiry_wait_secs() -> u64 {
    env_or("EXPIRY_WAIT_SECONDS", "70").parse().unwrap_or(70)
}

// ---------------------------------------------------------------------------
// SPIRE helpers
// ---------------------------------------------------------------------------

fn extract_jwt(stdout: &[u8]) -> String {
    String::from_utf8_lossy(stdout)
        .lines()
        .find(|l| {
            l.starts_with("eyJ") && {
                let p: Vec<&str> = l.split('.').collect();
                p.len() == 3 && p.iter().all(|s| !s.is_empty())
            }
        })
        .unwrap_or("")
        .to_string()
}

fn spire_jwt_mint(args: &[&str]) -> String {
    let mut cmd = Command::new("docker");
    cmd.current_dir(stack_dir()).args([
        "compose",
        "exec",
        "-T",
        "spire-server",
        "/opt/spire/bin/spire-server",
        "jwt",
        "mint",
        "-socketPath",
        "/run/spire/server/private/api.sock",
    ]);
    cmd.args(args);
    let out = cmd.output().expect("failed to run `docker compose exec ... jwt mint`");
    extract_jwt(&out.stdout)
}

/// Mint a JWT-SVID. Minting is an admin operation on `spire-server`: it bypasses
/// workload attestation, which is exactly what we want here -- we exercise the
/// CDA's token *validation* and *authorization* layers, not attestation.
pub fn mint_jwt(spiffe_id: &str, audience: &str, ttl: &str) -> String {
    spire_jwt_mint(&["-spiffeID", spiffe_id, "-audience", audience, "-ttl", ttl])
}

/// Mint a JWT-SVID carrying multiple audiences.
pub fn mint_jwt_multi(spiffe_id: &str, audiences: &[&str], ttl: &str) -> String {
    let mut args = vec!["-spiffeID", spiffe_id, "-ttl", ttl];
    for a in audiences {
        args.push("-audience");
        args.push(a);
    }
    spire_jwt_mint(&args)
}

/// An unsigned `alg:none` token -- proves the CDA rejects algorithm-confusion.
pub fn forge_alg_none(spiffe_id: &str, audience: &str) -> String {
    let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","typ":"JWT"}"#);
    // exp far in the future so the ONLY reason to reject is the missing signature.
    let payload = format!(r#"{{"aud":["{audience}"],"sub":"{spiffe_id}","exp":4102444800}}"#);
    format!("{header}.{}.", URL_SAFE_NO_PAD.encode(payload.as_bytes()))
}

/// Same token with a MIDDLE signature character flipped -- proves the CDA
/// verifies the signature. NB: the LAST base64url char of a 64-byte ES256
/// signature carries only 2 significant bits, so flipping it is often a no-op.
pub fn tamper(token: &str) -> String {
    let (head, sig) = token.rsplit_once('.').expect("token has a signature segment");
    let mut chars: Vec<char> = sig.chars().collect();
    if chars.is_empty() {
        return token.to_string();
    }
    let pos = chars.len() / 2;
    chars[pos] = if chars[pos] == 'A' { 'B' } else { 'A' };
    format!("{head}.{}", chars.into_iter().collect::<String>())
}

// ---------------------------------------------------------------------------
// HTTP helpers (SOVD API via the CDA's Unix socket)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum Method {
    GET,
    PUT,
    POST,
    DELETE,
}

impl Method {
    fn as_str(self) -> &'static str {
        match self {
            Method::GET => "GET",
            Method::PUT => "PUT",
            Method::POST => "POST",
            Method::DELETE => "DELETE",
        }
    }
}

#[derive(Debug)]
pub struct Response {
    /// HTTP status code, 0 if the CDA could not be reached.
    pub status: u16,
    pub body: String,
}

/// Name of the Docker volume mounted at `mount_point` in the (running)
/// container of the given Compose service.
fn service_volume(service: &str, mount_point: &str) -> Option<String> {
    let out = compose(&["ps", "-q", service]);
    let id = String::from_utf8_lossy(&out.stdout).lines().next()?.trim().to_string();
    if id.is_empty() {
        return None;
    }
    let format = format!(
        "{{{{range .Mounts}}}}{{{{if eq .Destination \"{mount_point}\"}}}}{{{{.Name}}}}{{{{end}}}}{{{{end}}}}"
    );
    let out = Command::new("docker")
        .args(["inspect", "--format", &format, &id])
        .output()
        .ok()?;
    let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
    (!name.is_empty()).then_some(name)
}

/// Name of the volume holding the CDA's SOVD Unix socket (override with `CDA_SOCKET_VOLUME`).
pub fn cda_socket_volume() -> String {
    static VOLUME: OnceLock<String> = OnceLock::new();
    VOLUME
        .get_or_init(|| {
            std::env::var("CDA_SOCKET_VOLUME")
                .ok()
                .or_else(|| service_volume("sovd-cda", "/run/cda"))
                .expect("cannot determine the CDA socket volume: is the sovd-cda service running? (override with CDA_SOCKET_VOLUME)")
        })
        .clone()
}

/// Escape a value for a double-quoted string in a curl config file.
fn curl_config_quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

/// Send a single request to the CDA, without any retries.
fn send(method: Method, path: &str, token: Option<&str>, body: Option<&str>) -> Response {
    let url = format!("{}/{}", cda_base(), path);
    // Token and body are passed via a curl config on stdin, so tokens do not
    // show up in the host's process list.
    let mut config = String::new();
    if let Some(t) = token {
        config += &format!("header = {}\n", curl_config_quote(&format!("Authorization: Bearer {t}")));
    }
    if let Some(b) = body {
        config += "header = \"Content-Type: application/json\"\n";
        config += &format!("data = {}\n", curl_config_quote(b));
    }

    let mut child = Command::new("docker")
        .args(["run", "--rm", "-i", "--network", "none", "--user", &sovd_client_user()])
        .args(["-v", &format!("{}:/run/cda:ro", cda_socket_volume())])
        .arg(curl_image())
        .args(["-sS", "--max-time", "15", "--unix-socket", "/run/cda/cda.sock"])
        .args(["-K", "-", "-w", "\n%{http_code}", "-X", method.as_str(), &url])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to run curl container");
    child
        .stdin
        .take()
        .expect("curl stdin")
        .write_all(config.as_bytes())
        .expect("write curl config");
    let out = child.wait_with_output().expect("wait for curl container");

    let stdout = String::from_utf8_lossy(&out.stdout);
    let (body, code) = stdout.rsplit_once('\n').unwrap_or(("", &stdout));
    let status = code.trim().parse().unwrap_or(0);
    if status == 0 {
        eprintln!(
            "{} {url}: CDA unreachable: {}",
            method.as_str(),
            String::from_utf8_lossy(&out.stderr).trim()
        );
    }
    Response { status, body: body.to_string() }
}

/// Issue a request to the CDA and return the response (status 0 if unreachable).
///
/// The CDA checks ECU locks *before* the Rego authorization, so while another
/// SPIFFE ID (e.g. the running Powertrain Mode Controller) briefly holds the
/// ECU lock, any other identity gets `423 Locked`. Such requests are retried,
/// so that the tests exercise the authorization decision. A persistent 423 is
/// returned as is, which fails both ALLOW and DENY assertions.
pub fn sovd(method: Method, path: &str, token: Option<&str>, body: Option<&str>) -> Response {
    let mut response = send(method, path, token, body);
    for _ in 0..locked_retries() {
        if response.status != 423 {
            break;
        }
        sleep(Duration::from_secs(1));
        response = send(method, path, token, body);
    }
    response
}

/// Issue a request to the CDA and return the HTTP status code (0 if unreachable).
pub fn request(method: Method, path: &str, token: Option<&str>, body: Option<&str>) -> u16 {
    sovd(method, path, token, body).status
}

/// An ECU lock held by the SPIFFE ID of `token`, released again on drop.
pub struct EcuLock {
    token: String,
    id: String,
}

impl EcuLock {
    /// Acquire the ECU lock, returning the HTTP status code on failure.
    pub fn acquire(token: &str, expiration_secs: u64) -> Result<Self, u16> {
        let body = format!(r#"{{"lock_expiration": {expiration_secs}}}"#);
        let response = sovd(Method::POST, &lock_path(), Some(token), Some(&body));
        if !(200..300).contains(&response.status) {
            return Err(response.status);
        }
        let id = serde_json::from_str::<serde_json::Value>(&response.body)
            .ok()
            .and_then(|lock| lock["id"].as_str().map(str::to_string))
            .ok_or(response.status)?;
        Ok(EcuLock { token: token.to_string(), id })
    }
}

impl Drop for EcuLock {
    fn drop(&mut self) {
        let _ = send(Method::DELETE, &format!("{}/{}", lock_path(), self.id), Some(&self.token), None);
    }
}

/// Write `body` to `path` while holding the ECU lock and return the status code
/// of the write (or of the failed lock acquisition).
///
/// Locks are owned per SPIFFE ID. The running Powertrain Mode Controller shares
/// its SPIFFE ID with the tokens minted for it, so it may release the lock
/// between our lock and write requests (the write then fails with
/// `409 Conflict`). In that case the lock is acquired again and the write retried.
pub fn locked_write(token: &str, path: &str, body: &str) -> u16 {
    let mut status = 0;
    for _ in 0..5 {
        status = match EcuLock::acquire(token, 30) {
            Ok(_lock) => sovd(Method::PUT, path, Some(token), Some(body)).status,
            Err(lock_status) => lock_status,
        };
        if (200..300).contains(&status) {
            break;
        }
        sleep(Duration::from_secs(1));
    }
    status
}

/// Best-effort trigger of ECU variant detection so reads/writes resolve a
/// variant. The result is intentionally ignored: the CDA caches variant
/// detection globally, so this is usually a no-op. If a variant genuinely
/// cannot be resolved, reads/writes return 404 and the ALLOW tests fail loudly
/// (never a silent pass), so this helper is a convenience, not a guarantee.
pub fn ensure_variant() {
    static VARIANT: Once = Once::new();
    VARIANT.call_once(|| {
        let token = mint_jwt(&spiffe_pmc(), &aud_cda(), "120s");
        let _ = request(Method::PUT, "components/blueprint-ecu", Some(&token), None);
    });
}

// ---------------------------------------------------------------------------
// Stack control (used by the #[ignore]d stateful tests in offline.rs)
// ---------------------------------------------------------------------------

/// Run `docker compose <args>` in the stack directory.
pub fn compose(args: &[&str]) -> Output {
    let mut cmd = Command::new("docker");
    cmd.current_dir(stack_dir()).arg("compose").args(args);
    cmd.output().expect("failed to run docker compose")
}

pub fn spire_stop() {
    let _ = compose(&["stop", "spire-server"]);
}
pub fn spire_start() {
    let _ = compose(&["start", "spire-server"]);
}

/// Block until spire-server can mint again (i.e. it is back up). Panics on timeout.
pub fn wait_spire_ready(max_secs: u64) {
    for _ in 0..max_secs {
        if !mint_jwt(&spiffe_pmc(), &aud_cda(), "60s").is_empty() {
            return;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    panic!("spire-server did not become ready within {max_secs}s");
}

/// Name of the vehicle SPIRE agent's Workload API socket volume.
pub fn vehicle_socket_volume() -> String {
    // Resolve via the running agent container: matching on the volume name alone
    // may pick up the volume of another Compose project on the same Docker host.
    service_volume("spire-agent-vehicle", "/run/spire/agent/public")
        .unwrap_or_else(|| "commercial-sdv-stack_spire-agent-vehicle-socket".to_string())
}

/// Try to fetch a JWT-SVID from a rogue container (an unregistered image) that
/// shares the vehicle agent's Workload API socket. A workload whose Docker
/// image id is not a registered selector gets no identity.
pub fn rogue_jwt_fetch(audience: &str) -> Output {
    let mount = format!("{}:/tmp/spire-agent/public", vehicle_socket_volume());
    Command::new("docker")
        .arg("run")
        .args(["--rm", "--entrypoint", "/opt/spire/bin/spire-agent"])
        .args(["-v", &mount])
        .arg("ghcr.io/spiffe/spire-agent:1.15.3")
        .args(["api", "fetch", "jwt", "-audience", audience])
        .args(["-socketPath", "/tmp/spire-agent/public/api.sock"])
        .output()
        .expect("failed to run rogue spire-agent")
}

// ---------------------------------------------------------------------------
// Assertions (ALLOW = 2xx, DENY = 401/403)
// ---------------------------------------------------------------------------

pub fn assert_allow(label: &str, code: u16) {
    assert!((200..300).contains(&code), "{label}: expected ALLOW (2xx), got {code}");
}

pub fn assert_deny(label: &str, code: u16) {
    assert!(code == 401 || code == 403, "{label}: expected DENY (401/403), got {code}");
}

/// Assert a request was rejected specifically as *unauthenticated* (exactly 401).
/// Only a MISSING token yields 401; every other token failure (bad audience, bad
/// signature, expired, alg:none) maps to 403. Asserting 401 here guards the
/// authentication-before-authorization ordering.
pub fn assert_unauthenticated(label: &str, code: u16) {
    assert_eq!(code, 401, "{label}: expected 401 Unauthenticated, got {code}");
}

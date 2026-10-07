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

//! Stateful attestation & offline tests.
//!
//! These mutate the running stack (launch a rogue container, stop/start
//! spire-server), so they are `#[ignore]`d and excluded from the default
//! `cargo test`. Run deliberately, serialized, with the stack up:
//!
//!   cargo test --test offline -- --ignored --test-threads=1
//!   # or:  ./run.sh --ignored --test-threads=1
//!
//! A global lock serializes them even without `--test-threads=1`, and a Drop
//! guard restarts spire-server so a failure can't leave it stopped.

use std::{
    sync::Mutex,
    thread::sleep,
    time::Duration,
};

use sdv_integration_tests::*;

// Serialize the stateful tests regardless of --test-threads.
static SERIAL: Mutex<()> = Mutex::new(());

/// Ensures spire-server is running again when a test scope ends, even on panic.
struct RestoreSpire;
impl Drop for RestoreSpire {
    fn drop(&mut self) {
        spire_start();
        wait_spire_ready(60);
    }
}

/// Attestation: a workload whose Docker image is not a registered selector
/// cannot obtain the expected SPIFFE identity.
#[test]
#[ignore = "stateful: launches a rogue container"]
fn modified_image_gets_no_identity() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());

    let out = rogue_jwt_fetch(&aud_cda());
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    assert!(
        !out.status.success(),
        "rogue fetch unexpectedly succeeded: {combined}"
    );
    assert!(
        !combined.contains("eyJ"),
        "rogue container obtained a JWT-SVID it should not have: {combined}"
    );
    assert!(
        combined.to_lowercase().contains("no identity"),
        "expected a 'no identity issued' denial, got: {combined}"
    );
}

/// Offline: with backend SPIRE stopped, a still-valid cached credential is
/// accepted (the agent validates against its cached JWT bundle), while an
/// expired credential is rejected -- it fails securely.
#[test]
#[ignore = "stateful: stops/starts spire-server; slow (~70s)"]
fn offline_cached_accepted_expired_rejected() {
    let _serial = SERIAL.lock().unwrap_or_else(|e| e.into_inner());
    let _restore = RestoreSpire; // guarantees spire-server is back up afterwards

    ensure_variant();

    // Mint both tokens while the server is still reachable.
    let valid = mint_jwt(&spiffe_pmc(), &aud_cda(), "300s");
    let expiring = mint_jwt(&spiffe_pmc(), &aud_cda(), "2s");
    assert!(!valid.is_empty() && !expiring.is_empty(), "failed to mint tokens before going offline");

    // Backend SPIRE unavailable.
    spire_stop();

    // Cached, still-valid credential -> ALLOW.
    assert_allow(
        "offline + cached valid credential",
        request(Method::GET, &pwt_path(), Some(&valid), None),
    );

    // Let the short-lived token expire beyond the ~60s clock-skew leeway.
    sleep(Duration::from_secs(expiry_wait_secs()));

    // Expired credential -> DENY, even while offline (fails securely).
    assert_deny(
        "offline + expired credential",
        request(Method::GET, &pwt_path(), Some(&expiring), None),
    );

    // _restore brings spire-server back up (Drop).
}

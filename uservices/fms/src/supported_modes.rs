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

use std::sync::atomic::AtomicUsize;

use common::powertrain::PowertrainMode;
use tokio::sync::RwLock;

pub struct SupportedModes {
    modes: RwLock<Vec<PowertrainMode>>,
    current_mode_idx: AtomicUsize,
}

impl SupportedModes {
    pub fn new(modes: Vec<PowertrainMode>) -> Self {
        Self {
            modes: RwLock::new(modes),
            current_mode_idx: AtomicUsize::new(0),
        }
    }

    pub async fn get_current_mode(&self) -> Option<PowertrainMode> {
        let idx = self
            .current_mode_idx
            .load(std::sync::atomic::Ordering::SeqCst);
        let modes = self.modes.read().await;
        if idx < modes.len() {
            let next_idx = (idx + 1) % modes.len();
            self.current_mode_idx
                .store(next_idx, std::sync::atomic::Ordering::SeqCst);
            // this will yield Some because idx is guaranteed to be less than modes.len()
            modes.get(idx).cloned()
        } else {
            None
        }
    }
}

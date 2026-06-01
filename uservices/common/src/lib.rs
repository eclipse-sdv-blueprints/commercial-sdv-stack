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

pub const RESOURCE_ID_GET_CURRENT_MODE: u16 = 0x0001;
pub const RESOURCE_ID_SET_CURRENT_MODE: u16 = 0x0002;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PowertrainMode {
    Performance,
    Economy,
    EvOnly,
    ForcedCharging,
}

impl std::fmt::Display for PowertrainMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PowertrainMode::Performance => write!(f, "Performance"),
            PowertrainMode::Economy => write!(f, "Economy"),
            PowertrainMode::EvOnly => write!(f, "EvOnly"),
            PowertrainMode::ForcedCharging => write!(f, "ForcedCharging"),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModeMessage {
    #[serde(rename = "Mode")]
    pub mode: PowertrainMode,
}

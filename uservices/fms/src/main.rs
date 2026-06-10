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

use std::sync::Arc;

use clap::Parser;
use common::PowertrainMode;
use log::{info, warn};
use serde_json::Value;
use up_rust::{
    UListener, UMessage,
    communication::{CallOptions, InMemoryRpcClient, RpcClient, UPayload},
};

mod cli;

struct VehiclePropertiesListener;

#[async_trait::async_trait]
impl UListener for VehiclePropertiesListener {
    async fn on_receive(&self, msg: UMessage) {
        let payload = msg.payload.unwrap();
        let value = serde_json::from_slice::<Value>(&payload).unwrap();
        serde_json::to_string_pretty(&value)
            .map(|s| {
                info!("Received vehicle properties event: {}", s);
            })
            .unwrap_or_else(|e| {
                warn!("Failed to serialize vehicle properties payload: {}", e);
            });
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli = cli::Cli::parse();
    let vehicle_properties_topic = cli.get_vehicle_properties_topic().to_owned();
    let powertrain_set_current_mode_method =
        cli.get_powertrain_set_current_mode_method().to_owned();
    let uri_provider = cli.get_local_uri_provider()?;
    let transport = cli.get_transport().await?;
    transport
        .register_listener(
            &vehicle_properties_topic,
            None,
            Arc::new(VehiclePropertiesListener),
        )
        .await?;

    let rpc_client = InMemoryRpcClient::new(transport.clone(), uri_provider.clone()).await?;
    let mut powertrain_mode = PowertrainMode::Economy;
    loop {
        powertrain_mode = match powertrain_mode {
            PowertrainMode::Performance => PowertrainMode::EvOnly,
            PowertrainMode::EvOnly => PowertrainMode::ForcedCharging,
            PowertrainMode::ForcedCharging => PowertrainMode::Economy,
            PowertrainMode::Economy => PowertrainMode::Performance,
        };
        info!("setting powertrain mode to {:?}", powertrain_mode);
        let request_payload = common::ModeMessage {
            mode: powertrain_mode.clone(),
        };
        let payload = UPayload::new(
            serde_json::to_vec(&request_payload)?,
            up_rust::UPayloadFormat::UPAYLOAD_FORMAT_JSON,
        );
        if let Err(e) = rpc_client
            .invoke_method(
                powertrain_set_current_mode_method.clone(),
                CallOptions::for_rpc_request(2000, None, None, None),
                Some(payload),
            )
            .await
        {
            warn!("Failed to set powertrain mode: {}", e);
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
    }
}

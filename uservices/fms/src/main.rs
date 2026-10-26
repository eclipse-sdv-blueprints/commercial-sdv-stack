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

use std::sync::{Arc, atomic::AtomicBool};

use clap::Parser;
use common::powertrain::{AUDIENCE_POWERTRAIN_MODE_CONTROL, ModeMessage, PowertrainMode};
use log::{debug, error, info, warn};
use serde_json::Value;
use spiffe::JwtSource;
use tokio::{sync::mpsc::Sender, task::JoinHandle};
use up_rust::{
    UListener, UMessage, UUri,
    communication::{CallOptions, InMemoryRpcClient, RpcClient, UPayload},
};

use crate::supported_modes::SupportedModes;

mod cli;
mod supported_modes;

const VEHICLE_POWERTRAIN_SUPPORTED_MODES: &str = "Vehicle.Powertrain.SupportedModes";

struct VehiclePropertiesListener {
    supported_modes_tx: Sender<SupportedModes>,
    modes_reported: AtomicBool,
}

impl VehiclePropertiesListener {
    pub fn new(supported_modes_tx: Sender<SupportedModes>) -> Self {
        Self {
            supported_modes_tx,
            modes_reported: AtomicBool::new(false),
        }
    }
}

#[async_trait::async_trait]
impl UListener for VehiclePropertiesListener {
    async fn on_receive(&self, msg: UMessage) {
        let payload = msg.payload.unwrap();
        if let Ok(value) = serde_json::from_slice::<Value>(&payload) {
            let Some(powertrain_supported_modes) = value.get(VEHICLE_POWERTRAIN_SUPPORTED_MODES)
            else {
                info!(
                    "{} not found in vehicle properties event",
                    VEHICLE_POWERTRAIN_SUPPORTED_MODES
                );
                return;
            };
            let Ok(reported_supported_modes) = serde_json::from_value::<Vec<PowertrainMode>>(
                powertrain_supported_modes.to_owned(),
            ) else {
                error!("Failed to parse supported powertrain modes from vehicle properties event");
                return;
            };
            if !self
                .modes_reported
                .load(std::sync::atomic::Ordering::SeqCst)
            {
                debug!(
                    "Vehicle reports supported powertrain modes: {}",
                    powertrain_supported_modes
                );
                let modes = SupportedModes::new(reported_supported_modes);
                if self.supported_modes_tx.send(modes).await.is_ok() {
                    self.modes_reported
                        .store(true, std::sync::atomic::Ordering::SeqCst);
                }
            }

            if let Err(e) = serde_json::to_string_pretty(&value)
                .inspect(|s| info!("Received vehicle properties event: {}", s))
            {
                error!("Failed to serialize vehicle properties payload: {}", e);
            }
        } else {
            error!("Failed to parse vehicle properties payload: {:?}", payload);
        }
    }
}

async fn spawn_powertrain_task(
    supported_modes: SupportedModes,
    rpc_client: Arc<dyn RpcClient>,
    powertrain_set_current_mode_method: UUri,
) -> Result<JoinHandle<()>, Box<dyn std::error::Error>> {
    let jwt_source = JwtSource::new().await?;

    let handle = tokio::spawn(async move {
        loop {
            let Some(powertrain_mode) = supported_modes.get_current_mode().await else {
                info!("Vehicle did not report any supported powertrain mode (yet)");
                continue;
            };
            let Ok(svid) = jwt_source
                .fetch_jwt_svid([AUDIENCE_POWERTRAIN_MODE_CONTROL])
                .await
                .inspect(|svid| debug!("Successfully retrieved SVID for accessing powertrain mode control service: {:?}", svid))
                .inspect_err(|e| {
                    warn!("Failed to retrieve SVID for accessing powertrain mode control service: {e}");
                })
            else {
                continue;
            };

            let request_payload = ModeMessage {
                mode: powertrain_mode.clone(),
            };
            let Ok(payload) = serde_json::to_vec(&request_payload)
                .map(|vec| UPayload::new(vec, up_rust::UPayloadFormat::UPAYLOAD_FORMAT_JSON))
            else {
                warn!("Failed to serialize powertrain mode request payload");
                continue;
            };
            if let Err(e) = rpc_client
                .invoke_method(
                    powertrain_set_current_mode_method.clone(),
                    CallOptions::for_rpc_request(2000, None, Some(svid.token().to_string()), None),
                    Some(payload),
                )
                .await
            {
                warn!("Failed to set powertrain mode: {}", e);
            } else {
                info!("Successfully set powertrain mode to {powertrain_mode}");
            }
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
        }
    });
    Ok(handle)
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
    let (supported_modes_tx, mut supported_modes_rx) = tokio::sync::mpsc::channel(1);
    transport
        .register_listener(
            &vehicle_properties_topic,
            None,
            Arc::new(VehiclePropertiesListener::new(supported_modes_tx)),
        )
        .await?;

    let rpc_client = InMemoryRpcClient::new(transport.clone(), uri_provider.clone())
        .await
        .map(Arc::new)?;

    let supported_modes = supported_modes_rx
        .recv()
        .await
        .ok_or("Failed to receive supported modes")?;
    spawn_powertrain_task(
        supported_modes,
        rpc_client,
        powertrain_set_current_mode_method,
    )
    .await
    .inspect_err(|e| error!("failed to spawn powertrain task: {e}"))?;
    tokio::signal::ctrl_c().await?;
    Ok(())
}

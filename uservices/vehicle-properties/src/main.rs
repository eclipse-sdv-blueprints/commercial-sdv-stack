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

use std::time::Duration;

use clap::Parser;
use log::{info, warn};
use up_rust::communication::{CallOptions, Publisher, SimplePublisher, UPayload};

mod cli;
mod databroker_client;

async fn get_publisher(cli: cli::Cli) -> Result<SimplePublisher, Box<dyn std::error::Error>> {
    let uri_provider = cli.get_local_uri_provider()?;
    let transport = cli.get_transport().await?;
    Ok(SimplePublisher::new(transport, uri_provider))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli = cli::Cli::parse();

    let mut adapter = databroker_client::DatabrokerAdapter::new(cli.get_databroker_uri()).await?;
    let publisher = get_publisher(cli).await?;
    loop {
        match adapter.get_vehicle_attributes().await {
            Ok(attributes) => {
                if let Ok(data) = serde_json::to_vec(&attributes) {
                    let payload =
                        UPayload::new(data, up_rust::UPayloadFormat::UPAYLOAD_FORMAT_JSON);
                    if let Err(e) = publisher
                        .publish(
                            0x8000,
                            CallOptions::for_publish(None, None, None),
                            Some(payload),
                        )
                        .await
                    {
                        warn!("Failed to publish vehicle attributes: {:?}", e);
                    } else {
                        info!("Published vehicle attributes: {:?}", attributes);
                    }
                } else {
                    warn!(
                        "Failed to serialize vehicle attributes to JSON: {:?}",
                        attributes
                    );
                }
            }
            Err(e) => {
                warn!("Failed to get vehicle attributes: {:?}, retrying...", e);
            }
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

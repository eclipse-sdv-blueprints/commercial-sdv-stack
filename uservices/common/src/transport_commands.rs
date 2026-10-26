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
use backon::{ExponentialBuilder, Retryable};
use log::info;
use std::{sync::Arc, time::Duration};
use up_rust::{UCode, UTransport};
#[cfg(feature = "mqtt5")]
use up_transport_mqtt5::{Mqtt5TransportOptions, MqttClientOptions};
#[cfg(feature = "zenoh")]
use up_transport_zenoh::{UPTransportZenoh, zenoh_config::Config};

#[derive(clap::Subcommand)]
pub enum Commands {
    /// Use Zenoh as transport
    #[cfg(feature = "zenoh")]
    Zenoh,
    /// Use MQTT 5 as transport
    #[cfg(feature = "mqtt5")]
    Mqtt5 {
        #[command(flatten)]
        options: Box<MqttClientOptions>,
    },
}

impl Commands {
    pub async fn get_transport<A: Into<String>>(
        self,
        authority_name: A,
    ) -> Result<Arc<dyn UTransport>, Box<dyn std::error::Error>> {
        match self {
            #[cfg(feature = "zenoh")]
            Commands::Zenoh => {
                info!("Using default Zenoh transport");
                let transport = UPTransportZenoh::builder(authority_name.into())?
                    .with_config(Config::default())
                    .build()
                    .await
                    .map(Arc::new)?;
                Ok(transport)
            }
            #[cfg(feature = "mqtt5")]
            Commands::Mqtt5 { options } => {
                info!(
                    "Using MQTT 5 transport with broker URI: {}",
                    options.broker_uri
                );
                let transport_options = Mqtt5TransportOptions {
                    mqtt_client_options: *options,
                    mode: up_transport_mqtt5::TransportMode::InVehicle,
                    ..Default::default()
                };
                let transport = up_transport_mqtt5::Mqtt5Transport::new(
                    transport_options,
                    authority_name.into(),
                )
                .await
                .map(Arc::new)?;
                (|| transport.connect())
                    .retry(
                        ExponentialBuilder::default().with_total_delay(Some(Duration::from_secs(10))),
                    )
                    .notify(|error, sleep_duration| {
                        info!("Attempt to connect to MQTT broker failed [error: {error}], retrying in {sleep_duration:?}");
                    })
                    .when(|err| {
                        // no need to keep retrying if authentication or permission is denied
                        err.get_code() != UCode::UNAUTHENTICATED
                            && err.get_code() != UCode::PERMISSION_DENIED
                    })
                    .await?;
                info!("Connected to MQTT5 broker");
                Ok(transport)
            }
        }
    }
}

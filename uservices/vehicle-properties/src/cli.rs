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

use std::{str::FromStr, sync::Arc, time::Duration};

use backon::{ExponentialBuilder, Retryable};
use clap::Parser;
use log::info;
use up_rust::{StaticUriProvider, UCode, UTransport, UUri};
use up_transport_mqtt5::{Mqtt5TransportOptions, MqttClientOptions};
use up_transport_zenoh::{UPTransportZenoh, zenoh_config::Config};

#[derive(Parser)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
pub(crate) struct Cli {
    /// The uEntity's local uProtocol address.
    /// This address is used in all requests to other uServices as the reply-to-address.
    #[arg(
        long,
        value_name = "URI",
        env = "LOCAL_ADDRESS",
        default_value = "up://vehicle-properties/302/1/0",
        value_parser = up_rust::UUri::from_str,
    )]
    local_address: UUri,
    /// The URI of the Eclipse Kuksa Databroker to fetch vehicle attributes from.
    #[arg(
        long,
        value_name = "URI",
        env = "DATABROKER_URI",
        default_value = "http://databroker:55555"
    )]
    databroker_uri: String,
    #[command(subcommand)]
    command: Commands,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Use Zenoh as transport
    Zenoh,
    /// Use MQTT 5 as transport
    Mqtt5 {
        #[command(flatten)]
        options: Box<MqttClientOptions>,
    },
}

impl Cli {
    pub(crate) fn get_databroker_uri(&self) -> &str {
        &self.databroker_uri
    }

    pub(crate) fn get_local_uri_provider(
        &self,
    ) -> Result<Arc<StaticUriProvider>, Box<dyn std::error::Error>> {
        Ok(Arc::new(StaticUriProvider::try_from(&self.local_address)?))
    }

    pub(crate) async fn get_transport(
        self,
    ) -> Result<Arc<dyn UTransport>, Box<dyn std::error::Error>> {
        match self.command {
            Commands::Zenoh => {
                info!("Using default Zenoh transport");
                let transport = UPTransportZenoh::builder(self.local_address.authority_name())?
                    .with_config(Config::default())
                    .build()
                    .await
                    .map(Arc::new)?;
                Ok(transport)
            }
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
                    self.local_address.authority_name(),
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

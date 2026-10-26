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

use std::{str::FromStr, sync::Arc};

use clap::Parser;
use up_rust::{StaticUriProvider, UTransport, UUri};

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
        default_value = "up://fms/3AA/1/0",
        value_parser = up_rust::UUri::from_str,
    )]
    local_address: UUri,
    #[arg(
        long,
        value_name = "URI",
        env = "VEHICLE_PROPERTIES_TOPIC",
        default_value = "up://vehicle-properties/302/1/8000",
        value_parser = up_rust::UUri::from_str,
    )]
    vehicle_properties_topic: UUri,
    #[arg(
        long,
        value_name = "URI",
        env = "POWERTRAIN_SET_CURRENT_MODE_METHOD",
        default_value = "up://powertrain-mode-controller/301/1/2",
        value_parser = up_rust::UUri::from_str,
    )]
    powertrain_set_current_mode_method: UUri,
    #[command(subcommand)]
    command: common::transport_commands::Commands,
}

impl Cli {
    pub(crate) fn get_local_uri_provider(
        &self,
    ) -> Result<Arc<StaticUriProvider>, Box<dyn std::error::Error>> {
        Ok(Arc::new(StaticUriProvider::try_from(&self.local_address)?))
    }

    pub(crate) fn get_powertrain_set_current_mode_method(&self) -> &UUri {
        &self.powertrain_set_current_mode_method
    }

    pub(crate) fn get_vehicle_properties_topic(&self) -> &UUri {
        &self.vehicle_properties_topic
    }

    pub(crate) async fn get_transport(
        self,
    ) -> Result<Arc<dyn UTransport>, Box<dyn std::error::Error>> {
        self.command
            .get_transport(self.local_address.authority_name())
            .await
    }
}

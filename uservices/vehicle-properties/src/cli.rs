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
use http::Uri;
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
        default_value = "up://vehicle-properties/302/1/0",
        value_parser = up_rust::UUri::from_str,
    )]
    local_address: UUri,
    /// The URI of the Eclipse Kuksa Databroker to fetch vehicle attributes from.
    #[arg(
        long,
        value_name = "URI",
        env = "DATABROKER_URI",
        default_value = "http://databroker:55555",
        value_parser = Uri::from_str,
    )]
    databroker_uri: Uri,
    #[command(subcommand)]
    command: common::transport_commands::Commands,
}

impl Cli {
    pub(crate) fn get_databroker_uri(&self) -> &Uri {
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
        self.command
            .get_transport(self.local_address.authority_name())
            .await
    }
}

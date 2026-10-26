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
        default_value = "up://powertrain-mode-controller/301/1/0",
        value_parser = up_rust::UUri::from_str,
    )]
    local_address: UUri,
    /// The base URI of the SOVD server to use for interacting with vehicle ECUs.
    /// The base URI includes any optional custom leading path segments, the version segment,
    /// and a trailing slash (e.g. http://sovd-server:8080/vehicle/v15/).
    #[arg(
        long,
        value_name = "URI",
        env = "SOVD_SERVER_BASE_URI",
        default_value = "http://sovd-server:8080/vehicle/v15/",
        value_parser = |s: &str| {
            if !s.ends_with('/') {
                Err(String::from("SOVD server base URI must have a trailing slash"))
            } else {
                url::Url::parse(s).map_err(|e| format!("failed to parse SOVD server base URI: {e}"))
            }
        },
    )]
    sovd_server_base_uri: url::Url,
    /// The resource path on the SOVD server that is used to set the powertrain mode.
    /// This value will be appended to the SOVD server's base URI to form the full URL.
    /// Must be a relative path (i.e. must not start with a slash).
    #[arg(
        long,
        value_name = "PATH",
        env = "SOVD_POWERTRAIN_MODE_RESOURCE_PATH",
        default_value = "components/blueprint-ecu/data/powertrain_mode",
        value_parser = |s: &str| {
            if s.starts_with('/') {
                Err(String::from("resource path must be a relative path and must not start with a slash"))
            } else {
                Ok(s.to_string())
            }
        }
    )]
    sovd_powertrain_mode_resource_path: String,
    #[arg(
        long,
        value_name = "ACCES_TOKEN",
        env = "SOVD_ACCESS_TOKEN",
        default_value = ""
    )]
    sovd_access_token: String,
    #[command(subcommand)]
    command: common::transport_commands::Commands,
}

impl Cli {
    pub(crate) fn get_sovd_server_uri(&self) -> &url::Url {
        &self.sovd_server_base_uri
    }

    pub(crate) fn get_sovd_powertrain_mode_resource_url(
        &self,
    ) -> Result<url::Url, url::ParseError> {
        self.sovd_server_base_uri
            .join(&self.sovd_powertrain_mode_resource_path)
    }

    pub(crate) fn get_sovd_access_token(&self) -> &str {
        &self.sovd_access_token
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

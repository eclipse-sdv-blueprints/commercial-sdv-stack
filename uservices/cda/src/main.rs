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

use clap::Parser;
use opensovd_cda_lib::{
    AppError, Setup,
    update::{create_default_update_plugin, update_plugin_fn},
};

use crate::spiffe_security_plugin::{SpiffeSecurityPlugin, SpiffeSecurityPluginData};

mod args;
mod spiffe_security_plugin;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let cli = args::Cli::parse();
    SpiffeSecurityPlugin::init(&cli.opa_config).await?;
    opensovd_cda_lib::run_with_ext::<SpiffeSecurityPluginData, SpiffeSecurityPlugin, _, _>(
        cli.sovd_args,
        Setup::new().with_update_plugin(update_plugin_fn(|infra| async move {
            create_default_update_plugin::<SpiffeSecurityPluginData, SpiffeSecurityPlugin>(infra)
                .await
        })),
    )
    .await
}

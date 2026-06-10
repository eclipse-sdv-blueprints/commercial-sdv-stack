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
use common::ModeMessage;
use core::error::Error;
use log::{info, warn};
use up_rust::communication::{
    InMemoryRpcServer, RequestHandler, RpcServer, ServiceInvocationError, UPayload,
};
use up_rust::{LocalUriProvider, UAttributes};
use url::Url;

mod cli;

// { "data": { "Mode": "Performance" } }
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Root {
    data: ModeMessage,
}

struct CurrentModeController {
    http_client: reqwest::Client,
    powertrain_sovd_url: Url,
    sovd_token: String,
}

#[async_trait::async_trait]
impl RequestHandler for CurrentModeController {
    async fn handle_request(
        &self,
        resource_id: u16,
        _message_attributes: &UAttributes,
        request_payload: Option<UPayload>,
    ) -> Result<Option<UPayload>, ServiceInvocationError> {
        let sovd_url = self.powertrain_sovd_url.clone();
        match resource_id {
            common::RESOURCE_ID_GET_CURRENT_MODE => {
                info!(
                    "Getting current powertrain mode from SOVD server at {}",
                    sovd_url
                );
                match self
                    .http_client
                    .get(sovd_url)
                    .bearer_auth(self.sovd_token.clone())
                    .send()
                    .await
                {
                    Err(e) => {
                        warn!("Error communicating with SOVD server: {}", e);
                        Err(ServiceInvocationError::Internal(
                            "failed to communicate with SOVD server".to_string(),
                        ))
                    }
                    Ok(response) => {
                        if !response.status().is_success() {
                            warn!(
                                "Received non-success status from SOVD server: {}",
                                response.status()
                            );
                            Err(ServiceInvocationError::Unavailable(
                                "SOVD server is unavailable".to_string(),
                            ))
                        } else {
                            response.json().await
                                .map_err(|e| {
                                    warn!("Error deserializing SOVD PowertrainMode response: {}", e);
                                    ServiceInvocationError::Internal("failed to deserialize SOVD response".to_string())
                                })
                                .map(|root: Root| root.data.clone())
                                .and_then(|mode_message| {
                                    serde_json::to_vec(&mode_message)
                                        .map(|d| {
                                            Some(UPayload::new(
                                                d,
                                                up_rust::UPayloadFormat::UPAYLOAD_FORMAT_JSON,
                                            ))
                                        })
                                        .map_err(|e| {
                                            warn!("Failed to serialize current mode response payload: {}", e);
                                            ServiceInvocationError::Internal("failed to serialize response".to_string())
                                        })
                                })
                        }
                    }
                }
            }
            common::RESOURCE_ID_SET_CURRENT_MODE => {
                info!(
                    "Setting current powertrain mode to SOVD server at {}",
                    sovd_url
                );
                let mode_message = serde_json::from_slice::<ModeMessage>(
                    &request_payload
                        .ok_or_else(|| {
                            ServiceInvocationError::InvalidArgument(
                                "Request has no payload".to_string(),
                            )
                        })?
                        .payload(),
                )
                .map_err(|e| {
                    info!("Failed to deserialize set mode request payload: {}", e);
                    ServiceInvocationError::InvalidArgument("Invalid payload".to_string())
                })?;
                {
                    let request_payload = Root {
                        data: mode_message.clone(),
                    };

                    if let Err(error) = self
                        .http_client
                        .put(sovd_url)
                        .bearer_auth(self.sovd_token.clone())
                        .json(&request_payload)
                        .send()
                        .await
                    {
                        if let Some(source) = error.source() {
                            warn!("Error communicating with SOVD server: {}", source);
                        }
                        if error.is_status() {
                            warn!(
                                "HTTP error communicating with SOVD server: {}",
                                error.status().unwrap()
                            )
                        }
                        warn!("{error}");
                    }

                    info!(
                        "Successfully set powertrain mode to: {:?}",
                        mode_message.mode
                    );
                }
                Ok(None)
            }
            _ => Err(ServiceInvocationError::NotFound(
                "No such resource".to_string(),
            )),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let cli = cli::Cli::parse();
    let sovd_server_uri = cli.get_sovd_server_uri().to_owned();

    let http_client = reqwest::Client::builder()
        .build()
        .expect("Error setting up http client");
    let mode_controller = Arc::new(CurrentModeController {
        powertrain_sovd_url: cli.get_sovd_powertrain_mode_resource_url()?,
        sovd_token: cli.get_sovd_access_token().to_owned(),
        http_client,
    });

    let uri_provider = cli.get_local_uri_provider()?;
    let transport = cli.get_transport().await?;
    let rpc_server = InMemoryRpcServer::new(transport, uri_provider.clone());
    rpc_server
        .register_endpoint(
            None,
            common::RESOURCE_ID_GET_CURRENT_MODE,
            mode_controller.clone(),
        )
        .await?;
    info!(
        "Registered GET_CURRENT_MODE endpoint at {}",
        uri_provider.get_resource_uri(common::RESOURCE_ID_GET_CURRENT_MODE)
    );
    rpc_server
        .register_endpoint(None, common::RESOURCE_ID_SET_CURRENT_MODE, mode_controller)
        .await?;
    info!(
        "Registered SET_CURRENT_MODE endpoint at {}",
        uri_provider.get_resource_uri(common::RESOURCE_ID_SET_CURRENT_MODE)
    );
    info!(
        "Powertrain mode control service is running [SOVD Server: {}].",
        sovd_server_uri
    );
    tokio::signal::ctrl_c().await?;
    info!("Powertrain mode control service is shutting down.");
    Ok(())
}

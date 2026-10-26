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

//! # SPIRE Security Plugin Implementation
//!
//! This module provides the SPIRE security plugin implementation.
//!
//! ## SVID-Based Authentication
//!
//! The SPIRE implementation uses SPIFFE Validated Identities (SVIDs) for stateless authentication:
//! - Supports configurable token expiration
//! - Implements Bearer token authentication for API requests
//! - Provides secure token validation with signature verification
//!
//! ## Feature-Based Security
//!
//! The SPIRE plugin supports conditional compilation features:
//! - **auth feature disabled**: Bypasses credential validation (development/testing)
//! - **auth feature enabled**: Enforces proper credential validation (production)
//!
//! ## Token Validation
//!
//! Token validation includes:
//! - Extraction of Bearer tokens from Authorization headers
//! - JWT signature validation (when auth feature is enabled)
//! - Token expiration checking
//! - User claims extraction for downstream services

use std::sync::OnceLock;

use async_trait::async_trait;
use axum::{RequestPartsExt, body::Bytes, http::StatusCode, response::Response};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use cda_interfaces::DiagServiceError;
use http::{HeaderMap, request::Parts};
use opensovd_cda_lib::AppError;
use regorus::CompiledPolicy;
use spiffe::{WorkloadApiClient, svid::jwt::JwtSvid};

use cda_plugin_security::{
    AuthApi, AuthError, AuthorizationRequestHandler, Claims as ClaimsTrait, SecurityApi,
    SecurityPlugin, SecurityPluginInitializer, SecurityPluginLoader,
};

const EXPECTED_AUDIENCE: &str = "sovd.cda";

static WORKLOAD_API_CLIENT: OnceLock<WorkloadApiClient> = OnceLock::new();
static COMPILED_AUTH_POLICY: OnceLock<CompiledPolicy> = OnceLock::new();

/// Spiffe security plugin data containing validated user claims.
///
/// This struct represents an initialized security plugin instance that contains
/// validated JWT claims. It implements both [`AuthApi`] and [`SecurityApi`] to
/// provide complete authentication and authorization capabilities.
pub struct SpiffeSecurityPluginData {
    svid: JwtSvid,
}

impl ClaimsTrait for SpiffeSecurityPluginData {
    fn sub(&self) -> &str {
        self.svid.claims().sub()
    }
}

/// A Spiffe based security plugin that provides JWT-based authentication
/// and basic authorization capabilities.
///
/// ## Features
///
/// - Bearer (JWT) token validation using a SPIFFE Workload API client
/// - SOVD-compliant error responses
///
#[derive(Default)]
pub struct SpiffeSecurityPlugin;

impl SpiffeSecurityPlugin {
    /// Initializes the Spiffe security plugin by connecting to the SPIFFE Workload API.
    ///
    /// This method must be called before using the plugin to ensure that the workload API client is properly set up.
    ///
    /// # Returns
    ///
    /// Returns an error if the initialization fails, in particular if the plugin cannot connect to
    /// the SPIFFE Workload API.
    pub async fn init(opa_config: &common::open_policy_agent::OpaConfig) -> Result<(), AppError> {
        let workload_api = WorkloadApiClient::connect_env().await.map_err(|e| {
            tracing::warn!(error = %e, "Failed to connect to SPIFFE Workload API");
            AppError::InitializationFailed(format!("failed to connect to SPIFFE Workload API: {e}"))
        })?;
        let auth_policy = opa_config.get_compiled_auth_policy().map_err(|e| {
            tracing::warn!(error = %e, "Failed to create compiled auth policy");
            AppError::InitializationFailed(format!("failed to create compiled auth policy: {e}"))
        })?;
        WORKLOAD_API_CLIENT.set(workload_api).map_err(|_| {
            AppError::InitializationFailed("failed to set workload API client".to_string())
        })?;
        COMPILED_AUTH_POLICY.set(auth_policy).map_err(|_| {
            AppError::InitializationFailed("failed to set compiled auth policy".to_string())
        })?;
        Ok(())
    }
}

impl SecurityPluginLoader for SpiffeSecurityPlugin {}

#[async_trait]
impl AuthorizationRequestHandler for SpiffeSecurityPlugin {
    async fn authorize(_headers: HeaderMap, _body_bytes: Bytes) -> Response {
        return Response::builder()
            .status(StatusCode::NOT_IMPLEMENTED)
            .body(Default::default())
            .unwrap();
    }
}

#[async_trait]
impl SecurityPluginInitializer for SpiffeSecurityPlugin {
    async fn initialize_from_request_parts(
        &self,
        parts: &mut Parts,
    ) -> Result<Box<dyn SecurityPlugin>, AuthError> {
        // Extract the token from the authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|e| {
                tracing::debug!(error = %e, "Failed to extract token");
                AuthError::NoTokenProvided
            })?;
        // Validate the token using the SPIFFE Workload API
        let workload_api = WORKLOAD_API_CLIENT.get().ok_or(AuthError::Internal)?;
        let svid = workload_api
            .validate_jwt_token(EXPECTED_AUDIENCE, bearer.token())
            .await
            .map_err(|e| {
                tracing::debug!(error = %e, "Failed to validate token");
                AuthError::InvalidToken {
                    details: "Token validation failed".to_string(),
                }
            })?;
        tracing::debug!(
            "Successfully validated token [sub: {}, aud: {}]",
            svid.claims().sub(),
            EXPECTED_AUDIENCE
        );
        Ok(Box::new(SpiffeSecurityPluginData { svid }))
    }
}

impl AuthApi for SpiffeSecurityPluginData {
    fn claims(&self) -> Box<&dyn ClaimsTrait> {
        Box::new(self)
    }
}

impl SecurityApi for SpiffeSecurityPluginData {
    fn validate_service(
        &self,
        service: &cda_database::datatypes::DiagService,
    ) -> Result<(), cda_interfaces::DiagServiceError> {
        let service_short_name = service
            .diag_comm()
            .map_or("unknown", |comm| comm.short_name().unwrap_or("unknown"));

        // We authorize the request based on a Rego rule using Open Policy Agent (OPA)
        let auth_policy = COMPILED_AUTH_POLICY.get().ok_or_else(|| {
            tracing::warn!("Failed to get compiled auth policy");
            DiagServiceError::AccessDenied("Internal error".to_string())
        })?;
        let spiffe_id = self.claims().sub().to_string();
        let allowed = regorus::Value::from_yaml_str(&format!(
            r#"
            spiffe_id: "{}"
            service_name: "{}"
            "#,
            spiffe_id, service_short_name
        ))
        .and_then(|value| auth_policy.eval_with_input(value))
        .map_err(|e| {
            tracing::debug!("Failed to authorize request using OPA: {e}");
            DiagServiceError::AccessDenied(String::from("authorization error"))
        })?;
        match allowed {
            regorus::Value::Bool(true) => {
                tracing::debug!(
                    "Authorization successful for request [subject: {}, service: {}]",
                    spiffe_id,
                    service_short_name
                );
                Ok(())
            }
            _ => {
                tracing::debug!(
                    "Authorization failed for request [subject: {}, service: {}]",
                    spiffe_id,
                    service_short_name
                );
                Err(DiagServiceError::AccessDenied(String::from(
                    "not authorized to invoke service",
                )))
            }
        }
    }
}

impl SecurityPlugin for SpiffeSecurityPluginData {
    fn as_auth_plugin(&self) -> &dyn AuthApi {
        self
    }

    fn as_security_plugin(&self) -> &dyn SecurityApi {
        self
    }
}

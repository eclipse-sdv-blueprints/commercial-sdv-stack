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

use std::path::PathBuf;

use clap::Parser;
use regorus::{CompiledPolicy, Engine};

#[derive(Parser)]
pub struct OpaConfig {
    /// The path to the Rego policy file that should be used for authorization decisions.
    #[arg(
        long,
        value_name = "PATH",
        env = "OPA_REGO_POLICY_FILE",
        default_value = "/app/config/authz.rego"
    )]
    pub policy_file: PathBuf,
    /// The path to the JSON data file that should be used for authorization decisions.
    #[arg(
        long,
        value_name = "PATH",
        env = "OPA_DATA_FILE",
        default_value = "/app/config/authorization-data.json"
    )]
    pub authorization_data_file: PathBuf,
    /// The entrypoint within the Rego policy that should be evaluated for authorization decisions.
    #[arg(
        long,
        value_name = "ENTRYPPOINT",
        env = "OPA_AUTH_RULE_ENTRYPOINT",
        default_value = "data.authz.allow"
    )]
    pub auth_rule_entrypoint: String,
}

impl OpaConfig {
    pub fn get_compiled_auth_policy(&self) -> Result<CompiledPolicy, Box<dyn std::error::Error>> {
        let mut regorus_engine = Engine::new();
        regorus_engine.add_policy_from_file(&self.policy_file)?;
        regorus_engine.add_data(regorus::Value::from_json_file(
            &self.authorization_data_file,
        )?)?;
        regorus_engine
            .compile_with_entrypoint(&self.auth_rule_entrypoint.clone().into())
            .map_err(|e| e.into())
    }
}

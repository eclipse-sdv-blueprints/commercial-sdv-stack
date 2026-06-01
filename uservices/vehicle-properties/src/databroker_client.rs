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

use kuksa_rust_sdk::kuksa::common::ClientTraitV2;
use kuksa_rust_sdk::kuksa::val::v2::KuksaClientV2;
use kuksa_rust_sdk::v2_proto::Datapoint;
use serde_json::{Map, Value};

const PATH_VIN: &str = "Vehicle.VehicleIdentification.VIN";
const PATH_BRAND: &str = "Vehicle.VehicleIdentification.Brand";
const PATH_MODEL: &str = "Vehicle.VehicleIdentification.Model";
const PATH_PRODUCTION_DATE: &str = "Vehicle.VehicleIdentification.ProductionDate";
const PATH_POWERTRAIN_SUPPORTED_MODES: &str = "Vehicle.Powertrain.SupportedModes";
const PATH_POWERTRAIN_TYPE: &str = "Vehicle.Powertrain.Type";
const PATH_POWERTRAIN_SUPPORTED_FUEL_TYPES: &str =
    "Vehicle.Powertrain.FuelSystem.SupportedFuelTypes";

pub(crate) struct DatabrokerAdapter {
    client: KuksaClientV2,
}

impl DatabrokerAdapter {
    pub async fn new(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let uri = http::Uri::try_from(url)?;
        let client = KuksaClientV2::new(uri);
        Ok(Self { client })
    }

    fn get_string_value(datapoints: &[Datapoint], idx: usize) -> Option<Value> {
        datapoints
            .get(idx)
            .and_then(|dp| dp.value.as_ref())
            .and_then(|v| v.typed_value.as_ref())
            .and_then(|value| String::try_from(value).ok())
            .map(Value::String)
    }

    fn get_string_array_value(datapoints: &[Datapoint], idx: usize) -> Option<Vec<Value>> {
        datapoints
            .get(idx)
            .and_then(|dp| dp.value.as_ref())
            .and_then(|v| v.typed_value.as_ref())
            .and_then(|value| Vec::<String>::try_from(value).ok())
            .map(|vec| vec.into_iter().map(Value::String).collect())
    }

    pub async fn get_vehicle_attributes(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        let paths = vec![
            PATH_VIN.to_string(),
            PATH_BRAND.to_string(),
            PATH_MODEL.to_string(),
            PATH_PRODUCTION_DATE.to_string(),
            PATH_POWERTRAIN_TYPE.to_string(),
            PATH_POWERTRAIN_SUPPORTED_MODES.to_string(),
            PATH_POWERTRAIN_SUPPORTED_FUEL_TYPES.to_string(),
        ];
        let values_from_databroker = self.client.get_values(paths).await?;
        let mut result = Map::new();
        if let Some(vin) = Self::get_string_value(&values_from_databroker, 0) {
            result.insert(PATH_VIN.to_string(), vin);
        }
        if let Some(brand) = Self::get_string_value(&values_from_databroker, 1) {
            result.insert(PATH_BRAND.to_string(), brand);
        }
        if let Some(model) = Self::get_string_value(&values_from_databroker, 2) {
            result.insert(PATH_MODEL.to_string(), model);
        }
        if let Some(production_date) = Self::get_string_value(&values_from_databroker, 3) {
            result.insert(PATH_PRODUCTION_DATE.to_string(), production_date);
        }
        if let Some(powertrain_type) = Self::get_string_value(&values_from_databroker, 4) {
            result.insert(PATH_POWERTRAIN_TYPE.to_string(), powertrain_type);
        }
        if let Some(supported_modes) = Self::get_string_array_value(&values_from_databroker, 5) {
            result.insert(
                PATH_POWERTRAIN_SUPPORTED_MODES.to_string(),
                Value::Array(supported_modes),
            );
        }
        if let Some(supported_fuel_types) = Self::get_string_array_value(&values_from_databroker, 6)
        {
            result.insert(
                PATH_POWERTRAIN_SUPPORTED_FUEL_TYPES.to_string(),
                Value::Array(supported_fuel_types),
            );
        }

        Ok(Value::Object(result))
    }
}

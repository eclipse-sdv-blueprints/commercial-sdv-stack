#!/bin/bash

#
# SPDX-FileCopyrightText: Copyright (c) 2026 Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0
#
# SPDX-License-Identifier: Apache-2.0
#

script_dir=${0%/*}
symphony_api_url="http://localhost:8082/v1alpha2/"

token=$(curl -s -X POST -H "Content-Type: application/json" -d '{"username":"admin","password":""}' "${symphony_api_url}users/auth" | jq -r '.accessToken')
authorization_header="Authorization: Bearer $token"

curl -sS -X POST -H "$authorization_header" -H "Content-Type: application/json" --data @"${script_dir}/target.json" "${symphony_api_url}targets/registry/ecu-updater-target"

# Prompt user to press Enter to continue after the target has been registered
read -r -p "Target registered. Press Enter to remove..."

curl -sS -X DELETE -H "$authorization_header" "${symphony_api_url}targets/registry/ecu-updater-target"

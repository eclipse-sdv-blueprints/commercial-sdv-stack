#!/bin/bash

# SPDX-FileCopyrightText: 2026 Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
# 
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# SPDX-License-Identifier: Apache-2.0

deps_file=${DEPS_FILE:-"DEPS.txt"}
dash_url="https://repo.eclipse.org/repository/dash-maven2-releases/org/eclipse/dash/org.eclipse.dash.licenses/1.1.0/org.eclipse.dash.licenses-1.1.0.jar"
dash_jar=${DASH_JAR:-"/tmp/dash.jar"}
dash_summary=${DASH_SUMMARY:-"DASH_SUMMARY.txt"}
project=${PROJECT:-"automotive.sdv-blueprints"}
token=$1

echo "Creating list of 3rd party crates we depend on..."
cargo tree --manifest-path uservices/Cargo.toml -e no-build,no-dev --prefix none --no-dedupe --locked \
  | sed -n '2~1p' \
  | sort -u \
  | grep -v '^[[:space:]]*$' \
  | grep -v common \
  | grep -v fms \
  | grep -v powertrain \
  | grep -v vehicle-properties \
  | grep -v up- \
  | sed -E 's|([^ ]+) v([^ ]+).*|crate/cratesio/-/\1/\2|' \
  > "$deps_file"

echo "Creating list of 3rd party Java libraries we depend on..."
cat ecu-sim/gradle.lockfile \
  | grep -Pv '^#' \
  | grep -Pv '^empty' \
  | grep -Poh '^[^=]+' \
  >> "$deps_file"

if [[ ! -r "$dash_jar" ]]; then
  echo "Eclipse Dash JAR file [${dash_jar}] not found, downloading latest version from GitHub..."
  wget_bin=$(which wget)
  if [[ -z "$wget_bin" ]]; then
    echo "wget command not available on path"
    exit 127
  else
    wget --quiet -O "$dash_jar" "$dash_url"
  echo "successfully downloaded Eclipse Dash JAR to ${dash_jar}"
  fi
fi

args=(-jar "$dash_jar" -timeout 60 -batch 90 -summary "$dash_summary")
if [[ -n "$token" ]]; then
  args=("${args[@]}" -review -token "$token" -project "$project")
fi
args=("${args[@]}" "$deps_file")

echo "checking 3rd party licenses..."
java "${args[@]}"

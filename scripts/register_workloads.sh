#!/bin/bash

#*******************************************************************************
# Copyright (c) 2026 Contributors to the Eclipse Foundation
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Eclipse Public License 2.0 which is available at
# http://www.eclipse.org/legal/epl-2.0
#
# SPDX-License-Identifier: EPL-2.0
#*******************************************************************************

#  Registers the workloads with the SPIRE server

# Backend workloads

docker compose exec -T spire-server \
  /opt/spire/bin/spire-server entry create \
    -socketPath /run/spire/server/private/api.sock \
    -parentID spiffe://sdv.eclipse.org/spire/agent/x509pop/spire-agent-backend \
    -spiffeID spiffe://sdv.eclipse.org/backend/fms \
    -selector docker:image_id:ghcr.io/eclipse-sdv-blueprints/commercial-sdv-stack/fms:latest \
    -selector docker:env:UP_LOCAL_ADDRESS=up://backend/103AA/1/0

# In-vehicle workloads

docker compose exec -T spire-server \
  /opt/spire/bin/spire-server entry create \
    -socketPath /run/spire/server/private/api.sock \
    -parentID spiffe://sdv.eclipse.org/spire/agent/x509pop/spire-agent-vehicle \
    -spiffeID spiffe://sdv.eclipse.org/vehicle/properties \
    -selector docker:image_id:ghcr.io/eclipse-sdv-blueprints/commercial-sdv-stack/vehicle-properties:latest \
    -selector docker:env:UP_LOCAL_ADDRESS=up://vehicle/10302/1/0

docker compose exec -T spire-server \
  /opt/spire/bin/spire-server entry create \
    -socketPath /run/spire/server/private/api.sock \
    -parentID spiffe://sdv.eclipse.org/spire/agent/x509pop/spire-agent-vehicle \
    -spiffeID spiffe://sdv.eclipse.org/vehicle/powertrain-mode-controller \
    -selector docker:image_id:ghcr.io/eclipse-sdv-blueprints/commercial-sdv-stack/powertrain-mode-controller:latest \
    -selector docker:env:UP_LOCAL_ADDRESS=up://vehicle/10301/1/0

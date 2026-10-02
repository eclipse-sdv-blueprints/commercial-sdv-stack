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

#  Shows the workloads registered with the SPIRE server

docker compose exec -T spire-server \
  /opt/spire/bin/spire-server entry show \
    -socketPath /run/spire/server/private/api.sock

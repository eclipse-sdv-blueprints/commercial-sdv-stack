#!/bin/sh -e

# SPDX-License-Identifier: Apache-2.0
# SPDX-FileCopyrightText: 2025 The Contributors to Eclipse OpenSOVD (see CONTRIBUTORS)
#
# See the NOTICE file(s) distributed with this work for additional
# information regarding copyright ownership.
#
# This program and the accompanying materials are made available under the
# terms of the Apache License Version 2.0 which is available at
# https://www.apache.org/licenses/LICENSE-2.0


DOIP_TESTER_IP=$(ip -4 a show scope global | grep -oP '(?<=inet\s)\d+(\.\d+){3}')

echo "Using arguments:" "$@"
echo "Using DoIpTesterIp: $DOIP_TESTER_IP"

find "." -maxdepth 1 -type f -print0 | xargs -0 sha1sum

# AI-generated (GitHub Copilot, Claude Opus 5.5) - issue 7: begin
# no access for others to the SOVD Unix socket created by CDA
umask 0007

exec /app/opensovd-cda --tester-address "$DOIP_TESTER_IP" "$@"
# AI-generated - issue 7: end

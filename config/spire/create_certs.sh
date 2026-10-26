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

# A simple shell script for generating example certificates to be used with SPIRE installations.

certsDir=certs

commonSubject="/C=BE/L=Brussels/O=Eclipse SDV/OU=SPIRE"

# set to either EC or RSA
keyAlg=EC

# positional parameters:
# $1 - name of the file to write the key to
function create_key { 

  echo ""
  outFile="$certsDir/$1"
  if [ $keyAlg == "EC" ]
  then
    openssl ecparam -name secp384r1 -genkey -noout | openssl pkcs8 -topk8 -nocrypt -inform PEM -outform PEM -out "$outFile"
  else
    openssl genrsa 4096 | openssl pkcs8 -topk8 -nocrypt -inform PEM -outform PEM -out "$outFile"
  fi
}

#
# positional parameters:
# $1 - name of key/cert
function create_cert {

  echo ""
  echo "creating $1 key and certificate"
  if ! create_key "$1-key.pem"; then
    echo "failed to create key for $1"
    exit 1
  fi
  openssl req -config ca_opts -new -key "$certsDir/$1-key.pem" -subj "$commonSubject/CN=$1" | \
    openssl x509 -req -extfile ca_opts -extensions "req_ext_$1" -out "$certsDir/$1.pem" -days 365 -CA "$certsDir/ca-cert.pem" -CAkey "$certsDir/ca-key.pem" -CAcreateserial
  cat "$certsDir/$1.pem" "$certsDir/ca-cert.pem" > "$certsDir/$1-cert.pem" && rm "$certsDir/$1.pem"
}

if [ -d $certsDir ]
then
rm "$certsDir/*.pem"
rm "$certsDir/*.p12"
else
mkdir -p "$certsDir"
fi

echo "creating root key and certificate"
create_key root-key.pem
openssl req -x509 -config ca_opts -new -key "$certsDir/root-key.pem" -out "$certsDir/root-cert.pem" -days 365 -subj "$commonSubject/CN=root"

echo ""
echo "creating CA key and certificate"
create_key ca-key.pem
openssl req -config ca_opts -reqexts intermediate_ext -new -key "$certsDir/ca-key.pem" -days 365 -subj "$commonSubject/CN=ca" | \
 openssl x509 -req -extfile ca_opts -extensions intermediate_ext -out "$certsDir/ca-cert.pem" -days 365 -CA "$certsDir/root-cert.pem" -CAkey "$certsDir/root-key.pem" -CAcreateserial

echo ""
echo "creating PEM trust store ($certsDir/trusted-certs.pem) containing CA certificate"
#cat $certsDir/ca-cert.pem $certsDir/root-cert.pem > $certsDir/trusted-certs.pem
cat "$certsDir/ca-cert.pem" > "$certsDir/trusted-certs.pem"

for name in spire-server spire-agent-backend spire-agent-vehicle
do
  if ! create_cert "$name"; then
    echo "failed to create certificate for $name"
    exit 1
  fi
done

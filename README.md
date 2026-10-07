<!-- Portions of this file were generated with AI assistance (Github Copilot, Claude Opus 5.5). -->

# Commercial Vehicle Use Cases based on Eclipse SDV Software Components

This repository contains artifacts that implement a few use cases that are (not exclusively) relevant for commercial vehicles.
The artifacts have been implemented using open source components created by the Eclipse SDV community and other projects.

Use cases currently include:
* Over-the-Air (OTA) firmware updates of ECUs
* OTA configuration changes of ECUs

## Top Level View

The following diagram provides an overview of the components used for the example use cases and their relationship to each other:

![Top-Level View](./Top%20Level%20View.drawio.svg)

## Getting Started

The repository contains a [Docker Compose file](docker-compose.yaml) which can be used to start up all
components implementing the example use cases. To get started, run the following command

```bash
# Using the default Docker Compose file in the top level folder:
docker compose --profile infra up -d
# Or if your docker-compose.yaml file is located in a different folder:
# docker compose -f /path/to/your/docker-compose.yml up -d
```

This will start up some common infrastructure components, which are shared by all needed for all cases.
You can now open _Dozzle_ in your browser at http://localhost:8080 and see the running infrastructure containers and inspect their log output.

## Configure Authorization

Some of the service components that implement the use cases in this blueprint use JSON Web Tokens (JWT) to authenticate and authorize clients. The tokens are issued by [Spire](https://spiffe.io) agents running in the backend and on the vehicle. These agents are connected to a Spire server in the backend which provides the key material need for creating and verifying the tokens. The workload components then use the agent's _SPIFFE Workload API_ to create and/or validate tokens. For this to work, the workloads need to be registered with the Spire server by running the following script:

```bash
scripts/register_workloads.sh
```

After successful workload registration, the use cases can be run as described in the following sections.

### Protecting the SPIFFE Workload API

Workloads trust that the Unix domain socket `/tmp/spire-agent/public/api.sock` leads to the genuine local SPIRE agent.
To keep that assumption valid, the Docker Compose files enforce the following:

- Only the SPIRE agents mount their socket volume (`spire-agent-backend-socket`, `spire-agent-vehicle-socket`) read-write, so only they can create, replace or remove the socket.
- Workloads mount the socket volume read-only and with `nocopy`, so they can connect to the socket but cannot replace it or seed the volume with content from their image.
- Workloads run as dedicated non-root users, drop all Linux capabilities and cannot gain new privileges.
- Only trusted infrastructure (the SPIRE agents for workload attestation and Dozzle) mounts the Docker socket. Note that mounting `docker.sock` read-only does not make the Docker API read-only.

These rules can be checked without starting any containers by running:

```bash
scripts/check_socket_hardening.sh
```

Once the stack is running, the following command additionally verifies the live containers: each workload's socket directory refuses writes, and each workload can still reach its SPIRE agent.
The optional `--agent-restart` flag also restarts each SPIRE agent and verifies that the workloads reach the recreated socket without being restarted themselves:

```bash
scripts/check_socket_hardening.sh --runtime [--agent-restart]
```

## Run the Deploy Firmware Use Case

Start the required components and services by running:

```bash
# Using the default Docker Compose file in the top level folder:
docker compose --profile infra --profile fw-update up -d
```

In the Dozzle console you should now see a few additional containers running.

### Verify Access to Services

* To check access to Symphony's REST API, you can send a test request by opening http://localhost:8082/v1alpha2/greetings in your browser.
  The response should be: `Hello from Symphony K8s control plane (S8C)`
* To check access to the read-only Symphony Portal, point your browser to http://localhost:3000.
  Login with user `admin` without a password. You should see the Symphony portal page.
  During your experiments, the portal can used to determine the progress of updates by means of checking the _Targets_.

### Trigger Firmware Update

Now that the infrastructure is up and running, it is time to trigger the update of some ECU firmware images using the Symphony API:

```bash
# From the repository root folder
scripts/trigger_ecu_update.sh
```

This will create a new _Target_ in Symphony and deploy two firmware files to it:

* The logs of the `ecu-updater` container should contain something similar to

  ```log
  [2026-06-01T14:26:29Z INFO  ecu_updater::ecu_target] installing firmware [name: Engine Controller, FW Image: "https://acme.io/fw/engine-control-1.45.img"]
  [2026-06-01T14:26:29Z INFO  ecu_updater::ecu_target] installing firmware [name: Telematics Unit, FW Image: "https://non-existent.io/fw/telematics-unit-2.0.img"]
  ```

* A corresponding entry should show up under the _Targets_ tab in the Symphony Portal web UI.

After hitting _Enter_ in the terminal where you ran the shell script:

* The firmware files will be removed from the target.
  In the logs of the `ecu-updater` container, you should see something similar to:

  ```log
  [2026-06-01T14:26:53Z INFO  ecu_updater::ecu_target] removing firmware [Engine Controller]
  [2026-06-01T14:26:53Z INFO  ecu_updater::ecu_target] removing firmware [Telematics Unit]
  ```
* The _ecu-updater-target_ in Symphony will be removed after a few seconds.

The sequence diagram below shows the flow of messages through the system for triggering the firmware update:

```mermaid
sequenceDiagram
  autonumber
  box Back End
  actor C as Client
  participant SAPI as Symphony API : ApiServer
  end
  box Vehicle
  participant SA as Symphony Agent : ApiServer
  participant EU as <<uService>><br/>ECU Updater<br/>: eclipse.symphony.targetprovider
  end

  C->>SAPI: Create Target("ecu-updater-target", target spec)
  activate SAPI
  SAPI-)SA: Create Target("ecu-updater-target", target spec)
  deactivate SAPI

  activate SA
  SA->>EU: Get() : current deployment state
  activate EU
  deactivate EU

  SA->>EU: Update(target spec) : update result
  activate EU
  deactivate EU

  deactivate SA

  C->>SAPI: Delete Target("ecu-updater-target")
  activate SAPI
  SAPI-)SA: Delete Target("ecu-updater-target")
  deactivate SAPI

  activate SA
  SA->>EU: Get() : current deployment state
  activate EU
  deactivate EU

  SA->>EU: Delete(["ecu-update-target"]) : deletion result
  activate EU
  deactivate EU

  deactivate SA
```

1. The _Client_ uploads a new target deployment specification by means of an HTTP POST request to the _Symphony API_ server in the back end. The target specification contains the details of the firmware images to be updated.
2. The _Symphony API_ server forwards the target spec to the _Symphony Agent_ running on the vehicle by means of an MQTT message.
3. The _Symphony Agent_ determines the current deployment status of the _ECU Updater_ deployment target by means of invoking its _Get_ operation via uProtocol.
4. The _Symphony Agent_ triggers the installation of the firmware by means of invoking the _Update_ operation with the changed components of the target specification via uProtocol.

**Note** The ECU Updater in this example use case does not actually deploy any firmware images to any ECU but only maintains some state in memory. In a future extension of the blueprint, the OpenSOVD CDA server might be used to actually perform an ECU update via UDS.

## Building Behind a TLS-Intercepting Proxy

The local Rust and ECU simulator Dockerfiles accept an optional BuildKit secret
named `proxy_ca`. Export your organization's trusted root CA as a PEM certificate
to `config/proxy/ca-cert.crt`, then create `docker-compose.override.yaml`:

```yaml
services:
  fms:
    build: &proxy_build
      secrets: [proxy_ca]
  vehicle-properties:
    build: *proxy_build
  powertrain-mode-controller:
    build: *proxy_build
  sovd-cda:
    build: *proxy_build
  ecu-sim:
    build: *proxy_build
secrets:
  proxy_ca:
    file: ./config/proxy/ca-cert.crt
```

Compose loads this override automatically. Both local files are ignored by Git.
The CA is installed in the containers' OS trust stores and, for Java, the JVM
trust stores; TLS verification remains enabled. Builds without the secret retain
their default trust stores. After rotating the certificate, rebuild with
`docker compose --profile infra --profile powertrain build --no-cache` because
BuildKit does not invalidate its cache when secret contents change.

This configures container trust, not Docker daemon trust for image pulls.

## Run the Set Powertrain Mode Use Case

In this use case, a Fleet Management System in the backend uses the _Powertrain Mode Controller_ uService on the vehicle to cycle the vehicle's powertrain through all supported modes.

For this purpose, the Powertrain Mode Controller invokes the Powertrain ECU's _PowerTrain_Mode_Write_ UDS operation by means of the [Eclipse OpenSOVD's Classic Diagnostic Adapter](https://github.com/eclipse-opensovd/classic-diagnostic-adapter) (CDA) component.
The CDA exposes an HTTP based API that can be used by clients to interact with ECUs via UDS over DoIP.

The Powertrain ECU is represented by a modified version of the ECU Simulator component that is part of OpenSOVD's test infrastructure. In particular, it has been modified to expose _PowerTrain_Mode_Read_ and _PowerTrain_Mode_Write_ operations.

Start the required components and services by running:

```bash
# Using the default Docker Compose file in the top level folder:
docker compose --profile infra --profile powertrain up -d
```

**Note** The `fms`, `vehicle-properties`, `powertrain-mode-controller`, `sovd-cda` and `ecu-sim` images are always built from the local sources (`pull_policy: build`), so `up` rebuilds them whenever their sources have changed and takes unchanged images from the build cache. After pulling new changes, run the command above again to rebuild the images and recreate the affected containers.

The setting of the powertrain mode can be traced through the system by means of the container logs, which you can examine in the Dozzle console.

* The log file of the Fleet Management System contains entries like these:
  ```log
  [2026-06-02T09:12:14Z INFO  fms] setting powertrain mode to Economy
  ...
  [2026-06-02T09:12:19Z INFO  fms] setting powertrain mode to Performance
  ```
* The log file of the Powertrain Mode Controller contains entries like these:
  ```log
  [2026-06-02T09:13:25Z INFO  powertrain] Setting current powertrain mode to SOVD server at http://sovd-cda:20002/vehicle/v15/components/blueprint-ecu/data/powertrain_mode
  [2026-06-02T09:13:25Z INFO  powertrain] Powertrain mode set to: Economy
  ```
* The log file of the ECU Simulator contains entries like these:
  ```log
  Set PowerTrain Mode: 2
  09:13:25.536 DEBUG [DefaultDispatcher-worker-10] SimEcu blueprint-ecu Request for blueprint-ecu: '2E 4E 66 02' matched '{ PowerTrain_Mode_Write; Bytes: 2E 4E 66 [] }' -> Send response '6E 4E 66'
  ```

The FMS, Powertrain Mode Controller and CDA components also log information about the JWTs that are created and validated at _DEBUG_ level.

The sequence diagram below shows the flow of messages for setting the current mode on the powertrain ECU:

```mermaid
sequenceDiagram
  autonumber
  box Back End
  participant FMS as <<uEntity>><br/>Fleet Management<br/>System
  end
  box Vehicle
  participant PMC as <<uService>><br/>Powertrain Mode Controller<br/>: powertrain.mode-control
  participant CDA as CDA Server<br/>: SOVD
  participant ECU as Blueprint ECU : ECU Simulator
  end

  FMS->>PMC: SetCurrentMode(Economy)
  activate PMC
  PMC->>CDA: WriteDataValue(<br/>entity=/components/blueprint-ecu/data/powertrain_mode,<br/>{mode: Economy})
  activate CDA
  CDA->>ECU: PowerTrain_Mode_Write(Economy)
  deactivate CDA
  deactivate PMC
```

1. The _Fleet Managament System_ sets the powertrain mode to _Economy_ by means of a uProtocol RPC call to the _Powertrain Mode Controller_.
<!-- AI-modified (GitHub Copilot, Claude Opus 5.5) - issue 7: begin -->
2. The _Powertrain Mode Controller_ sets the powertrain mode to _Economy_ by acquiring a short-lived lock on the ECU and updating the corresponding SOVD entity's data value by means of an HTTP PUT request on the _CDA Server_ (via the CDA's Unix domain socket). The lock is released again afterwards.
<!-- AI-modified - issue 7: end -->
3. The _CDA Server_ sets the powertrain mode to _Economy_ by means of invoking the _Powertrain_Mode_Write_ operation on the _Blueprint ECU_ via UDS.

### Authorization Integration Tests

A standalone Cargo crate in [`tests/`](tests/) verifies the stack's authorization model end to end: valid attested workloads succeed, while unauthorized identities, forbidden operations, wrong audiences, and expired or forged credentials all fail closed.

With the stack running and the workloads registered:

```bash
# From the repository root folder
docker compose --profile infra --profile powertrain up -d --build
scripts/register_workloads.sh

cd tests
cargo test            # runs the whole matrix (use ./run.sh if you have no local Rust toolchain)
```

See [tests/README.md](tests/README.md) for the full scenario matrix and characterization notes.

### What's in the Box?

The Docker Compose file starts up the following components:

- The `ecu-sim` service is a basic DoIP-connected ECU simulation built on the [doip-sim-ecu framework](https://github.com/doip-sim-ecu). It acts as the southbound conterpart for the classic-diagnostic-adapters diagnostic communication.
- The `sovd-cda` service is in essence an SOVD-to-UDS bridge. It uses ODD/PDX diagnostic definitions (pre-processed by OpenSOVD [odx-converter](https://github.com/eclipse-opensovd/odx-converter)) to populate its SOVD API, mapping those resources to corresponding UDS communication with connected ECUs (via DoIP).
- The `powertrain-mode-controller` service is a uProtocol entity that exposes a _setCurrentMode_ operation that can be used by other uProtocol entities to set the vehicle's powertrain mode.
- The `fms` service is a uProtocol entity that represents a Fleet Management System running in the back end. The service periodically invokes the _setCurrentMode_ operation to toggle the powertrain's mode of operation between _Economy_ and _Performance_.

The repository also contains the [odx](odx/) directory, which is a copy of the OpenSOVD Classic Diagnostic Adapter testcontainer, containing Python scripts for generating example ODX data using the [odxtools](https://github.com/mercedes-benz/odxtools) library.

### Modifications

This project copy of the upstream CDA testcontainer is modified to better represent the uServices PoC use cases, by:

- creating appropriately named ODX service definitions (by modifying the odx generation scripts)
- implementing the ecu-sim counterpart to these services (by modifying the ecu-simulation code)

<!-- AI-modified (GitHub Copilot, Claude Opus 5.5) - issue 7: begin -->
### Service APIs

- SOVD API: only available via the Unix domain socket `/run/cda/cda.sock` in the `commercial-sdv-stack_cda-sovd-socket` volume (base URI `http://localhost/vehicle/v15`, see [Securing Access to the CDA](#securing-access-to-the-cda))
- ECU Simulator Control API: http://localhost:8181

#### Example Requests

Requests to the SOVD API need to be sent from a container that mounts the socket volume and runs with group `10100` (`sovd-clients`).
`ACCESS_TOKEN` must be a JWT-SVID with audience `sovd.cda` for a SPIFFE ID that is authorized in [authorization-data.json](config/cda/config/authorization-data.json).
Writing data or changing modes requires an ECU lock held by the same SPIFFE ID.

```sh
sovd_curl() {
  docker run --rm --user 10003:10100 \
    -v commercial-sdv-stack_cda-sovd-socket:/run/cda:ro \
    curlimages/curl -s --unix-socket /run/cda/cda.sock \
    -H "Authorization: Bearer $ACCESS_TOKEN" "$@"
}

# retrieve standardized resource collection for ECU (+ variant)
sovd_curl -X GET "http://localhost/vehicle/v15/components/blueprint-ecu"

# force variant detection
sovd_curl -X PUT "http://localhost/vehicle/v15/components/blueprint-ecu"

# acquire component lock
sovd_curl -X POST -H "Content-Type: application/json" --data '{"lock_expiration": 100000}' "http://localhost/vehicle/v15/components/blueprint-ecu/locks"

# switch into extended session
sovd_curl -X PUT -H "Content-Type: application/json" --data '{"value": "extended"}' "http://localhost/vehicle/v15/components/blueprint-ecu/modes/session"

# switch sim to boot variant
curl -s -X PUT -H "Content-Type: application/json" --data '{"variant": "BOOT"}' "http://localhost:8181/blueprint-ecu/state"

```

### Securing Access to the CDA

The CDA does not listen on any TCP port. Its SOVD API is only exposed via a Unix domain socket, which is protected by several layers:

- The socket lives in the dedicated `cda-sovd-socket` volume, which is only mounted into `sovd-cda` (read-write) and `powertrain-mode-controller` (read-only).
- The CDA runs as non-root user `10001` and creates the socket in a `0750` directory with mode `0770`, so only members of group `10100` can connect.
- Both containers drop all capabilities and run with `no-new-privileges`, and the Powertrain Mode Controller is not attached to the `vehicle-sovd` network.
- Every request still needs a valid JWT-SVID (audience `sovd.cda`), and access to diagnostic services is authorized by the CDA's Rego policy.

If you have run an earlier version of the stack, recreate the volumes once so that the new socket volume gets the correct ownership: `docker compose --profile infra --profile powertrain down -v`.

A Unix domain socket is a good fit as long as client and CDA share the same kernel. Mutual TLS (using X.509-SVIDs from the same SPIRE deployment) is the better choice when:

- client and CDA run on different hosts, VMs, ECUs or Kubernetes nodes,
- traffic crosses an untrusted or shared network (e.g. in-vehicle Ethernet backbone, remote diagnostics),
- stolen bearer tokens must not be usable (mTLS binds the identity to a private key),
- encryption and mutual authentication in transit are required (e.g. ISO/SAE 21434, UNECE R155).

**Known limitation:** the CDA's lock endpoints are not subject to the Rego authorization, so any workload with access to the socket and a valid JWT-SVID can acquire an ECU lock.
<!-- AI-modified - issue 7: end -->

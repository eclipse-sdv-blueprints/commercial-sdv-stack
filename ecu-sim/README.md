# CDA ECU SIM

This is a modification of the [Eclipse OpenSOVD](https://github.com/eclipse-opensovd) [classic-diagnostic-adapter](https://github.com/eclipse-opensovd/classic-diagnostic-adapter)(CDA) [testcontainer](https://github.com/eclipse-opensovd/classic-diagnostic-adapter/tree/main/testcontainer) ECU simulation setup, specifically emulating Drivetrain ECU diagnostics for use with the [powertrain-mode-controller uService](../uservices/powertrain/).

The primarily involved/modified code for this is in `./src/main/kotlin/ecu/DiagnosticFunctionalities.kt`, with some subsequent modifications to related code.

## Features

- Simulates a DoIP/UDS topology of ECUs for testing with the CDA
- Mock for token creation
  - Used to create and verify tokens in the default security plugin
- Offers endpoints to:
  - Retrieve and modify the ECU state
  - Retrieve data transfer data
  - Record incoming UDS messages

  This allows integration tests to verify that the request to the CDA are translated correctly for the target ECU.

## Usage in this Blueprint

The Docker Compose file in the repository root runs the simulator as the `ecu-sim` service (profile `powertrain`). It simulates the `blueprint-ecu`, which implements the `PowerTrain_Mode_Read` and `PowerTrain_Mode_Write` operations defined by the [ODX data](../odx/).

- The simulator is reachable by the CDA via DoIP on the `vehicle-sovd` network only.
- The control API is published on host port `8181` by default. Set the `SIM_CONTROL_PORT` environment variable (e.g. `127.0.0.1:8181`) to change the port or to bind it to the loopback interface only.
- The control API can be used to switch the simulated ECU to another variant, e.g. `curl -X PUT -H "Content-Type: application/json" --data '{"variant": "BOOT"}' http://localhost:8181/blueprint-ecu/state`. Other variants may expose different diagnostic resources, so variant detection needs to be triggered again on the CDA afterwards. The powertrain mode use case is not expected to work unchanged with another variant.

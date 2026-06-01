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


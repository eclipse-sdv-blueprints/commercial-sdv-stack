# Example uServices

This folder contains uProtocol service definitions (uServices) following the [Async API specification](https://www.asyncapi.com/en)
and corresponding implementations based on the [uProtocol Rust Language Library](https://github.com/eclipse-uprotocol/up-rust).

## uService Definintions

| Service Type ID | Service Name                    | AsyncAPI definition |
| :-------------- | :------------------------------ | :------------------ |
| `0x0301`        | powertrain.mode-control         | [Powertrain-asyncapi.yaml](powertrain//Powertrain-asyncapi.yaml) |
| `0x0302`        | vehicle.properties              | [VehicleProperties-asyncapi.yaml](vehicle-properties//VehicleProperties-asyncapi.yaml) |

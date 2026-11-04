# Example uServices

This folder contains uProtocol service definitions (uServices) following the [Async API specification](https://www.asyncapi.com/en)
and corresponding implementations based on the [uProtocol Rust Language Library](https://github.com/eclipse-uprotocol/up-rust).

## uService Definintions

| Service Type ID | Service Name                    | AsyncAPI definition |
| :-------------- | :------------------------------ | :------------------ |
| `0x0301`        | powertrain.mode-control         | [Powertrain-asyncapi.yaml](powertrain//Powertrain-asyncapi.yaml) |
| `0x0302`        | vehicle.properties              | [VehicleProperties-asyncapi.yaml](vehicle-properties//VehicleProperties-asyncapi.yaml) |

## Building

The container images for the Fleet Management System (`fms`), the Powertrain Mode Controller, the Vehicle Properties service and the OpenSOVD CDA (`sovd-cda`, built from [cda](cda/)) are built from this folder. Pulling the published images is usually sufficient. To build the images locally instead, run the following from the repository root folder:

```bash
docker compose --profile powertrain build fms powertrain-mode-controller vehicle-properties sovd-cda
docker compose --profile infra --profile powertrain up -d
```

Building requires access to the dependency registries and takes more time and disk space than pulling images. Some other images are not built from this repository. For example, the `ecu-updater` image is built from a remote Git repository.

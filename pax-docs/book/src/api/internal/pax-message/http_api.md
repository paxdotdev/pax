# http_api
<!-- summary: Shared request and response types for Pax's public HTTP service. -->
<!-- tags: api, pax-message -->

Shared request and response types for Pax's public HTTP service.

This module deliberately contains only the closed wire contract. HTTP clients,
provider mappings, persistence, and policy belong at the respective edges.

## Structs
### `LatestReleaseResponse`
Response returned by [`CLI_LATEST_RELEASE_PATH`].

#### Properties
##### `latest_version`
Type: `String`

---

### `TelemetryRequest`
One privacy-bounded CLI telemetry event.

#### Properties
##### `installation_id`
Type: `String`

Random UUID v4 representing one OS-user CLI installation.

##### `cli_version`
Type: `String`

##### `host_os`
Type: [`HostOs`](../../../api/internal/pax-message/http_api.md#hostos)

##### `host_arch`
Type: [`HostArch`](../../../api/internal/pax-message/http_api.md#hostarch)

##### `event`
Type: [`TelemetryEvent`](../../../api/internal/pax-message/http_api.md#telemetryevent)

## Enums
### `CommandFamily`
Public top-level CLI command families eligible for telemetry.

#### Variants
##### `Create`
##### `Run`
##### `Build`
##### `Clean`
##### `Eject`
##### `Format`
##### `Lsp`
##### `Docs`
##### `Dev`
##### `SvgImport`
---

### `CommandOutcome`
Coarse command result. Error details never cross the HTTP boundary.

#### Variants
##### `Succeeded`
##### `Failed`
---

### `HostArch`
Coarse host CPU architecture.

#### Variants
##### `X86_64`
##### `Aarch64`
##### `Other`
---

### `HostOs`
Coarse host operating-system family.

#### Variants
##### `Macos`
##### `Linux`
##### `Windows`
##### `Other`
---

### `Target`
Supported Pax build or run targets.

#### Variants
##### `Web`
##### `Macos`
##### `Ios`
##### `Ipados`
---

### `TelemetryEvent`
Closed set of telemetry events accepted by the launch API.

#### Variants
##### `CommandOutcome` { `command`: [`CommandFamily`](../../../api/internal/pax-message/http_api.md#commandfamily), `target`: `Option`<[`Target`](../../../api/pax-std/core/link.md#target)>, `outcome`: [`CommandOutcome`](../../../api/internal/pax-message/http_api.md#commandoutcome) }
##### `RunReady` { `target`: [`Target`](../../../api/pax-std/core/link.md#target) }
## Constants
### `CLI_LATEST_RELEASE_PATH`
Path for querying the latest published `pax-cli` release.

---

### `CLI_TELEMETRY_PATH`
Path for submitting a single CLI telemetry event.

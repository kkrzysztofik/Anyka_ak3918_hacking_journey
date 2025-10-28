# ONVIF Device Service Contract Updates

## Supported Operations

| Operation | Request Requirements | Response Data | Fault Conditions |
|-----------|----------------------|---------------|------------------|
| `GetDeviceInformation` | None | Manufacturer, model, firmware, serial, hardware ID sourced from unified configuration | `ONVIF_ERROR` if configuration unavailable |
| `GetServiceCapabilities` | Optional `IncludeCapability` flag | Device capability set reflecting zero relay outputs and unsupported certificate lifecycle | None |
| `GetDiscoveryMode` | None | Current discovery mode from configuration | `ONVIF_ERROR_NOT_SUPPORTED` if mode cannot be read |
| `SetDiscoveryMode` | Mode enum (Discoverable/NonDiscoverable) | Ack, persisted within 2s | `ONVIF_ERROR_INVALID_PARAMETER` on invalid mode |
| `GetHostname` | None | Hostname plus DHCP flag | `ONVIF_ERROR` if runtime/config missing |
| `SetHostname` | Hostname string | Ack + persistence | `ONVIF_ERROR_INVALID_PARAMETER` (length/format) |
| `SetHostnameFromDHCP` | Bool | Ack + persistence | `ONVIF_ERROR` if DHCP toggle invalid |
| `GetNetworkInterfaces` | None | Interfaces with MAC, MTU, IPv4 config | `ONVIF_ERROR` for retrieval failure |
| `SetNetworkInterfaces` | Interface token, IPv4 config, DHCP flag | Ack + persistence | `ONVIF_ERROR_NOT_SUPPORTED` for unsupported fields (IPv6), `ONVIF_ERROR_INVALID_PARAMETER` for bad config |
| `GetNetworkProtocols` | None | Enabled protocol list (HTTP/RTSP) with ports | `ONVIF_ERROR` on failure |
| `SetNetworkProtocols` | Protocol definitions | Ack + persistence | `ONVIF_ERROR_INVALID_PARAMETER` (ports out of range) |
| `GetNetworkDefaultGateway` | None | Current IPv4 gateway | `ONVIF_ERROR` if unavailable |
| `SetNetworkDefaultGateway` | IPv4 address | Ack + persistence | `ONVIF_ERROR_INVALID_PARAMETER` on invalid IP |
| `GetDNS` | None | DHCP/manual DNS addresses | `ONVIF_ERROR` on failure |
| `SetDNS` / `SetDynamicDNS` | DNS entries or mode | Ack + persistence | `ONVIF_ERROR_INVALID_PARAMETER` invalid entries |
| `GetNTP` | None | Manual NTP list or default fallback | `ONVIF_ERROR` on retrieval failure |
| `SetNTP` | NTP manual list | Ack + persistence | `ONVIF_ERROR_INVALID_PARAMETER` invalid host |
| `GetScopes` | None | Scope URIs derived from config/runtime (hostname, hardware, profiles) | `ONVIF_ERROR` on failure |
| `SetScopes` / `AddScopes` / `RemoveScopes` | Scope URIs | Ack + persistence | `ONVIF_ERROR_INVALID_PARAMETER` invalid URIs |
| `GetUsers` | None | Active users with roles | `ONVIF_ERROR` if config unavailable |
| `CreateUsers` / `SetUser` / `DeleteUsers` | User list with credentials | Ack + persistence | `ONVIF_ERROR_INVALID_PARAMETER` (username/pw policy), `ONVIF_ERROR_TOO_MANY_USERS` |
| `GetRelayOutputs` | None | Empty list with zero relay outputs | None |
| `SetRelayOutputSettings` / `SetRelayOutputState` | Any | `ONVIF_ERROR_NOT_SUPPORTED` | Always |
| `GetCertificates` / `GetCertificatesStatus` | None | `ONVIF_ERROR_NOT_SUPPORTED` | Always |
| `CreateCertificate` / `LoadCertificate` / `DeleteCertificates` / `SetCertificatesStatus` | Any | `ONVIF_ERROR_NOT_SUPPORTED` | Always |
| `SetDeviceInformation` | Manufacturer, model, etc. | Ack + persistence | `ONVIF_ERROR_INVALID_PARAMETER` invalid strings |

## Response & Persistence Guarantees
- All setters must persist updated values via `config_runtime` within two seconds.
- All getters must read live data from runtime/config modules; caching allowed only with invalidation on change.
- Faults must follow ONVIF fault conventions with meaningful subcodes.

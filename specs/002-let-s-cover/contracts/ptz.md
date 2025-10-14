# ONVIF PTZ Service Contract Updates

| Operation | Request Requirements | Response Data | Fault Conditions |
|-----------|----------------------|---------------|------------------|
| `GetNodes` | None | PTZ node capabilities (spaces, limits, home support) | `ONVIF_ERROR` on retrieval failure |
| `GetNode` | Node token | Specific node details | `ONVIF_ERROR_NOT_FOUND` invalid token |
| `GetStatus` | Profile token | Current PTZ position, move status, timestamp | `ONVIF_ERROR_NOT_FOUND` invalid profile |
| `ContinuousMove` | Profile token, velocity, timeout | Starts continuous move using adapter | `ONVIF_ERROR_INVALID_PARAMETER` (out of range) |
| `Stop` | Profile token, stop flags | Stops motion | `ONVIF_ERROR_INVALID_PARAMETER` invalid flags |
| `SetPreset` | Profile token, optional preset token/name | Adds/updates preset in config | `ONVIF_ERROR_NO_SPACE` when preset limit reached |
| `GetPresets` | Profile token | Preset list for profile | `ONVIF_ERROR_NOT_FOUND` invalid token |
| `RemovePreset` | Profile token + preset token | Removes preset | `ONVIF_ERROR_NOT_FOUND` invalid token |
| `GotoPreset` | Profile token + preset token | Moves to preset coordinates | `ONVIF_ERROR_NOT_FOUND` preset missing |
| `SetHomePosition` | Profile token | Saves current position as home | `ONVIF_ERROR` if adapter fails |
| `GotoHomePosition` | Profile token | Moves to stored home | `ONVIF_ERROR_NOT_FOUND` if home missing |

## Behavior Notes
- Position values normalized to ONVIF ranges before invocation of platform adapter.
- Preset/home persistence stored per profile via PTZ configuration helpers.
- All movement commands rely on `ptz_adapter_*` wrappers; error codes propagated to ONVIF faults.

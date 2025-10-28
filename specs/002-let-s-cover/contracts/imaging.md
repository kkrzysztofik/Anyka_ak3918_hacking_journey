# ONVIF Imaging Service Contract Updates

| Operation | Request Requirements | Response Data | Fault Conditions |
|-----------|----------------------|---------------|------------------|
| `GetImagingSettings` | Video source token | Current imaging settings for source/profile | `ONVIF_ERROR_NOT_FOUND` invalid token |
| `SetImagingSettings` | Video source token + settings struct (force persistence flag) | Applies validated values to platform and config | `ONVIF_ERROR_INVALID_PARAMETER` for out-of-range values |
| `GetOptions` | Video source token | Limits for brightness, contrast, saturation, sharpness, hue, day/night | `ONVIF_ERROR_NOT_FOUND` invalid token |
| `GetMoveOptions` | Video source token | Supported imaging move types/ranges (focus/iris if applicable) | `ONVIF_ERROR_NOT_SUPPORTED` if move not supported |
| `Move` | Video source token + move struct | Executes supported imaging move(s) | `ONVIF_ERROR_NOT_SUPPORTED` when move type unavailable; `ONVIF_ERROR_INVALID_PARAMETER` for out-of-range inputs |

## Behavior Notes
- Settings values retrieved and stored via imaging config helpers that mirror schema limits.
- Move operations interact with platform VPSS or focus controls when available; otherwise respond with `ONVIF_ERROR_NOT_SUPPORTED`.
- Options MUST match actual capabilities so clients can validate before sending Move/Set requests.

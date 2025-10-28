# ONVIF Media Service Contract Updates

## Profiles & Configuration

| Operation | Request Requirements | Response Data | Fault Conditions |
|-----------|----------------------|---------------|------------------|
| `GetProfiles` | None | All active profiles (≤4) built from runtime stream profiles | `ONVIF_ERROR` if config unavailable |
| `GetProfile` | `ProfileToken` | Specific profile info | `ONVIF_ERROR_NOT_FOUND` invalid token |
| `CreateProfile` | Name, optional token | Creates non-fixed profile if capacity available | `ONVIF_ERROR_NOT_SUPPORTED` when >4 profiles or fixed tokens |
| `DeleteProfile` | `ProfileToken` | Removes non-fixed profile | `ONVIF_ERROR_NOT_SUPPORTED` for fixed profiles |
| `AddConfiguration` / `RemoveConfiguration` | Profile token + config tokens | Adjusts configuration bindings | `ONVIF_ERROR_INVALID_PARAMETER` or `ONVIF_ERROR_NOT_FOUND` |

## Video/Audio Sources & Encoders

| Operation | Request Requirements | Response Data | Fault Conditions |
|-----------|----------------------|---------------|------------------|
| `GetVideoSources` | None | Physical video source info with imaging defaults | `ONVIF_ERROR` on failure |
| `GetAudioSources` | None | Physical audio source info | `ONVIF_ERROR` on failure |
| `GetVideoSourceConfiguration` | None | Bounds and usage counts | `ONVIF_ERROR` on failure |
| `GetVideoEncoderConfiguration` | None | Encoder settings for each stream | `ONVIF_ERROR` on failure |
| `GetVideoEncoderConfigurationOptions` | Profile or configuration token | Ranges for resolution, bitrate, GOP, profiles | `ONVIF_ERROR_INVALID_PARAMETER` invalid tokens |
| `SetVideoSourceConfiguration` | Configuration struct | Applies to referenced source, persists | `ONVIF_ERROR_INVALID_PARAMETER`, `ONVIF_ERROR_NOT_FOUND` |
| `SetVideoEncoderConfiguration` | Encoder struct with profile token | Applies bitrate, GOP, etc., persists | `ONVIF_ERROR_INVALID_PARAMETER` |
| `SetAudioSourceConfiguration` / `SetAudioEncoderConfiguration` | Config struct | Applies/persists audio settings | `ONVIF_ERROR_INVALID_PARAMETER` |
| `GetMetadataConfigurations` | None | Metadata defaults | `ONVIF_ERROR` on failure |
| `SetMetadataConfiguration` | Metadata struct | Applies/persists metadata | `ONVIF_ERROR_INVALID_PARAMETER` |

## Streaming URIs

| Operation | Request Requirements | Response Data | Fault Conditions |
|-----------|----------------------|---------------|------------------|
| `GetStreamUri` | Profile token, protocol | Returns RTSP URI using current network settings | `ONVIF_ERROR_INVALID_PARAMETER` invalid protocol/token |
| `GetSnapshotUri` | Profile token | Returns snapshot HTTP URI | `ONVIF_ERROR` on failure |

## Persistence & Validation
- Video/audio configuration changes must route through `config_runtime` stream profile APIs.
- Options responses derive from hardware limits (resolution, bitrate ranges) defined in configuration schema.
- All setters validate requested values against schema limits before applying changes.

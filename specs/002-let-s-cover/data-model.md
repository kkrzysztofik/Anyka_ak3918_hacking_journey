# Data Model: ONVIF Service Gap Coverage

## ONVIF Service Operation
- **Attributes**:
  - `service`: enum {Device, Media, PTZ, Imaging}
  - `operation`: string (ONVIF SOAP action)
  - `status`: enum {Supported, NotSupported}
  - `capabilities`: map<string, value> (optional metadata advertised via capabilities calls)
- **Relationships**: Links to relevant configuration entities (e.g., Device operations read from Network Configuration Profile).
- **Validation Rules**: Operation must map to a dispatcher entry; unsupported operations must emit standardized faults.

## Network Configuration Profile
- **Attributes**:
  - `hostname`: string
  - `hostname_from_dhcp`: boolean
  - `interfaces`: list of InterfaceConfig
  - `protocols`: list of ProtocolConfig
  - `default_gateway`: IPv4 address
  - `dns_servers`: list of IPv4/hostname entries with source (DHCP/manual)
  - `ntp_servers`: list of hostnames/addresses with source (manual/default)
- **Relationships**: Read/write via Device service getters/setters.
- **Validation Rules**: IP address format, port ranges (1–65535), interface token uniqueness, DHCP/static exclusivity checks.

## ONVIF User Account
- **Attributes**:
  - `username`: string (3–32 alphanumeric)
  - `role`: enum {Administrator, Operator, User}
  - `password_hash`: salted SHA256 string (config runtime format)
  - `active`: boolean
- **Relationships**: Managed via Create/Set/Delete users; referenced during authentication.
- **Validation Rules**: Unique usernames, password policy enforcement, active flag consistency.

## Media Profile Definition
- **Attributes**:
  - `token`: string (Profile1..Profile4)
  - `name`: string (optional)
  - `fixed`: boolean
  - `video_source_token`: string
  - `video_encoder`: struct {token, encoding, resolution, framerate, bitrate, gov_length}
  - `audio_source_token`: string
  - `audio_encoder`: struct {token, encoding, bitrate, sample_rate}
  - `metadata`: struct {token, session_timeout, analytics flag}
  - `ptz`: struct {node_token, default spaces, limits}
- **Relationships**: Bound to video sources and PTZ presets; manipulated by Media service get/set/profile operations.
- **Validation Rules**: Max four profiles, resolution/bitrate within hardware limits, token uniqueness, non-fixed profiles only when created via ONVIF.

## PTZ Preset Collection
- **Attributes**:
  - `profile_token`: string reference to Media Profile
  - `presets`: list of Preset {token, name, pan, tilt, zoom}
  - `home_position`: optional Preset coordinates
- **Relationships**: Updated via PTZ preset operations; consumed by PTZ GET requests.
- **Validation Rules**: Coordinate normalization (-180..180 pan, -90..90 tilt), max four presets per profile, home position optional but unique.

## Imaging Settings Bundle
- **Attributes**:
  - `profile_token`: string reference to Media Profile
  - `brightness`, `contrast`, `saturation`, `sharpness`, `hue`: integers per hardware bounds
  - `daynight`: struct {mode, ir_led_mode, thresholds, lock_time}
- **Relationships**: Read via Imaging GET operations, written via SetImagingSettings/Move.
- **Validation Rules**: Each numeric value within config schema limits; mode enums validated.

## Supporting Structures
- **InterfaceConfig**: {token, enabled, mac_address, mtu, ipv4_config (dhcp flag, manual entries)}
- **ProtocolConfig**: {name, enabled, ports[]}
- **Persistence Queue Entry**: {section, key, value, timestamp} used for coalescing writes.

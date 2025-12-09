# Feature Specification: Rust ONVIF API Rewrite

**Feature Branch**: `001-rust-onvif-api`
**Created**: 2025-01-27
**Status**: Draft
**Input**: User description: "Plan rewritting the @onvif application into Rust. Focus on ONVIF API first."

**Test Specification Basis**: Implementation MUST be based on ONVIF Device Test Specifications version 24.12, specifically:

- ONVIF Base Device Test Specification
- ONVIF Media Device Test Specification (or Media Configuration Device Test Specification)
- ONVIF PTZ Device Test Specification
- ONVIF Imaging Device Test Specification

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Device Discovery and Information Retrieval (Priority: P1)

ONVIF clients need to discover the camera device and retrieve basic device information to understand capabilities and establish communication.

**Why this priority**: Device discovery and information retrieval are fundamental operations that all ONVIF clients perform first. Without this, clients cannot determine what the device supports or how to interact with it. This is the foundation for all other ONVIF operations.

**Independent Test**: Can be fully tested by sending SOAP requests to the Device Service endpoints (GetDeviceInformation, GetCapabilities, GetServices) and verifying correct XML responses. This delivers immediate value by enabling device discovery and capability detection without requiring any other services.

**Acceptance Scenarios**:

1. **Given** an ONVIF client connects to the camera, **When** it sends a GetDeviceInformation request, **Then** the system returns manufacturer, model, firmware version, and serial number in valid ONVIF XML format
2. **Given** an ONVIF client queries device capabilities, **When** it sends a GetCapabilities request, **Then** the system returns all available service endpoints and their supported features
3. **Given** an ONVIF client needs service information, **When** it sends a GetServices request, **Then** the system returns namespace URIs and XAddr URLs for all implemented services
4. **Given** an ONVIF client requests system time, **When** it sends a GetSystemDateAndTime request, **Then** the system returns current date/time in UTC and local timezone formats

---

### User Story 2 - Media Profile and Stream Configuration (Priority: P1)

ONVIF clients need to discover available media profiles, configure video/audio sources, and obtain stream URIs for video playback.

**Why this priority**: Media profiles and stream configuration are essential for clients to understand what video/audio streams are available and how to access them. This enables the core functionality of viewing camera feeds, which is the primary use case for IP cameras.

**Independent Test**: Can be fully tested by sending SOAP requests to Media Service endpoints (GetProfiles, GetStreamUri, GetVideoSources, GetAudioSources) and verifying correct responses with profile tokens, stream URIs, and source configurations. This delivers value by enabling clients to discover and access video streams.

**Acceptance Scenarios**:

1. **Given** an ONVIF client queries available profiles, **When** it sends a GetProfiles request, **Then** the system returns all configured media profiles with video and audio source configurations
2. **Given** an ONVIF client needs a stream URI, **When** it sends a GetStreamUri request with a valid profile token, **Then** the system returns a valid RTSP URI for that profile
3. **Given** an ONVIF client queries video sources, **When** it sends a GetVideoSources request, **Then** the system returns all available video source configurations with resolution and framerate information
4. **Given** an ONVIF client queries encoder configurations, **When** it sends a GetVideoEncoderConfigurations request, **Then** the system returns encoding parameters including codec, bitrate, and resolution settings

---

### User Story 3 - PTZ Control Operations (Priority: P2)

ONVIF clients need to control camera pan, tilt, and zoom movements, manage presets, and perform continuous or absolute positioning.

**Why this priority**: PTZ control is a core feature for cameras with motorized movement capabilities. While not all cameras support PTZ, those that do require reliable control operations. This is secondary to device discovery and media access because PTZ is optional functionality.

**Independent Test**: Can be fully tested by sending SOAP requests to PTZ Service endpoints (AbsoluteMove, RelativeMove, ContinuousMove, Stop, GetPresets, SetPreset, GotoPreset) and verifying correct command execution and response handling. This delivers value by enabling precise camera positioning and preset management.

**Acceptance Scenarios**:

1. **Given** an ONVIF client wants to move the camera to a specific position, **When** it sends an AbsoluteMove request with pan/tilt/zoom coordinates, **Then** the system executes the movement and returns success confirmation
2. **Given** an ONVIF client wants relative movement, **When** it sends a RelativeMove request with pan/tilt/zoom deltas, **Then** the system moves the camera by the specified amounts
3. **Given** an ONVIF client wants continuous movement, **When** it sends a ContinuousMove request with speed parameters, **Then** the system begins continuous movement until a Stop command is received
4. **Given** an ONVIF client wants to stop ongoing movement, **When** it sends a Stop request, **Then** the system immediately halts all pan/tilt/zoom movement
5. **Given** an ONVIF client wants to query PTZ status, **When** it sends a GetStatus request, **Then** the system returns current position and movement status
6. **Given** an ONVIF client wants to save a preset, **When** it sends a SetPreset request with a preset token, **Then** the system saves the current camera position for later recall
7. **Given** an ONVIF client wants to recall a preset, **When** it sends a GotoPreset request with a saved preset token, **Then** the system moves the camera to the saved position
8. **Given** an ONVIF client wants to move to home position, **When** it sends a GotoHomePosition request, **Then** the system moves the camera to the configured home position

---

### User Story 4 - Imaging Settings Control (Priority: P2)

ONVIF clients need to adjust image quality parameters such as brightness, contrast, saturation, and manage day/night mode settings.

**Why this priority**: Imaging control allows clients to optimize video quality for different lighting conditions and preferences. This is important for user experience but secondary to core discovery and streaming functionality.

**Independent Test**: Can be fully tested by sending SOAP requests to Imaging Service endpoints (GetImagingSettings, SetImagingSettings, GetOptions) and verifying correct parameter retrieval and modification. This delivers value by enabling image quality optimization.

**Acceptance Scenarios**:

1. **Given** an ONVIF client queries current imaging settings, **When** it sends a GetImagingSettings request, **Then** the system returns current values for brightness, contrast, saturation, sharpness, and hue
2. **Given** an ONVIF client wants to adjust image quality, **When** it sends a SetImagingSettings request with new parameter values, **Then** the system applies the changes and returns success confirmation
3. **Given** an ONVIF client queries available imaging options, **When** it sends a GetOptions request, **Then** the system returns valid ranges and supported values for all imaging parameters
4. **Given** an ONVIF client wants to change day/night mode, **When** it sends a SetImagingSettings request with mode change, **Then** the system switches between day, night, and auto modes as requested

---

### User Story 5 - User Management (Priority: P2)

ONVIF clients with administrator privileges need to manage user accounts for access control and authentication.

**Why this priority**: User management is essential for security but secondary to core camera functionality. Administrators need to create, modify, and delete user accounts to control who can access the camera.

**Independent Test**: Can be fully tested by sending SOAP requests to Device Service user management endpoints (GetUsers, CreateUsers, DeleteUsers, SetUser) and verifying correct account creation, modification, and deletion.

**Acceptance Scenarios**:

1. **Given** an authenticated administrator, **When** they send a GetUsers request, **Then** the system returns a list of all configured user accounts with usernames and user levels
2. **Given** an authenticated administrator with valid user data, **When** they send a CreateUsers request, **Then** the system creates the new user account and returns success confirmation
3. **Given** an authenticated administrator, **When** they send a DeleteUsers request with a valid username, **Then** the system removes the user account and returns success confirmation
4. **Given** an authenticated administrator, **When** they send a SetUser request with updated password or user level, **Then** the system updates the user account and returns success confirmation
5. **Given** an unauthenticated or non-administrator client, **When** they attempt user management operations, **Then** the system returns an appropriate authorization fault

---

### Edge Cases

#### EC-001: Invalid Operation Name

- **Scenario**: An ONVIF client sends a request with an invalid or unsupported operation name
- **Expected Behavior**: System returns ONVIF fault `ter:ActionNotSupported` with HTTP 500 and descriptive message indicating the operation is not recognized

#### EC-002: Malformed SOAP XML

- **Scenario**: Client sends a request with malformed or invalid SOAP XML structure
- **Expected Behavior**: System returns ONVIF fault `ter:WellFormed` (SOAP sender fault) with HTTP 400 and message describing the XML parsing error

#### EC-003: Invalid Profile Token

- **Scenario**: Client requests a media profile using a token that doesn't exist
- **Expected Behavior**: System returns ONVIF fault `ter:InvalidArgVal` with subcode `ter:NoProfile` and HTTP 400

#### EC-004: Concurrent Service Requests

- **Scenario**: Multiple clients send requests to the same service endpoint simultaneously
- **Expected Behavior**: System processes requests concurrently using thread pool, maintains state consistency via mutex/lock protection, and returns correct responses to each client independently

#### EC-005: Hardware Unavailable

- **Scenario**: PTZ, imaging, or video command is sent but camera hardware is not responding or unavailable
- **Expected Behavior**: System returns ONVIF fault `ter:HardwareFailure` with HTTP 500 and message indicating hardware communication failure; operation is not partially applied

#### EC-006: Missing Required Parameters

- **Scenario**: Client sends a request missing one or more required SOAP parameters
- **Expected Behavior**: System returns ONVIF fault `ter:InvalidArgVal` with subcode `ter:InvalidArgs` and HTTP 400, message identifies the missing parameter(s)

#### EC-007: Unimplemented Service Endpoint

- **Scenario**: Client sends a request to a service endpoint that isn't implemented (e.g., Events, Analytics)
- **Expected Behavior**: System returns HTTP 404 Not Found with message indicating service is not available

#### EC-008: Oversized Request Payload

- **Scenario**: Client sends a SOAP request exceeding the maximum allowed payload size
- **Expected Behavior**: System rejects request with HTTP 413 Payload Too Large before full parsing; connection is closed to prevent resource exhaustion; maximum payload size is configurable (default 1MB)

#### EC-009: Concurrent Settings Modification

- **Scenario**: Multiple clients simultaneously attempt to modify imaging settings or other shared configuration
- **Expected Behavior**: System serializes modifications using mutex protection; last write wins; no partial updates are applied; each client receives correct success/failure response

#### EC-010: System Time Unavailable

- **Scenario**: GetSystemDateAndTime is called but system clock is not synchronized or unavailable
- **Expected Behavior**: System returns available time information with `DateTimeType` set to `Manual`; if no time source is available, returns current system time with appropriate timezone offset

#### EC-011: Authentication Failure

- **Scenario**: Client provides invalid username, password, or malformed authentication credentials
- **Expected Behavior**: System returns ONVIF fault `ter:NotAuthorized` with HTTP 401 Unauthorized; includes `WWW-Authenticate` header for HTTP Digest; logs authentication failure event

#### EC-012: Brute Force Attack Detection

- **Scenario**: Same client IP repeatedly fails authentication attempts
- **Expected Behavior**: After configurable threshold (default: 5 failures in 60 seconds), system temporarily blocks the IP address; blocked requests receive HTTP 429 Too Many Requests; block duration is configurable (default: 300 seconds); security event is logged

#### EC-013: Maximum Users Reached

- **Scenario**: CreateUsers is called when 8 user accounts already exist
- **Expected Behavior**: System returns ONVIF fault `ter:MaxUsers` with HTTP 400 and message indicating maximum user limit reached

#### EC-014: Discovery Mode Disabled

- **Scenario**: WS-Discovery probe is received when discovery mode is set to NonDiscoverable
- **Expected Behavior**: System silently ignores the probe message; no response is sent; no error is logged (normal operation)

#### EC-015: Configuration Storage Full

- **Scenario**: Configuration persistence fails due to storage being full or write-protected
- **Expected Behavior**: System returns ONVIF fault `ter:ConfigurationConflict` with HTTP 500; runtime configuration remains in memory but is not persisted; warning is logged; subsequent requests continue to work with in-memory configuration

#### EC-016: Memory Limit Exceeded

- **Scenario**: Request processing would cause memory usage to exceed the 24MB hard limit
- **Expected Behavior**: System rejects new requests with HTTP 503 Service Unavailable until memory is freed; existing connections are maintained; memory pressure event is logged

#### EC-017: XML Security Attack Detected

- **Scenario**: Request contains XML bomb (billion laughs), XXE injection, or other XML-based attack
- **Expected Behavior**: System immediately terminates parsing and closes connection; returns HTTP 400 Bad Request; logs security event with attack type and source IP; increments rate limit counter for source IP

## Scope Exclusions

The following items are explicitly **out of scope** for this specification:

- **RTSP Streaming Implementation**: Video streaming is a separate concern; this spec covers only the ONVIF API for retrieving stream URIs
- **Events Service**: ONVIF event notification and subscription services are not planned for this implementation
- **Analytics Service**: ONVIF video analytics services are not planned for this implementation
- **Recording Service**: ONVIF recording and replay services are not included
- **Access Control Service**: ONVIF door/access control services are not included

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST implement ONVIF Device Service with all operations covered in the Base Device Test Specification including:
  - GetDeviceInformation, GetCapabilities, GetServices, GetSystemDateAndTime, SystemReboot
  - SetSystemDateAndTime (time management)
  - GetHostname, SetHostname (network configuration)
  - GetNetworkInterfaces, SetNetworkInterfaces (network configuration)
  - GetDNS, SetDNS (network configuration - conditional)
  - GetNTP, SetNTP (time management - conditional)
  - GetScopes, SetScopes, AddScopes (discovery)
  - GetDiscoveryMode, SetDiscoveryMode (discovery)
  - GetUsers, CreateUsers, DeleteUsers, SetUser (user management - see FR-044 to FR-048 for detailed requirements)
  - SetSystemFactoryDefault (maintenance - conditional)
  - And all other operations defined in the Base Device Test Specification
- **FR-002**: System MUST implement ONVIF Media Service with all operations covered in the Media Device Test Specification including:
  - GetProfiles, GetProfile, CreateProfile, DeleteProfile (profile management)
  - GetStreamUri, GetSnapshotUri (URI retrieval)
  - GetVideoSources, GetAudioSources (source enumeration)
  - GetVideoSourceConfigurations, GetVideoSourceConfiguration, SetVideoSourceConfiguration (video source config)
  - GetVideoEncoderConfigurations, GetVideoEncoderConfiguration, SetVideoEncoderConfiguration (video encoder config)
  - GetVideoEncoderConfigurationOptions (encoder options)
  - GetAudioSourceConfigurations, GetAudioSourceConfiguration, SetAudioSourceConfiguration (audio source config)
  - GetAudioEncoderConfigurations, GetAudioEncoderConfiguration, SetAudioEncoderConfiguration (audio encoder config)
  - AddVideoSourceConfiguration, RemoveVideoSourceConfiguration (profile modification)
  - AddVideoEncoderConfiguration, RemoveVideoEncoderConfiguration (profile modification)
  - AddAudioSourceConfiguration, RemoveAudioSourceConfiguration (profile modification)
  - AddAudioEncoderConfiguration, RemoveAudioEncoderConfiguration (profile modification)
  - GetCompatibleVideoSourceConfigurations, GetCompatibleVideoEncoderConfigurations (compatibility)
  - GetCompatibleAudioSourceConfigurations, GetCompatibleAudioEncoderConfigurations (compatibility)
  - GetMetadataConfigurations, SetMetadataConfiguration (metadata config)
  - StartMulticastStreaming, StopMulticastStreaming (multicast control)
  - And all other operations defined in the Media Device Test Specification
- **FR-003**: System MUST implement ONVIF PTZ Service with all operations covered in the PTZ Device Test Specification including:
  - GetNodes, GetNode (node discovery)
  - GetConfigurations, GetConfiguration, SetConfiguration (configuration management)
  - GetConfigurationOptions (configuration options)
  - AbsoluteMove, RelativeMove, ContinuousMove, Stop (movement control)
  - GetStatus (status query)
  - GetPresets, SetPreset, GotoPreset, RemovePreset (preset management)
  - GotoHomePosition, SetHomePosition (home position)
  - GetCompatibleConfigurations (compatibility)
  - SendAuxiliaryCommand (auxiliary control - conditional)
  - And all other operations defined in the PTZ Device Test Specification
- **FR-004**: System MUST implement ONVIF Imaging Service with all operations covered in the Imaging Device Test Specification including:
  - GetImagingSettings, SetImagingSettings (imaging parameters)
  - GetOptions (parameter ranges and supported values)
  - GetMoveOptions, Move, Stop (focus control - conditional)
  - GetStatus (current imaging status)
  - GetServiceCapabilities (service capabilities)
  - And all other operations defined in the Imaging Device Test Specification
- **FR-005**: System MUST accept and process SOAP XML requests over HTTP according to ONVIF 24.12 specification
- **FR-006**: System MUST generate valid ONVIF-compliant SOAP XML responses for all implemented operations
- **FR-006a**: System MUST implement SOAP 1.2 per WS-I Basic Profile 2.0
- **FR-007**: System MUST validate incoming SOAP request structure and parameters before processing
- **FR-008**: System MUST return appropriate ONVIF fault messages for invalid requests or operation failures
- **FR-009**: System MUST support HTTP POST requests to service endpoints (e.g., `/onvif/device_service`, `/onvif/media_service`)
- **FR-010**: System MUST handle concurrent requests to multiple service endpoints simultaneously
- **FR-011**: System MUST maintain service state consistency across concurrent operations
- **FR-012**: System MUST provide thread-safe access to shared service data structures
- **FR-013**: System MUST parse SOAP action headers to route requests to correct service operations
- **FR-014**: System MUST support ONVIF namespace URIs for all implemented services
- **FR-015**: System MUST return service capabilities that accurately reflect implemented functionality
- **FR-016**: System MUST handle missing or optional parameters in SOAP requests according to ONVIF specification
- **FR-017**: System MUST validate parameter ranges and types before executing operations
- **FR-018**: System MUST provide error handling that returns appropriate ONVIF fault codes and messages
- **FR-019**: System MUST support service discovery through GetServices operation returning all available service endpoints
- **FR-020**: System MUST maintain backward compatibility with existing ONVIF client implementations
- **FR-021**: System MUST implement operations and behaviors as defined in ONVIF Device Test Specifications version 24.12 for Base Device, Media, PTZ, and Imaging services
- **FR-022**: System MUST pass all applicable test cases defined in the referenced ONVIF Device Test Specifications for implemented services (full conformance validation required)
- **FR-023**: System MUST validate implementation against ONVIF test tool or equivalent test framework using the referenced test specifications
- **FR-024**: System MUST implement all operations from test specifications regardless of whether they existed in the original C implementation
- **FR-025**: System MUST use platform abstraction layer stubs for hardware-dependent operations to enable API testing without actual hardware
- **FR-026**: System MUST return appropriate ONVIF fault responses when hardware-dependent operations are invoked but underlying hardware is unavailable

#### Authentication Requirements

- **FR-027**: System MUST implement WS-Security UsernameToken with Digest authentication (nonce + created timestamp + password digest)
- **FR-028**: System MUST implement HTTP Digest authentication per RFC 2617 with Quality of Protection (qop) support
- **FR-029**: System MUST validate credentials against configured user database for all authenticated operations
- **FR-030**: System MUST reject requests with invalid or expired credentials with appropriate ONVIF fault responses
- **FR-030a**: System MUST expire authentication nonces after configurable duration (default: 60 seconds)

#### CI/CD Requirements

- **FR-031**: System MUST include GitHub Actions workflow for continuous integration
- **FR-032**: CI workflow MUST run on every push and pull request to feature branch
- **FR-033**: CI workflow MUST execute all unit tests with `cargo test`
- **FR-034**: CI workflow MUST run `cargo clippy` for static analysis with warnings as errors
- **FR-035**: CI workflow MUST verify code formatting with `cargo fmt --check`
- **FR-036**: CI workflow MUST generate and archive test coverage reports in multiple formats: HTML format for human viewing (uploaded as artifact), LCOV format for machine processing and CI integration, and Cobertura XML format for GitHub PR annotations
- **FR-037**: CI workflow SHOULD cache Cargo dependencies and build artifacts for performance
- **FR-038**: Cross-compilation workflow MUST run only on git tags (release triggers)
- **FR-039**: Cross-compilation MUST target `armv5te-unknown-linux-uclibceabi`
- **FR-040**: Release workflow MUST produce ZIP archive containing SD_card_content structure
- **FR-041**: Release workflow MUST upload ZIP as GitHub Release artifact
- **FR-042**: CI workflow MUST use updated Docker image with Rust cross-compilation toolchain
- **FR-043**: Docker image MUST include ARM cross-compiler and Anyka SDK dependencies

#### Rust Testing & Quality Gates (onvif-rust)

- For the Rust ONVIF API implementation in `cross-compile/onvif-rust/`, the mandatory testing and quality gates are:
  - `cargo fmt --check` (formatting)
  - `cargo clippy -- -D warnings` (linting)
  - `cargo test --all-features` (unit and integration tests)
  - `cargo doc --no-deps` (API documentation)
  - `cargo tarpaulin` with a minimum of 80% code coverage (coverage)
- These gates are defined and governed by the Rust addendum to the constitution in `.specify/memory/constitution-rust.md` and serve as the Rust equivalents of the main constitution’s `lint_code.sh`, `format_code.sh`, `make test`, and gcov/lcov requirements.

#### User Management Requirements

- **FR-044**: System MUST support up to 8 user accounts with username, password, and user level
- **FR-045**: System MUST implement ONVIF GetUsers, CreateUsers, DeleteUsers, and SetUser operations
- **FR-046**: System MUST store passwords using Argon2id hashing (OWASP recommended, never plaintext)
- **FR-047**: System MUST persist user accounts across system restarts
- **FR-048**: System MUST support user levels: Administrator, Operator, and User (no Anonymous level)
- **FR-048a**: System MUST support configurable authentication bypass via `[server] auth_enabled = true/false` configuration option; when `auth_enabled = false`, all requests bypass authentication entirely

#### WS-Discovery Requirements

- **FR-049**: System MUST implement WS-Discovery protocol for device discovery
- **FR-050**: System MUST respond to Probe messages on multicast group 239.255.255.250:3702
- **FR-051**: System MUST send Hello messages on startup and Bye messages on shutdown
- **FR-052**: System MUST include device scopes and endpoint references in discovery responses

#### Snapshot Service Requirements

- **FR-053**: System MUST implement GetSnapshotUri operation in Media Service
- **FR-054**: System MUST support configurable snapshot resolution (width, height)
- **FR-055**: System MUST support configurable JPEG quality (1-100)
- **FR-055a**: Snapshot URI MUST follow format: `http://{device_ip}/onvif/snapshot?profile={token}&resolution={width}x{height}&quality={1-100}`

#### Configuration Requirements

- **FR-056**: System MUST persist configuration in TOML format
- **FR-057**: System MUST validate configuration values against defined min/max ranges
- **FR-058**: System MUST support runtime configuration updates with validation
- **FR-058a**: The following configuration parameters MUST be updatable at runtime without restart: log level, rate limit thresholds, brute force protection thresholds. All other parameters require service restart.
- **FR-059**: System MUST support at least 4 configurable media profiles with independent settings

#### Input Validation Requirements

- **FR-060**: System MUST validate all HTTP request components (method, path, version) before processing
- **FR-061**: System MUST validate SOAP envelope structure and action headers
- **FR-062**: System MUST validate and sanitize all user-provided input strings

#### Platform Abstraction Requirements

- **FR-063**: System MUST define platform abstraction traits for all hardware operations (video, audio, PTZ, imaging)
- **FR-064**: System MUST provide stub implementations of all platform traits for testing without hardware

#### Error Handling Requirements

- **FR-065**: System MUST return ONVIF fault `ter:ActionNotSupported` with HTTP 500 for unrecognized operation names (EC-001)
- **FR-066**: System MUST return ONVIF fault `ter:WellFormed` with HTTP 400 for malformed SOAP XML requests (EC-002)
- **FR-067**: System MUST return ONVIF fault `ter:InvalidArgVal/ter:NoProfile` with HTTP 400 for invalid profile tokens (EC-003)
- **FR-068**: System MUST process concurrent requests using thread pool with mutex-protected shared state (EC-004)
- **FR-069**: System MUST return ONVIF fault `ter:HardwareFailure` with HTTP 500 when hardware is unavailable (EC-005)
- **FR-070**: System MUST return ONVIF fault `ter:InvalidArgVal/ter:InvalidArgs` with HTTP 400 for missing required parameters (EC-006)
- **FR-071**: System MUST return HTTP 404 Not Found for requests to unimplemented service endpoints (EC-007)
- **FR-072**: System MUST reject requests exceeding configurable maximum payload size (default 1MB) with HTTP 413 (EC-008)
- **FR-073**: System MUST serialize concurrent configuration modifications using mutex protection (EC-009)
- **FR-074**: System MUST return available time information with `DateTimeType=Manual` when system time is not synchronized (EC-010)
- **FR-075**: System MUST return ONVIF fault `ter:NotAuthorized` with HTTP 401 and `WWW-Authenticate` header for authentication failures (EC-011)
- **FR-076**: System MUST implement brute force protection with configurable threshold (default: 5 failures/60s) and block duration (default: 300s) (EC-012)
- **FR-077**: System MUST return ONVIF fault `ter:MaxUsers` with HTTP 400 when maximum user count (8) is reached (EC-013)
- **FR-078**: System MUST silently ignore WS-Discovery probes when discovery mode is NonDiscoverable (EC-014)
- **FR-079**: System MUST return ONVIF fault `ter:ConfigurationConflict` with HTTP 500 when configuration persistence fails (EC-015)
- **FR-080**: System MUST return HTTP 503 Service Unavailable when memory limit would be exceeded (EC-016)
- **FR-081**: System MUST immediately terminate connections and return HTTP 400 when XML security attacks are detected (EC-017)
- **FR-082**: System MUST log all security events including authentication failures, blocked IPs, and detected attacks

### Non-Functional Requirements

#### Memory Constraints

- **NFR-001**: System runtime heap memory usage MUST NOT exceed 24MB (hard limit)
- **NFR-002**: System runtime heap memory usage SHOULD remain below 16MB under normal operation (soft limit)
- **NFR-003**: System MUST implement memory profiling capability for development builds
- **NFR-004**: System MUST handle memory allocation failures gracefully with appropriate ONVIF fault responses

#### Security Hardening

- **NFR-005**: System MUST implement rate limiting with configurable request thresholds per client IP (default: 60 requests per minute)
- **NFR-006**: System MUST detect and block common web attack patterns relevant to ONVIF HTTP/SOAP traffic, including at minimum cross-site scripting (XSS) payloads in user-controllable fields and path traversal attempts in URLs or filenames. Where the implementation introduces persistent storage or query-like subsystems (for example SQL databases or key–value queries), equivalent input validation and injection protection MUST be applied to those interfaces.
- **NFR-007**: System MUST implement XML security measures (XXE prevention, XML bomb detection)
- **NFR-008**: System MUST log security events for audit purposes

#### Server Configuration

- **NFR-009**: System MUST support configurable worker thread count (1-32 threads, default: 4)
- **NFR-010**: System MUST support configurable connection limits (default: 100) and timeouts (connection: 30s, request: 60s)

#### Logging

- **NFR-011**: System MUST implement configurable logging with multiple levels (ERROR, WARNING, INFO, DEBUG; default: INFO)
- **NFR-012**: System MUST support configurable HTTP request/response logging for debugging via a dedicated verbose mode. When enabled, the system MUST log at least HTTP method, path, response status code, and request latency for ONVIF endpoints, and MAY log truncated, sanitized SOAP payloads; logging MUST be controllable via configuration (for example a `[logging] http_verbose = true/false` option).

### Key Entities *(include if feature involves data)*

- **ONVIF Service**: Represents an ONVIF-compliant web service (Device, Media, PTZ, Imaging) with namespace URI, XAddr endpoint, and version information
- **Service Operation**: Represents a single ONVIF operation (e.g., GetDeviceInformation) that accepts SOAP requests and returns SOAP responses
- **Media Profile**: Represents a configured media stream profile containing video source, video encoder, audio source, and audio encoder configurations with a unique token identifier
- **PTZ Preset**: Represents a saved camera position with pan, tilt, and zoom coordinates identified by a unique preset token
- **Imaging Settings**: Represents current image quality parameters including brightness, contrast, saturation, sharpness, hue, and day/night mode configuration
- **Service Capabilities**: Represents the set of features and operations supported by each ONVIF service, used to inform clients of available functionality
- **User Account**: Represents a system user with username, password hash (Argon2id), user level (Administrator/Operator/User), and active status
- **Auto Day/Night Configuration**: Represents automatic day/night switching settings including luminance thresholds, lock time, and IR LED mode
- **Video Encoder Configuration**: Represents video encoding parameters including codec type, resolution, framerate, bitrate, GOP size, and bitrate mode (CBR/VBR)
- **Security Context**: Represents per-request security information including client IP, request timestamp, and authentication status for rate limiting and audit logging

## Clarifications

### Session 2025-01-27

- Q: What ONVIF specification version should be enforced? → A: ONVIF 24.12 (updated from 2.5 per user requirement)
- Q: Which ONVIF Device Test Specifications should serve as the primary basis for implementation? → A: Base Device, Media, PTZ, and Imaging test specifications (all four core services)
- Q: What is the scope of operations to implement from the test specifications? → A: All operations covered in the Base Device, Media, PTZ, and Imaging test specifications (comprehensive coverage)
- Q: What level of test case conformance should the implementation target? → A: Pass all applicable test cases from the referenced test specifications (full conformance validation)
- Q: How should we handle operations in the test specifications that weren't in the original C implementation or have hardware dependencies? → A: Implement all operations from test specs, use platform abstraction stubs for hardware-dependent operations

### Session 2025-01-28 (Spec Validation)

- Q: Is RTSP streaming implementation in scope? → A: No, RTSP streaming is a separate concern; only ONVIF API for stream URIs is in scope
- Q: Is Events Service in scope? → A: No, Events Service is not planned for this implementation
- Q: Is Analytics Service in scope? → A: No, Analytics Service is not planned for this implementation
- Q: What authentication schemes are required? → A: Both WS-Security UsernameToken (Digest) AND HTTP Digest authentication are required
- Q: What are the memory constraints? → A: 16MB soft limit, 24MB hard limit (32MB total RAM available)
- Q: What CI/CD approach should be used? → A: GitHub Actions with CI on every push/PR; cross-compilation only on tags producing ZIP with SD_card_content
- Q: Is user management required? → A: Yes, full user management (GetUsers, CreateUsers, DeleteUsers, SetUser) as implemented in C version

### Session 2025-01-28 (Plan Validation)

- Q: What password hashing algorithm should be used? → A: Argon2id (OWASP recommended) instead of SHA256; updated FR-046
- Q: Should Anonymous user level be supported? → A: No, only Administrator, Operator, and User levels; updated FR-048
- Q: How should authentication be disabled for development/testing? → A: Via `[server] auth_enabled = false` configuration option; added FR-048a
- Q: What coverage report formats are required? → A: HTML (human readable), LCOV (machine readable for CI), Cobertura XML (GitHub PR annotations); updated FR-036
- Q: Is hyper crate needed alongside axum? → A: No, axum re-exports required hyper types; hyper not added as direct dependency

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: ONVIF clients can successfully discover the device and retrieve device information within 2 seconds of connection
- **SC-002**: All implemented ONVIF service operations respond to valid requests with correct SOAP XML format according to ONVIF 24.12 specification
- **SC-003**: System processes 95% of valid ONVIF requests successfully without errors
- **SC-004**: System handles concurrent requests from up to 10 simultaneous ONVIF clients without performance degradation
- **SC-005**: Invalid or malformed SOAP requests are rejected with appropriate ONVIF fault messages within 500 milliseconds
- **SC-006**: All service operations return responses that pass ONVIF XML schema validation
- **SC-007**: System maintains service state consistency with zero data corruption under concurrent access scenarios
- **SC-008**: ONVIF Device Manager and other standard ONVIF client tools can successfully discover and interact with all implemented services
- **SC-009**: System provides equivalent functionality to the existing C implementation for all ONVIF API operations
- **SC-010**: Service endpoint routing correctly identifies and dispatches requests to appropriate service handlers 100% of the time
- **SC-011**: System passes 100% of applicable test cases from ONVIF Base Device, Media, PTZ, and Imaging Device Test Specifications version 24.12
- **SC-012**: All implemented operations demonstrate conformance with ONVIF 24.12 specification requirements as validated by test specification test cases
- **SC-013**: All CI checks pass before pull request can be merged
- **SC-014**: Tagged releases produce valid ARM binary in SD_card_content ZIP archive
- **SC-015**: Unit test coverage maintains minimum 80% line coverage
- **SC-016**: No clippy warnings or errors in CI run
- **SC-017**: Memory profiling under load test shows peak usage below 16MB for typical workloads
- **SC-018**: System remains stable and responsive when memory usage approaches 24MB limit

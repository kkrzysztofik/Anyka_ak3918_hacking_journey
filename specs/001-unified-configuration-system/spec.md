# Feature Specification: Unified Configuration System for ONVIF Daemon

**Feature Branch**: `001-unified-configuration-system`
**Created**: 2025-10-10
**Status**: Draft
**Input**: User description: "Unified configuration system for ONVIF daemon with runtime updates, stream profiles, and user management"

## User Scenarios & Testing _(mandatory)_

### User Story 1 - Single Source of Truth for Configuration (Priority: P1)

As a firmware maintainer, I want all subsystems to read configuration from a single canonical source so that configuration drift between modules is eliminated and system behavior is predictable.

**Why this priority**: This is the foundation of the unified config system. Without a single source of truth, all other features will be built on an inconsistent base, leading to bugs and maintenance nightmares.

**Independent Test**: Can be fully tested by loading configuration at daemon startup, querying values from different subsystems (services, platform, networking), and verifying all receive identical values. Delivers immediate value by eliminating configuration inconsistencies.

**Acceptance Scenarios**:

1. **Given** daemon starts with a valid INI configuration file, **When** multiple subsystems (media service, PTZ service, platform layer) request the same configuration value, **Then** all subsystems receive the identical value from the unified manager
2. **Given** primary configuration file is missing or corrupted, **When** daemon initializes, **Then** system loads validated defaults, emits warning logs, and continues running with documented baseline behavior
3. **Given** daemon is running with loaded configuration, **When** a developer inspects configuration state via CLI or logs, **Then** the canonical values including derived defaults are displayed within one event cycle

---

### User Story 2 - Schema-Driven Configuration Validation (Priority: P1)

As a service developer, I want configuration parameters to be validated against a declared schema so that invalid values are caught at load time rather than causing runtime failures.

**Why this priority**: Runtime failures due to invalid configuration cause camera malfunctions and customer complaints. Schema-driven validation prevents these issues and makes the system more robust.

**Independent Test**: Can be fully tested by providing various invalid configuration files (type mismatches, out-of-bounds values, missing required keys) and verifying the system rejects them with specific error messages. Delivers immediate value by catching configuration errors early.

**Acceptance Scenarios**:

1. **Given** configuration schema defines an integer parameter with range 1-100, **When** INI file provides value 150, **Then** validation fails with error indicating the specific key, section, and constraint violation
2. **Given** configuration schema marks a parameter as required, **When** INI file omits this parameter, **Then** system fails validation with structured log identifying the missing required key
3. **Given** a developer adds a new configuration parameter, **When** parameter is registered with type, limits, and default value in central schema, **Then** validation and parsing automatically handle the new parameter without additional code

---

### User Story 3 - Runtime Configuration Updates with Async Persistence (Priority: P2)

As an operations engineer, I want to update configuration values at runtime via ONVIF SetConfiguration calls or management tools with changes persisted asynchronously so that operations remain responsive while ensuring configuration survives reboots.

**Why this priority**: Runtime updates enable remote management without camera reboots, critical for production deployments. Async persistence ensures responsiveness while providing durability.

**Independent Test**: Can be fully tested by calling configuration update API, verifying immediate in-memory change, confirming operation returns quickly, then checking persistence occurs within seconds and survives daemon restart. Delivers value by enabling live configuration management.

**Acceptance Scenarios**:

1. **Given** camera is operational, **When** operator updates video bitrate via ONVIF SetVideoEncoderConfiguration, **Then** change is applied immediately in-memory, API returns within 200ms, and value is persisted to disk within 2 seconds
2. **Given** configuration update is persisted successfully, **When** daemon is restarted, **Then** system loads the updated value on next boot
3. **Given** filesystem is full or read-only, **When** async persistence fails, **Then** in-memory change remains active, error is logged with specific failure reason, and management UI shows persistence failure status
4. **Given** multiple rapid configuration updates occur, **When** async persistence is triggered, **Then** system coalesces writes to avoid filesystem thrashing and ensures final state is correctly persisted

---

### User Story 4 - Stream Profile Configuration Management (Priority: P2)

As a camera administrator, I want to configure up to 4 video stream profiles with different resolutions, bitrates, and encoding settings so that I can optimize streams for different use cases (recording, live view, mobile).

**Why this priority**: Multi-profile support is essential for modern IP cameras to serve different client needs efficiently. Limiting to 4 profiles balances flexibility with resource constraints on embedded hardware.

**Independent Test**: Can be fully tested by defining 4 stream profiles with varying settings, requesting streams via ONVIF GetProfiles and GetStreamUri, and verifying each profile delivers video with the specified parameters. Delivers value by enabling use-case-optimized streaming.

**Acceptance Scenarios**:

1. **Given** administrator configures Profile 1 (1080p@30fps, 4Mbps) and Profile 2 (720p@15fps, 1Mbps), **When** ONVIF client calls GetProfiles, **Then** both profiles are listed with correct resolution, framerate, and bitrate settings
2. **Given** 4 stream profiles are already configured, **When** administrator attempts to add a 5th profile, **Then** system rejects the request with error "Maximum profile limit (4) reached"
3. **Given** stream profile is updated at runtime, **When** active clients are streaming from that profile, **Then** encoder settings are updated seamlessly without dropping existing connections
4. **Given** configuration file contains profile with invalid resolution or unsupported codec, **When** daemon loads configuration, **Then** validation fails with specific error identifying the invalid profile parameter

---

### User Story 5 - User Credential Management (Priority: P2)

As a security administrator, I want to manage up to 8 user accounts with usernames and passwords for ONVIF authentication so that I can control access to camera functionality with proper credential management.

**Why this priority**: Multi-user support with credential management is essential for enterprise deployments where different operators need different access levels. Limiting to 8 users balances security flexibility with embedded resource constraints.

**Independent Test**: Can be fully tested by creating multiple user accounts, authenticating ONVIF requests with different credentials, and verifying access is granted or denied appropriately. Delivers value by enabling multi-user access control.

**Acceptance Scenarios**:

1. **Given** administrator configures users "admin" (password "secure123") and "viewer" (password "view456"), **When** ONVIF client authenticates with correct credentials, **Then** authentication succeeds and client can access authorized operations
2. **Given** 8 users are already configured, **When** administrator attempts to add a 9th user, **Then** system rejects the request with error "Maximum user limit (8) reached"
3. **Given** user credentials are stored in configuration, **When** configuration is persisted to disk, **Then** passwords are hashed using SHA256 algorithm and plaintext passwords are never written to storage
4. **Given** user password is updated via ONVIF SetUser operation, **When** update completes, **Then** new hashed password is persisted and subsequent authentication requires new password
5. **Given** ONVIF client provides incorrect credentials, **When** authentication is attempted, **Then** access is denied, failure is logged with username and timestamp, and no sensitive information is revealed in error message

### Edge Cases

- What happens when async persistence queue is full due to excessive rapid updates?

  - System throttles updates, returns "too many requests" error to caller, and logs queue saturation

- How does system handle concurrent configuration updates from multiple ONVIF clients?

  - Updates are serialized through runtime manager with mutex protection, last-write-wins semantics

- What happens when stream profile is deleted while clients are actively streaming?

  - Active streams continue until client disconnects, new stream requests for deleted profile return "profile not found" error

- How does system recover if async persistence crashes mid-write?

  - Temp-file atomic rename pattern ensures on-disk state is never corrupted; incomplete writes are discarded

- What happens when configuration file size exceeds embedded filesystem limits?

  - Schema validation enforces maximum configuration file size (16KB limit), rejects oversized configurations at load time

- How are user credentials handled during configuration export/backup?

  - Export includes hashed passwords only, plaintext passwords are never exported or logged

- What happens when daemon hot-reloads configuration while serving ONVIF requests?
  - Configuration updates are applied atomically, active requests use snapshot of pre-update state until completion

## Requirements _(mandatory)_

### Functional Requirements

#### Core Configuration Management

- **FR-001**: System MUST load configuration through single initialization pathway (`config_lifecycle_load_configuration`) producing fully-populated `application_config` structure used across all subsystems
- **FR-002**: System MUST serve configuration data through unified `config_manager_t` API or generated accessors, eliminating all dependencies on legacy `platform_config_get_*` buffers
- **FR-003**: System MUST register each configuration key with metadata (type, limits, default, required flag) in central schema consumed automatically by loader and validation routines
- **FR-004**: System MUST fail validation with ONVIF error constant and emit structured log identifying offending key and section when configuration violates declared constraints
- **FR-005**: System MUST provide fallback to validated defaults when primary config file is missing or corrupted, emit warning log, and continue running with documented baseline behavior

#### Runtime Updates and Persistence

- **FR-006**: System MUST apply configuration value updates immediately in-memory when `config_set_value` or equivalent API is called
- **FR-007**: System MUST persist configuration changes asynchronously to disk within 2 seconds of in-memory update
- **FR-008**: System MUST use atomic persistence pattern (temp-file write + rename) to prevent partial writes and filesystem corruption
- **FR-009**: System MUST roll back in-memory changes when persistence fails due to filesystem errors (read-only, disk full), log recoverable error, and surface status for management UI
- **FR-010**: System MUST coalesce multiple rapid configuration updates to avoid filesystem thrashing while ensuring final state is correctly persisted
- **FR-011**: System MUST provide configuration summary/introspection endpoint (CLI dump, REST hook, or log output) reflecting canonical values including derived defaults within one event cycle

#### Stream Profile Configuration

- **FR-012**: System MUST support configuration of up to 4 video stream profiles with distinct resolution, framerate, bitrate, and encoding parameters
- **FR-013**: System MUST reject attempts to create more than 4 stream profiles with error "Maximum profile limit (4) reached"
- **FR-014**: System MUST validate stream profile parameters (resolution, codec, bitrate ranges) against hardware capabilities during configuration load and runtime updates
- **FR-015**: System MUST allow runtime updates to stream profile settings with changes applied seamlessly to encoder without dropping active client connections
- **FR-016**: System MUST persist stream profile configurations to disk and restore them on daemon restart

#### User Credential Management

- **FR-017**: System MUST support configuration of up to 8 user accounts with username and password for ONVIF authentication
- **FR-018**: System MUST reject attempts to create more than 8 user accounts with error "Maximum user limit (8) reached"
- **FR-019**: System MUST hash all passwords using SHA256 algorithm before persisting to disk, never storing plaintext passwords
- **FR-020**: System MUST validate username format (alphanumeric, 3-32 characters) during user creation and updates
- **FR-021**: System MUST support runtime user account updates (add, delete, modify password) via ONVIF SetUser operations with changes persisted asynchronously
- **FR-022**: System MUST log all authentication attempts (success and failure) with username, timestamp, and source IP without revealing password information
- **FR-023**: System MUST never expose plaintext passwords in logs, error messages, configuration dumps, or exports

### Key Entities _(include if feature involves data)_

- **Configuration Schema Entry**: Metadata for a single configuration parameter including section, key, type (int, string, float, bool), required flag, validation constraints (min/max value, max length), and default value
- **Application Configuration**: Complete runtime configuration state containing all subsystem settings including network, services, platform, stream profiles, and user accounts
- **Stream Profile**: Video stream configuration with unique profile identifier, resolution (width/height), framerate, bitrate, codec (H.264/H.265), GOP size, and encoder quality settings
- **User Account**: Credential entry with unique username, hashed password (bcrypt), account status (active/disabled), creation timestamp, and last modified timestamp
- **Configuration Snapshot**: Point-in-time immutable view of application configuration with generation counter for change detection
- **Persistence Queue Entry**: Pending configuration change awaiting async disk write, containing updated key-value pairs and timestamp

## Success Criteria _(mandatory)_

### Measurable Outcomes

- **SC-001**: Configuration initialization completes within 150ms on AK3918 hardware for configuration files under 16KB
- **SC-002**: Runtime configuration getters execute in under 10 microseconds without heap allocations
- **SC-003**: Configuration updates return to caller within 200ms with in-memory changes applied immediately
- **SC-004**: Async persistence completes within 2 seconds of in-memory update under normal filesystem conditions
- **SC-005**: System handles 100 configuration queries per second without performance degradation
- **SC-006**: All configuration parsing operations sanitize strings and reject oversized inputs, preventing injection vulnerabilities
- **SC-007**: Daemon continues operation with default configuration when primary config file is missing, achieving 100% uptime resilience
- **SC-008**: Configuration validation catches 100% of type mismatches, bounds violations, and missing required keys before runtime
- **SC-009**: Stream profile switching occurs within 500ms without dropping active RTSP connections
- **SC-010**: User authentication operations complete within 100ms including SHA256 password verification
- **SC-011**: Zero configuration-related security vulnerabilities detected by Snyk Code and manual security review
- **SC-012**: Configuration system unit tests achieve >90% code coverage with 100% pass rate
- **SC-013**: All configuration operations pass linting and formatting validation without errors
- **SC-014**: System supports 10 concurrent ONVIF clients performing configuration queries without contention delays

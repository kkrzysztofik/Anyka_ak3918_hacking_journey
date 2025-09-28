# Requirements Document

## Introduction

This specification defines the goals for refactoring the ONVIF daemon configuration system so that all modules (core daemon, platform adapters, services, utilities, and tests) share a single, validated source of truth. The outcome should eliminate duplicated configuration parsers, make typed access predictable, and support future feature work (stream profiles, platform tuning, web UI updates) without touching multiple code paths.

## Alignment with Product Vision

Centralizing configuration directly supports the project principles of security, reliability, and developer experience. A unified configuration layer improves maintainability, reduces risk of configuration drift between Anyka reference defaults and custom firmware, and prepares the system for advanced camera capabilities while keeping the embedded footprint small.

## Requirements

### Requirement 1

**User Story:** As a firmware maintainer, I want a single canonical configuration representation for the daemon, so that every subsystem reads consistent values and defaults.

#### Acceptance Criteria

1. WHEN the daemon starts, THEN configuration SHALL be loaded through one initialization pathway (`config_lifecycle_load_configuration`) that produces a fully-populated `application_config` structure used across services, networking, and platform layers.
2. WHEN any subsystem requests configuration data, THEN the system SHALL serve it through the unified `config_manager_t` API (or generated accessors), and no module SHALL depend on legacy buffers such as `platform_config_get_*`.
3. IF the primary config file is missing or corrupted, THEN the system SHALL fall back to validated defaults, emit a warning, and continue running with documented baseline behavior.

### Requirement 2

**User Story:** As a service developer, I want strongly validated, schema-driven configuration definitions, so that new parameters can be added without duplicating parsing logic or risking invalid runtime values.

#### Acceptance Criteria

1. WHEN a configuration key is registered, THEN its metadata (type, limits, default, required flag) SHALL be declared in a central schema and consumed automatically by both loader and validation routines.
2. WHEN configuration data violates declared constraints (type mismatch, bounds, missing required key), THEN validation SHALL fail with an ONVIF error constant and emit structured logs that identify the offending key and section.
3. WHEN new configuration keys are added, THEN unit tests and documentation SHALL be updated automatically or via templated helpers, preventing unchecked parameters from reaching runtime.

### Requirement 3

**User Story:** As an operations engineer, I want a predictable persistence and introspection interface for configuration updates, so that tooling (SD-card UI, automation scripts, ONVIF SetConfiguration calls) can modify settings without manual file edits.

#### Acceptance Criteria

1. WHEN configuration values are updated at runtime via `config_set_value` or equivalent API, THEN the system SHALL provide an atomic persistence path that writes back to disk (or staging) without corrupting existing data.
2. WHEN the configuration state changes, THEN a summary/introspection endpoint (CLI dump, REST hook, or log output) SHALL reflect the canonical values, including derived defaults, within one event cycle.
3. IF persistence fails (filesystem read-only, disk full), THEN the system SHALL roll back in-memory changes, log a recoverable error, and surface status for the management UI or diagnostics.

## Non-Functional Requirements

### Code Architecture and Modularity
- **Single Responsibility Principle**: Configuration schema definition, I/O, validation, and accessors SHALL live in clearly separated modules (e.g., `core/config`, `core/lifecycle`, `utils`) with no duplicated parsing logic.
- **Modular Design**: Services (media, PTZ, imaging, device, snapshot) SHALL consume configuration only via published interfaces to avoid cross-layer coupling.
- **Dependency Management**: The refactor SHALL keep platform adapters free from direct file parsing logic; platform code remains consumers of the unified API.
- **Clear Interfaces**: Documented accessors and data transfer structures SHALL hide internal parsing details from higher layers.

### Performance
- Configuration initialization SHALL complete within 150 ms on AK3918 hardware for standard INI files under 8 KB.
- Runtime getters SHALL execute in O(1) time without heap allocations to preserve daemon responsiveness.

### Security
- Configuration parsing SHALL sanitize strings (length, encoding) and reject unsanitized paths to prevent injection and buffer overflow vulnerabilities.
- Persistence routines SHALL write to temporary files and perform atomic rename to avoid partial writes exploitable by attackers.

### Reliability
- The configuration layer SHALL detect and report checksum or syntax errors before services start, preventing partially configured states.
- Hot reload or reinitialization paths SHALL leave the daemon in a consistent state even if mid-operation updates fail.

### Usability
- Developer documentation SHALL include a generated schema reference and onboarding guide for adding new keys.
- Tooling (CLI/UI) SHALL expose clear success/failure messaging for configuration load/save operations, reducing guesswork during SD-card testing.

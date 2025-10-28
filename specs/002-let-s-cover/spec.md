# Feature Specification: ONVIF Service Gap Coverage

**Feature Branch**: `002-let-s-cover`  
**Created**: 2025-10-13  
**Status**: Draft  
**Input**: User description: "let's cover a gap in existing services (we won't implement events and analytics) between our onvif implementation and the captured dumps in cap folder"

## User Scenarios & Testing *(mandatory)*

### User Story 1 - Retrieve Accurate Service Data (Priority: P1)

ONVIF client integrators want the camera to return complete, standards-compliant responses for all Device, Media, PTZ, and Imaging queries captured in the `cap/` traffic so that discovery, capability assessment, and monitoring work without manual tweaks.

**Why this priority**: Without correct read operations the camera fails certification and basic client interoperability, blocking distribution.

**Independent Test**: Replay captured GET requests (excluding events/analytics) against the camera and confirm responses match ONVIF schema and observed payloads.

**Acceptance Scenarios**:

1. **Given** the camera is online with default configuration, **When** an ONVIF client issues `GetServiceCapabilities`, `GetHostname`, `GetNetworkInterfaces`, and related Device requests, **Then** the camera returns success responses containing the expected data values and flags.
2. **Given** the client requests `GetProfiles`, `GetStreamUri`, `GetNode`, `GetStatus`, `GetImagingSettings`, and other Media/PTZ/Imaging queries present in the captures, **When** those calls are exercised, **Then** the camera returns populated, schema-valid responses without faults.

---

### User Story 2 - Manage Configuration via ONVIF (Priority: P1)

Remote operators need to adjust network, user, video, PTZ, and imaging settings through standard ONVIF Set operations so they can provision devices without physical access.

**Why this priority**: Configuration management is mandatory for field deployment and part of ONVIF conformance suites; missing setters forces out-of-band tooling.

**Independent Test**: Execute setter requests (`SetHostname`, `SetNTP`, media configuration updates, preset changes, etc.) and verify the camera applies changes immediately and persists them.

**Acceptance Scenarios**:

1. **Given** the operator submits `SetHostname`, `SetDNS`, or `SetNetworkProtocols` with valid values, **When** the requests complete, **Then** the runtime configuration reflects the updates and persists through a reboot.
2. **Given** the operator adjusts ONVIF users, media profiles, PTZ presets, or imaging parameters via corresponding Set operations, **When** changes are applied, **Then** subsequent GET calls return the updated values and unsupported fields return standards-compliant faults.

---

### User Story 3 - Ensure Automated Regression Coverage (Priority: P2)

QA engineers need comprehensive unit and integration test coverage for the enhanced services so that regressions are caught automatically during continuous integration without running full end-to-end certification suites.

**Why this priority**: Automated tests provide fast feedback, reduce manual effort, and are a prerequisite for future e2e validation efforts.

**Independent Test**: Execute the ONVIF unit and integration test suites introduced for this feature and verify they pass locally and in CI.

**Acceptance Scenarios**:

1. **Given** new handlers and configuration logic, **When** the associated unit tests run, **Then** they validate success and failure paths (including persistence and error handling) for each operation without flakiness.
2. **Given** integration tests replaying representative SOAP requests derived from the captures, **When** the tests execute, **Then** they confirm end-to-end request/response behavior for GET and SET operations without manual intervention.

### Edge Cases

- Configuration files contain legacy or missing values when setters run; responses must fall back to documented defaults without faults.
- Multiple administrators update overlapping settings through ONVIF within the same minute; the system must serialize updates and ensure the final persisted state is deterministic.
- A client requests optional fields we do not support (e.g., IPv6-only protocol updates); the system must return a standards-compliant fault without altering current settings.
- Tokens supplied in Media/PTZ setters do not exist; the system must respond with clear ONVIF errors and leave state unchanged.
- Network updates request DHCP disablement on an interface that currently lacks a static configuration; the system must reject the change gracefully and explain the reason.
- Clients probe for relay outputs despite the hardware lacking them; the system must advertise zero relay channels and return `ONVIF_ERROR_NOT_SUPPORTED` for relay-specific setters.

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: The camera MUST provide successful ONVIF Device service responses for all captured GET operations, including but not limited to `GetServiceCapabilities`, `GetDiscoveryMode`, `GetHostname`, `GetNetworkInterfaces`, `GetNetworkProtocols`, `GetNetworkDefaultGateway`, `GetDNS`, `GetNTP`, `GetScopes`, `GetRelayOutputs`, `GetUsers`, and `GetDeviceInformation`.
- **FR-002**: The camera MUST support the corresponding Device service setters (`SetHostname`, `SetHostnameFromDHCP`, `SetDNS`, `SetDynamicDNS`, `SetNTP`, `SetNetworkDefaultGateway`, `SetNetworkProtocols`, `SetNetworkInterfaces`, `SetDiscoveryMode`, `SetScopes`, `AddScopes`, `RemoveScopes`) with validation, persistence, and ONVIF-compliant error handling.
- **FR-003**: The system MUST allow creation, modification, and deletion of ONVIF user accounts via `CreateUsers`, `SetUser`, and `DeleteUsers`, applying password policies defined in project standards and reflecting changes in subsequent queries.
- **FR-004**: The Device service MUST declare zero relay outputs and respond to any relay-related requests (`GetRelayOutputs`, `SetRelayOutputSettings`, `SetRelayOutputState`) with standards-compliant `ONVIF_ERROR_NOT_SUPPORTED` faults without altering system state.
- **FR-005**: The Device service MUST preserve manufacturer, model, firmware, serial number, and hardware identifiers when `SetDeviceInformation` succeeds, storing updates for inclusion in later discovery responses.
- **FR-006**: The Media service MUST provide accurate data for `GetProfiles`, `GetProfile`, `GetVideoSources`, `GetAudioSources`, `GetVideoSourceConfiguration`, `GetVideoEncoderConfiguration`, `GetVideoEncoderConfigurationOptions`, `GetStreamUri`, and `GetSnapshotUri`, reflecting the active runtime configuration.
- **FR-007**: The Media service MUST support `SetVideoSourceConfiguration`, `SetVideoEncoderConfiguration`, `SetAudioSourceConfiguration`, `SetAudioEncoderConfiguration`, and `SetMetadataConfiguration`, validating tokens, limits, and cross-references before applying changes.
- **FR-008**: The Media service MUST allow profile lifecycle adjustments through `CreateProfile`, `DeleteProfile`, `AddConfiguration`, and `RemoveConfiguration`, ensuring the total number of profiles never exceeds four and that bindings remain consistent.
- **FR-009**: The PTZ service MUST expose movement and status operations `GetNode`, `GetNodes`, `GetStatus`, `ContinuousMove`, `Stop`, `SetPreset`, `RemovePreset`, `GotoPreset`, `SetHomePosition`, and `GotoHomePosition`, matching the ranges captured in the traffic.
- **FR-010**: PTZ preset and home position updates MUST persist per media profile so that subsequent reboots and queries return the last configured positions.
- **FR-011**: The Imaging service MUST provide `GetImagingSettings`, `GetOptions`, and `GetMoveOptions`, and accept updates via `SetImagingSettings` and imaging Move operations, with validation that aligns imaging ranges to hardware capabilities.
- **FR-012**: All configuration changes triggered through ONVIF setters MUST persist to the unified configuration system within two seconds and survive power cycles.
- **FR-013**: The system MUST emit ONVIF-compliant faults with descriptive reasons whenever a request contains unsupported parameters, out-of-range values, or non-existent tokens, without modifying existing settings.
- **FR-014**: The camera MUST log each successful and failed ONVIF setter invocation with context (operation, actor, outcome) to support auditing and troubleshooting.
- **FR-015**: Every supported `Get*` ONVIF endpoint in scope MUST return live data sourced from the current runtime or persisted configuration rather than placeholder or hard-coded values.
- **FR-016**: Every supported `Set*` ONVIF endpoint in scope MUST update the appropriate runtime state and ensure the new value is saved to the unified configuration or underlying system facility so the change survives reboot and is reflected in subsequent queries.
- **FR-017**: Certificate management operations (`GetCertificates`, `GetCertificatesStatus`, `CreateCertificate`, `LoadCertificate`, `DeleteCertificates`, `SetCertificatesStatus`) MUST respond with a standards-compliant `ONVIF_ERROR_NOT_SUPPORTED` fault accompanied by explanatory detail, as SSL/TLS features are out of scope for this effort.

### Key Entities *(include if feature involves data)*

- **ONVIF Service Operation**: Catalog of Device, Media, PTZ, and Imaging actions the camera supports, including expected inputs, outputs, and status codes, as well as operations explicitly not supported (e.g., relay control, certificate lifecycle) with expected fault responses.
- **Network Configuration Profile**: Represents hostname, interfaces, protocols, gateway, DNS, and NTP settings managed through ONVIF and stored in the unified configuration system.
- **ONVIF User Account**: Captures username, role, credential metadata, and activation state used for authentication and lifecycle management via ONVIF requests.
- **Media Profile Definition**: Describes logical video/audio stream profiles with tokens, encoding parameters, and bindings to source configurations.
- **PTZ Preset Collection**: Holds preset tokens, names, and absolute positions, including designated home positions per profile.
- **Imaging Settings Bundle**: Aggregates brightness, contrast, saturation, sharpening, day/night modes, and related parameters exposed to ONVIF clients.

## Assumptions & Dependencies

- Events and analytics services remain out of scope for this feature and will continue returning current behavior.
- The unified configuration system introduced in earlier work is available to persist all ONVIF-driven updates and expose current values to responders.
- Hardware capabilities (PTZ motors, relay outputs, imaging controls) match the Anyka AK3918 reference implementation; unsupported hardware variations will be handled separately.
- Access to the captured SOAP traffic in `cap/` and the existing unit/integration test infrastructure (CMocka suites, integration harness) is maintained for validation.

## Success Criteria *(mandatory)*

### Measurable Outcomes

- **SC-001**: 100% of captured ONVIF GET operations (excluding events and analytics) return schema-valid success responses with population parity against the recorded payloads during regression testing.
- **SC-002**: 95% of ONVIF configuration updates complete and persist within two seconds, and no accepted update is lost across a reboot in QA soak tests.
- **SC-003**: Automated unit and integration test suites covering the new and updated ONVIF operations run within CI and pass consistently with zero regressions introduced by this feature.
- **SC-004**: Field trial feedback indicates at least 90% of installers can provision devices end-to-end using only ONVIF clients, without resorting to device-specific fallbacks, during acceptance testing.

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
4. **Given** an ONVIF client wants to save a preset, **When** it sends a SetPreset request with a preset token, **Then** the system saves the current camera position for later recall
5. **Given** an ONVIF client wants to recall a preset, **When** it sends a GotoPreset request with a saved preset token, **Then** the system moves the camera to the saved position

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

### Edge Cases

- What happens when an ONVIF client sends a request with an invalid operation name?
- How does the system handle malformed SOAP XML requests?
- What happens when a client requests a profile token that doesn't exist?
- How does the system handle concurrent requests to the same service endpoint?
- What happens when a PTZ command is sent but the camera hardware is not available?
- How does the system handle requests with missing required parameters?
- What happens when a client sends a request to a service endpoint that isn't implemented?
- How does the system handle extremely large SOAP request payloads?
- What happens when multiple clients simultaneously modify imaging settings?
- How does the system handle timezone information in GetSystemDateAndTime when system time is unavailable?

## Requirements *(mandatory)*

### Functional Requirements

- **FR-001**: System MUST implement ONVIF Device Service with all operations covered in the Base Device Test Specification (including but not limited to: GetDeviceInformation, GetCapabilities, GetSystemDateAndTime, GetServices, SystemReboot, and all other operations defined in the test specification)
- **FR-002**: System MUST implement ONVIF Media Service with all operations covered in the Media Device Test Specification (including but not limited to: GetProfiles, GetStreamUri, GetVideoSources, GetAudioSources, GetVideoSourceConfigurations, GetVideoEncoderConfigurations, and all other operations defined in the test specification)
- **FR-003**: System MUST implement ONVIF PTZ Service with all operations covered in the PTZ Device Test Specification (including but not limited to: GetConfigurations, AbsoluteMove, RelativeMove, ContinuousMove, Stop, GetPresets, SetPreset, GotoPreset, and all other operations defined in the test specification)
- **FR-004**: System MUST implement ONVIF Imaging Service with all operations covered in the Imaging Device Test Specification (including but not limited to: GetImagingSettings, SetImagingSettings, GetOptions, and all other operations defined in the test specification)
- **FR-005**: System MUST accept and process SOAP XML requests over HTTP according to ONVIF 24.12 specification
- **FR-006**: System MUST generate valid ONVIF-compliant SOAP XML responses for all implemented operations
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

### Key Entities *(include if feature involves data)*

- **ONVIF Service**: Represents an ONVIF-compliant web service (Device, Media, PTZ, Imaging) with namespace URI, XAddr endpoint, and version information
- **Service Operation**: Represents a single ONVIF operation (e.g., GetDeviceInformation) that accepts SOAP requests and returns SOAP responses
- **Media Profile**: Represents a configured media stream profile containing video source, video encoder, audio source, and audio encoder configurations with a unique token identifier
- **PTZ Preset**: Represents a saved camera position with pan, tilt, and zoom coordinates identified by a unique preset token
- **Imaging Settings**: Represents current image quality parameters including brightness, contrast, saturation, sharpness, hue, and day/night mode configuration
- **Service Capabilities**: Represents the set of features and operations supported by each ONVIF service, used to inform clients of available functionality

## Clarifications

### Session 2025-01-27

- Q: What ONVIF specification version should be enforced? → A: ONVIF 24.12 (updated from 2.5 per user requirement)
- Q: Which ONVIF Device Test Specifications should serve as the primary basis for implementation? → A: Base Device, Media, PTZ, and Imaging test specifications (all four core services)
- Q: What is the scope of operations to implement from the test specifications? → A: All operations covered in the Base Device, Media, PTZ, and Imaging test specifications (comprehensive coverage)
- Q: What level of test case conformance should the implementation target? → A: Pass all applicable test cases from the referenced test specifications (full conformance validation)
- Q: How should we handle operations in the test specifications that weren't in the original C implementation or have hardware dependencies? → A: Implement all operations from test specs, use platform abstraction stubs for hardware-dependent operations

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

# Requirements Document

## Introduction

Enhance the ONVIF media pipeline to deliver production-ready video and audio streaming by completing the RTSP stack, aligning Media service responses with Profile S/T expectations, and wiring the implementation to Anyka AK3918 hardware blocks. The feature delivers dependable dual-stream video, synchronized audio, and standards-compliant configuration surfaces so integrators can ship the custom firmware without gaps in streaming functionality.

## Alignment with Product Vision

This work eliminates remaining interoperability gaps, strengthening ONVIF 2.5 compliance and enabling professional surveillance deployments on affordable Anyka hardware. It advances the product goals of standards adherence, hardware compatibility, and developer trust by matching the vendor reference behaviour while keeping the system fully open and observable.

## Requirements

### Requirement 1

**User Story:** As a security system integrator, I want ONVIF clients to retrieve stream URIs and media capabilities that exactly match Profile S/T expectations, so that my VMS can automatically configure camera video feeds without manual tweaks.

#### Acceptance Criteria

1. WHEN an ONVIF client issues `GetProfiles`, `GetVideoEncoderConfigurationOptions`, or `GetStreamUri` for any advertised profile THEN the service SHALL return parameters (codec, resolution, frame rate, protocol, transport, URI) that are consistent with the AK3918 capabilities and the values exposed by the Anyka reference implementation.
2. IF a client requests RTSP URLs using transport preferences defined in Profiles S or T (unicast/multicast over UDP or UDP-multicast) THEN the Media service SHALL either supply a compliant URI or respond with an ONVIF error detailing unsupported transports, and SHALL explicitly reject RTPS/RTSPS or other secure transport requests with `ONVIF_ERROR_UNSUPPORTED`.
3. WHEN a client negotiates Profile T specific settings (H.265, dynamic GOP, HDR metadata flags) THEN the system SHALL respond with supported/unsupported flags per capability set and never advertise options the pipeline cannot honour.

### Requirement 2

**User Story:** As a VMS operator, I want synchronized audio tracks exposed alongside video, so that monitoring stations can capture and play two-way evidence from the camera.

#### Acceptance Criteria

1. WHEN a client requests `GetAudioSources`, `GetAudioEncoderConfigurations`, or `GetAudioOutputs` THEN the Media service SHALL surface accurate channel count, sample rate, and codec support derived from the audio pipeline (AAC-LC and/or G.711) and match SDP announcements.
2. IF audio streaming is enabled for a profile THEN the generated SDP in the RTSP DESCRIBE response SHALL include a usable audio track with correct payload type, sampling rate, and control attributes that succeed in the Anyka reference player and ONVIF Profile S conformance tests.
3. WHEN audio is disabled via ONVIF configuration or missing from hardware THEN the system SHALL omit the audio track from SDP and return deterministic ONVIF faults (e.g., `ONVIF_ERROR_UNSUPPORTED`) without crashing the RTSP session handler.

### Requirement 3

**User Story:** As a network administrator, I want the RTSP server to manage sessions reliably under load, so that monitoring software can maintain continuous streams without manual intervention.

#### Acceptance Criteria

1. WHEN multiple ONVIF clients connect concurrently (minimum two main-stream, one sub-stream) THEN the RTSP server SHALL maintain separate session state, enforce digest authentication, and keep stream latency within Profile S tolerances (<500 ms jitter on LAN, measured against reference implementation).
2. WHEN clients send keep-alive requests (OPTIONS/GET_PARAMETER) or RTCP reports THEN the RTSP server SHALL respond per RFC 2326/3550, updating internal timers and tearing down idle sessions after configurable timeouts that match the Media service responses.
3. IF a network interruption occurs mid-stream THEN the server SHALL release Anyka encoder resources within the timeout window and allow subsequent PLAY requests without requiring a process restart.

### Requirement 4

**User Story:** As a firmware developer, I want automated validation for the enhanced media pipeline, so that regressions are caught before flashing updated images.

#### Acceptance Criteria

1. WHEN running `make test` or the dedicated media service suite THEN new unit tests SHALL cover Media service validation logic, RTSP SDP generation, and audio/video capability parsing with both success and failure paths.
2. IF integration tests run against the SD-card hack environment THEN scripts SHALL verify RTSP DESCRIBE/SETUP/PLAY exchanges, RTP payload correctness, and digest authentication using captured fixtures from the Anyka reference firmware.
3. WHEN documentation is generated via `doxygen Doxyfile` THEN updated module overviews SHALL describe the finished media/RTSP workflow, including how to enable Profile T options and configure dual-stream output.

### Requirement 5

**User Story:** As a platform maintainer, I want the media pipeline to reuse the existing hardware encoders without duplicating video or audio encoding work, so that future web streaming features can share the same streams without triggering costly re-encoding.

#### Acceptance Criteria

1. WHEN RTSP main or sub-stream sessions are started THEN the system SHALL attach to the active Anyka encoder outputs and avoid spawning additional encoder instances or software transcoders.
2. IF a second consumer (e.g., forthcoming web streaming service) subscribes to an already active stream THEN the pipeline SHALL reuse the hardware-generated buffers via shared memory or tee mechanisms, maintaining a single encoder per stream profile.
3. WHEN diagnostic logging is enabled THEN the service SHALL report encoder handle reuse and flag any unexpected encoder duplication as an error-level event for operators.

## Non-Functional Requirements

### Code Architecture and Modularity
- Preserve separation between Media service logic, RTSP transport, and hardware adapters; introduce new helpers in `src/utils/stream/` when behaviour is reusable.
- Ensure configuration flows through existing `core/config` APIs rather than adding hard-coded constants.
- Mirror Anyka reference state machines in isolated adapters to simplify verification and future hardware swaps.

### Performance
- Maintain 1080p@30fps main-stream and 720p@25fps sub-stream throughput with <65% encoder CPU utilisation.
- Keep RTSP session setup latency under 250 ms for first frame delivery on a gigabit LAN.

### Security
- Reuse centralized authentication and credential storage, enforcing ONVIF digest authentication for RTSP DESCRIBE/SETUP/PLAY.
- Reject RTPS/RTSPS or other TLS-backed transport requests deterministically with `ONVIF_ERROR_UNSUPPORTED`; do not introduce SSL/TLS dependencies.

### Reliability
- Guarantee graceful recovery within 5 seconds after encoder or network faults, with automatic session reinitialisation.
- Provide runtime metrics/logging hooks so watchdog scripts can detect stalled streams.

### Usability
- Document configuration toggles and supported media profiles in the SD-card web UI and project README updates.
- Supply troubleshooting guidance for common RTSP/ONVIF client interoperability issues surfaced during conformance testing.

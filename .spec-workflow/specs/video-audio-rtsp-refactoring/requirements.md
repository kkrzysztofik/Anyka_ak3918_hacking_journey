# Requirements Document

## Introduction

The video-audio-rtsp-refactoring initiative restructures the ONVIF RTSP pipeline to achieve end-to-end compliance with RFC 2326 while aligning video and audio transport with the camera's embedded constraints. The effort establishes predictable RTSP state transitions, robust media negotiation, and interoperable SDP descriptions so that third-party NVRs and compliant clients can rely on consistent behaviour.

## Alignment with Product Vision

This feature strengthens the project's goal of delivering professional-grade, standards-compliant firmware by ensuring that every RTSP interaction matches the expectations of security integrators and ONVIF tooling. Full RFC 2326 adherence eliminates interoperability gaps, improves reliability for mission-critical surveillance deployments, and reinforces the project's commitment to open, standards-based control surfaces.

## Requirements

### Requirement 1

**User Story:** As a security system integrator, I want the camera's RTSP server to support the complete RFC 2326 request lifecycle so that any compliant client can establish, control, and terminate sessions without custom workarounds.

#### Acceptance Criteria

1. WHEN a client issues DESCRIBE, SETUP, PLAY, PAUSE, and TEARDOWN requests in the sequences defined by RFC 2326 sections 6 and 11 THEN the server SHALL respond with the correct status codes, CSeq handling, and required headers (Date, Session, Transport, Range) for each state transition.
2. IF a client submits conditional requests (e.g., Range headers, aggregate control URIs) THEN the server SHALL enforce the session state machine described in RFC 2326 section 10, refusing unsupported transitions with 455 Method Not Valid In This State responses.
3. WHEN multiple SETUP requests are received for separate media tracks AND the client includes the aggregate control URI in subsequent PLAY/PAUSE requests THEN the server SHALL coordinate track state in accordance with RFC 2326 section 12, ensuring synchronized start and pause operations.

### Requirement 2

**User Story:** As an ONVIF media client, I want accurate and standards-compliant SDP and transport negotiation so that I can subscribe to audio and video streams using RTP over UDP or interleaved TCP without manual configuration.

#### Acceptance Criteria

1. WHEN the server returns SDP bodies for DESCRIBE requests THEN the SDP SHALL include valid session-level and media-level attributes (v=0, o=, s=, t=, a=control, a=rtpmap, a=fmtp) that match the negotiated codecs and comply with RFC 2327 and RFC 2326 section 10.5.
2. IF the client requests RTP/AVP over UDP unicast or multicast using Transport headers THEN the server SHALL allocate ports, confirm SSRC values, and respond with Transport headers meeting RFC 2326 section 12.37 formatting requirements.
3. WHEN the client requests RTP/AVP/TCP interleaved transport THEN the server SHALL honour the interleaved channel negotiation, embed RTP and RTCP over the RTSP TCP connection, and acknowledge the setup with properly formed Transport headers.

### Requirement 3

**User Story:** As a surveillance operator, I want audio and video streams to remain synchronized and recover gracefully from network impairments so that recorded footage maintains evidentiary quality.

#### Acceptance Criteria

1. WHEN RTP packets are transmitted for audio and video tracks THEN the server SHALL maintain monotonically increasing RTP timestamps and sequence numbers that align with the clock rates specified in the SDP, enabling client-side jitter buffer alignment.
2. IF packet loss or late packets are detected THEN the server SHALL continue transmitting without session termination and SHALL provide RTCP Sender Reports at RFC 3550 interval guidelines so that clients can resynchronize streams.
3. WHEN the client issues GET_PARAMETER keep-alive requests or RTCP Receiver Reports indicate extended jitter THEN the server SHALL maintain the session and adjust scheduling without violating the RTSP state machine.

### Requirement 4

**User Story:** As a security administrator, I want RTSP authentication, authorization, and session expiry to align with RFC 2326 and ONVIF security requirements so that unauthorized access is prevented without interrupting legitimate sessions.

#### Acceptance Criteria

1. WHEN a protected resource is requested without valid credentials THEN the server SHALL respond with 401 Unauthorized and include WWW-Authenticate headers supporting Digest authentication per RFC 2617, as referenced by RFC 2326 section 15.1.
2. IF the client authenticates successfully THEN the server SHALL bind the credentials to the RTSP session identifier, enforce idle and absolute timeouts (as configured in media service policy), and terminate expired sessions with 454 Session Not Found.
3. WHEN authorization policies restrict media tracks or methods THEN the server SHALL respond with 403 Forbidden while maintaining RFC 2326-compliant headers and logging the denial via the platform logging subsystem.

### Requirement 5

**User Story:** As a QA engineer, I want comprehensive error reporting, capability negotiation, and observability so that I can certify RTSP interoperability during regression testing.

#### Acceptance Criteria

1. WHEN the server cannot meet a client's transport or media request THEN it SHALL return RFC 2326-compliant error codes (e.g., 461 Unsupported Transport, 415 Unsupported Media Type) along with Allow and Public headers enumerating supported methods.
2. IF the server encounters malformed requests or invalid headers THEN it SHALL reject the request with 400 Bad Request, emit structured diagnostics through the ONVIF logging utilities, and continue serving other clients.
3. WHEN diagnostic endpoints or ONVIF GetStreamUri operations are invoked THEN the system SHALL expose supported transports, codecs, control URIs, and keep-alive expectations consistent with the RTSP configuration.

## Non-Functional Requirements

### Code Architecture and Modularity
- The refactoring SHALL isolate RTSP parsing, session state management, transport handling, and RTP pipeline coordination into dedicated modules under `src/networking/rtsp/`, reusing shared utilities from `src/utils/` without duplicating functionality.
- Interfaces between RTSP control logic and media encoders SHALL be documented using Doxygen and expose stable contracts consumable by ONVIF media services.
- The solution SHALL use predefined error constants and maintain existing include order, global naming, and formatting standards.

### Performance
- The RTSP server SHALL sustain at least three concurrent clients streaming 1080p video with AAC audio while keeping end-to-end setup latency below 500 ms and steady-state RTP jitter below 50 ms on typical LAN conditions.
- Session setup and teardown operations SHALL complete within two round-trip times for compliant clients under normal network loads.

### Security
- All RTSP inputs SHALL be validated for length, character set, and header semantics before processing to prevent buffer overflows and injection attacks.
- Authentication workflows SHALL rely on the existing digest authentication utilities, store credentials securely, and log failed attempts without leaking sensitive information.
- Transport selection SHALL default to secure profiles (TLS or SRTP) when enabled by configuration, and fallback behaviour SHALL require explicit opt-in.

### Reliability
- The server SHALL recover from encoder restarts or network interface resets without leaking sessions or file descriptors, and SHALL re-register multicast streams automatically when interfaces bounce.
- Automated unit and integration tests SHALL verify RTSP method handling, SDP generation, transport negotiation, and error cases with coverage thresholds matching the existing onvif/tests mandate.

### Usability
- Configuration knobs for RTSP behaviour (timeouts, transport preferences, keep-alive interval, authentication realm) SHALL be exposed through the existing ONVIF configuration interfaces and documented in the administrator guide.
- Generated SDP and RTSP responses SHALL include human-readable logging tags to simplify troubleshooting during SD-card testing and field deployment.

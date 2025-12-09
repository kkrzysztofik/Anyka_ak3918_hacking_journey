# Functional Requirements

This document outlines the functional requirements for the `onvif-rust` project.

## General Requirements

- **FR-001**: System MUST implement ONVIF Device Service (Profile S/T).
- **FR-002**: System MUST implement ONVIF Media Service.
- **FR-003**: System MUST implement ONVIF PTZ Service.
- **FR-004**: System MUST implement ONVIF Imaging Service.
- **FR-005**: System MUST accept/process SOAP 1.2 requests over HTTP/1.1.
- **FR-006**: System MUST generate valid ONVIF-compliant SOAP responses.
- **FR-007**: System MUST return standard ONVIF Faults for errors.

## Device Service

- **FR-DEV-01**: `GetDeviceInformation` (Manufacturer, Model, Serial, Firmware).
- **FR-DEV-02**: `GetSystemDateAndTime` (NTP/Manual).
- **FR-DEV-03**: `GetUsers` / `CreateUsers` / `SetUser` (WS-UsernameToken).
- **FR-DEV-04**: `GetNetworkInterfaces` (IP, MAC, DHCP status).
- **FR-DEV-05**: `SystemReboot`.

## Media Service

- **FR-MED-01**: `GetProfiles` (Fixed profiles: MainStream, SubStream).
- **FR-MED-02**: `GetVideoSourceConfigurations`.
- **FR-MED-03**: `GetVideoEncoderConfigurations` (H.264/H.265).
- **FR-MED-04**: `GetStreamUri` (RTSP).
- **FR-MED-05**: `GetSnapshotUri` (JPEG).

## PTZ Service

- **FR-PTZ-01**: `ContinuousMove` (Velocity-based).
- **FR-PTZ-02**: `AbsoluteMove` (Position-based).
- **FR-PTZ-03**: `Stop`.
- **FR-PTZ-04**: `GetPresets` / `SetPreset` / `GotoPreset` / `RemovePreset`.

## Imaging Service

- **FR-IMG-01**: `GetImagingSettings` (Brightness, Contrast, Saturation, Sharpness).
- **FR-IMG-02**: `SetImagingSettings`.
- **FR-IMG-03**: `GetOptions` (Valid ranges for settings).

## Security & Performance

- **FR-SEC-01**: Validate WS-UsernameToken (Digest & PasswordText).
- **FR-PERF-01**: Handle concurrent requests without crashing.
- **FR-PERF-02**: Enforce memory limits (Graceful degradation > 24MB).

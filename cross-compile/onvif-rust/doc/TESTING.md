# Testing Strategy

This project employs a multi-layered testing strategy to ensure correctness, conformance with the ONVIF specification, and robustness on the target hardware.

## 1. Unit Tests

Unit tests focus on individual functions and modules in isolation. They are located within the same file as the code they are testing, under a `#[cfg(test)]` module.

**Goal**: Verify logic without external dependencies (network, hardware).

```bash
cargo test
```

## 2. Integration Tests

Integration tests are located in the `tests/` directory. They test the interaction between components, specifically:

- **SOAP Parsing**: Verifying that XML requests are parsed correctly into Rust structs.
- **SOAP Serialization**: Verifying that Rust structs are serialized into valid ONVIF XML.
- **Service Logic**: Testing the full request/response cycle for services (Device, Media, etc.).

## 3. User Scenarios (Acceptance Tests)

We validate the system against real-world usage scenarios derived from ONVIF specifications.

### Device Service

- **GetDeviceInformation**: Client requests manufacturer, model, firmware version.
- **GetCapabilities**: Client queries supported services (PTZ, Media, Imaging).

### Media Service

- **GetProfiles**: Client requests available video profiles (MainStream, SubStream).
- **GetStreamUri**: Client requests RTSP URI for a specific profile.

### PTZ Service

- **AbsoluteMove**: Client sends Pan/Tilt/Zoom coordinates.
- **Stop**: Client stops movement.
- **SetPreset**: Client saves current position.

### Imaging Service

- **GetImagingSettings**: Client retrieves Brightness, Contrast, Exposure.
- **SetImagingSettings**: Client updates image parameters.

## 4. Platform Abstraction for Testing

To enable testing without physical hardware, we use a **Platform Abstraction Layer (PAL)**.

- **`Platform` Trait**: Defines hardware interfaces.
- **`StubPlatform`**: A mock implementation used during `cargo test`.
- **`AnykaPlatform`**: The real implementation using FFI/Syscalls (only compiled for target).

## 5. Conformance Testing

The ultimate goal is passing the **ONVIF Device Test Tool**.

### ONVIF Device Test Tool

The **ONVIF Device Test Tool** is an official utility developed by ONVIF for testing conformance of IP-based physical security products to ONVIF standards.

**Important**: The test tool is **only available to ONVIF members**. Access is restricted to member companies for use in declaring product compliance.

**How to Obtain**:

- Join ONVIF as a member company (see [onvif.org](https://www.onvif.org) for membership information)
- Access the test tool through the ONVIF member portal
- Questions about the test tool can be asked in the [ONVIF Test Specifications Discussions](https://github.com/onvif/testspecs/discussions)

**Test Specifications** (Publicly Available):

- [Device Test Specifications](https://www.onvif.org/profiles/conformance/device-test-2/) - Public test specifications that define the test cases
- [Client Test Specifications](https://www.onvif.org/profiles/conformance/client-test/) - For client implementation testing

### Conformance Testing Process

1. Build the release binary.
2. Deploy to the camera (or a precise emulator).
3. Run the official ONVIF Device Test Tool against the device IP (requires ONVIF membership).
4. Verify all mandatory tests for Profile S/T pass.

**Alternative for Non-Members**:

- Use the publicly available [Device Test Specifications](https://www.onvif.org/profiles/conformance/device-test-2/) to implement manual test cases
- Use third-party ONVIF client tools for basic conformance validation
- Refer to the project's integration tests in `tests/onvif/` which are based on the test specifications

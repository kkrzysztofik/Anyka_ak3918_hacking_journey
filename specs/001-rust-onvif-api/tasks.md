# Tasks: Rust ONVIF API Rewrite

**Input**: `/home/kmk/anyka-dev/specs/001-rust-onvif-api/plan.md`, `/home/kmk/anyka-dev/specs/001-rust-onvif-api/spec.md`
**Prerequisites**: plan.md (required), spec.md (required for user stories)

---

## Phase 1: Setup & CI/CD

**Purpose**: Bootstrap the Rust ONVIF workspace, toolchain, cross-compilation assets, and CI/CD infrastructure from the implementation plan.

### Workspace & Build Setup

- [X] T001 Create the Rust workspace directory structure at `/home/kmk/anyka-dev/cross-compile/onvif-rust/` with subdirectories: `src/`, `tests/`, `scripts/`, `docs/`.
- [X] T002 Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/main.rs` with minimal async entry point using tokio runtime.
- [X] T003 Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/lib.rs` as library root with module declarations.
- [X] T004 Initialize `/home/kmk/anyka-dev/cross-compile/onvif-rust/Cargo.toml` with all dependencies from plan.md: tokio, axum, tower, tower-http, quick-xml, toml, serde, tracing, tracing-subscriber, tracing-log, argon2, sha1, md-5, hmac, base64, rand, constant_time_eq, socket2, chrono, uuid, parking_lot, dashmap, bytes, libc, anyhow, thiserror.
- [X] T005 Add dev-dependencies to Cargo.toml: tokio-test, mockall, reqwest, wiremock, criterion.
- [X] T006 Add build-dependencies to Cargo.toml: cc, bindgen.
- [X] T007 Configure Cargo.toml features: `default = []`, `memory-profiling = []`, `verbose-logging = []`.
- [X] T008 Configure Cargo.toml release profile with `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `strip = true`.
- [X] T009 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/.cargo/config.toml` with cross-compilation settings: target `armv5te-unknown-linux-uclibceabi`, linker path, rustflags for ARM architecture.
- [X] T010 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/arm-unknown-linux-uclibcgnueabi.json` custom target specification with ARMv5TE parameters, soft-float ABI, uClibc settings.

### CI/CD Infrastructure

- [X] T011 [P] Create `/home/kmk/anyka-dev/.github/workflows/ci.yml` GitHub Actions workflow for CI: triggers on push/PR to `001-rust-onvif-api` branch.
- [X] T012 [P] Add CI job `test` to run `cargo test --all-features` with Rust stable toolchain.
- [X] T013 [P] Add CI job step for `cargo clippy -- -D warnings` static analysis.
- [X] T014 [P] Add CI job step for `cargo fmt --check` formatting verification.
- [X] T015 [P] Add CI job `coverage` with cargo-tarpaulin generating HTML, LCOV, and Cobertura XML reports.
- [X] T016 [P] Configure CI job to upload coverage artifacts: `coverage-html`, `coverage-lcov`, `coverage-cobertura`.
- [X] T017 [P] Add cargo caching to CI workflow using `actions/cache@v4` for `~/.cargo/registry`, `~/.cargo/git`, `target/`.
- [X] T018 [P] Create `/home/kmk/anyka-dev/.github/workflows/release.yml` GitHub Actions workflow for releases: triggers on git tags `v*`.
- [X] T019 [P] Configure release workflow to use Docker container with ARM cross-compilation toolchain.
- [X] T020 [P] Add release job step to cross-compile for `armv5te-unknown-linux-uclibceabi` target.
- [X] T021 [P] Add release job step to create SD_card_content directory structure and copy binary.
- [X] T022 [P] Add release job step to create ZIP archive and upload as GitHub Release artifact.
- [X] T022a Verify CI workflow executes successfully with placeholder test (empty test suite passes).
- [X] T022b Verify release workflow configuration is valid (dry-run tag push validation).

**Checkpoint**: Workspace compiles, CI/CD pipelines operational.

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Implement the platform abstraction, configuration, logging, and SOAP server scaffolding required by every user story.

### FFI Bindings to Anyka SDK

- [X] T023 Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/build.rs` build script skeleton with bindgen configuration.
- [X] T024 [P] Add bindgen generation for `ak_common.h` header (common types and error codes) in build.rs.
- [X] T025 [P] Add bindgen generation for `ak_vi.h` header (video input) in build.rs.
- [X] T026 [P] Add bindgen generation for `ak_venc.h` header (video encoder) in build.rs.
- [X] T027 [P] Add bindgen generation for `ak_ai.h` header (audio input) in build.rs.
- [X] T028 [P] Add bindgen generation for `ak_aenc.h` header (audio encoder) in build.rs.
- [X] T029 [P] Add bindgen generation for `ak_drv_ptz.h` header (PTZ control) in build.rs.
- [X] T030 [P] Add bindgen generation for `ak_vpss.h` header (video processing) in build.rs.
- [X] T031 [P] Add bindgen generation for `ak_drv_irled.h` header (IR LED control) in build.rs.
- [X] T032 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/ffi/mod.rs` with module exports for generated bindings.
- [X] T033 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/ffi/anyka_sdk.rs` to host generated bindings with safe wrapper functions.

### Platform Abstraction Traits

- [X] T034 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/platform/mod.rs` with module declarations for traits, stubs, and anyka submodule.
- [X] T035 [P] Define `VideoInput` trait in `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/platform/traits.rs` with methods: `open()`, `close()`, `get_resolution()`.
- [X] T036 [P] Define `VideoEncoder` trait in traits.rs with methods: `init()`, `get_stream()`, `get_configuration()`.
- [X] T037 [P] Define `AudioInput` trait in traits.rs with methods: `open()`, `close()`, `get_configuration()`.
- [X] T038 [P] Define `AudioEncoder` trait in traits.rs with methods: `init()`, `get_stream()`, `get_configuration()`.
- [X] T039 [P] Define `PTZControl` trait in traits.rs with methods: `move_to_position()`, `get_position()`, `continuous_move()`, `stop()`, `get_presets()`, `set_preset()`, `goto_preset()`, `remove_preset()`.
- [X] T040 [P] Define `ImagingControl` trait in traits.rs with methods: `get_settings()`, `set_settings()`, `get_options()`, `set_brightness()`, `set_contrast()`, `set_saturation()`, `set_sharpness()`.
- [X] T041 [P] Define `Platform` super-trait in traits.rs combining all hardware traits with `get_device_info()` method.
- [X] T042 [P] Add `#[mockall::automock]` attribute to all platform traits for test mocking support.

### Hardware Stubs

- [X] T043 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/platform/stubs.rs` with `StubPlatform` struct implementing `Platform` trait.
- [X] T044 [P] Implement `StubVideoInput` in stubs.rs returning configurable mock resolution data.
- [X] T045 [P] Implement `StubVideoEncoder` in stubs.rs returning configurable mock encoder configurations.
- [X] T046 [P] Implement `StubAudioInput` in stubs.rs returning configurable mock audio configurations.
- [X] T047 [P] Implement `StubAudioEncoder` in stubs.rs returning configurable mock audio encoder configurations.
- [X] T048 [P] Implement `StubPTZControl` in stubs.rs with in-memory position state and preset storage.
- [X] T049 [P] Implement `StubImagingControl` in stubs.rs with in-memory imaging settings state.
- [X] T050 [P] Add `StubPlatformBuilder` for configuring stub behavior (success/failure scenarios) in tests.

### Configuration System

- [X] T051 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/config/mod.rs` with module declarations for schema, runtime, storage.
- [X] T052 [P] Define `ConfigValueType` enum in `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/config/schema.rs` with variants: Int, String, Bool, Float.
- [X] T053 [P] Define `ConfigParameter` struct in schema.rs with fields: key, value_type, default, min, max, required.
- [X] T054 [P] Define `ConfigSection` struct in schema.rs with fields: name, parameters.
- [X] T055 [P] Define `ConfigSchema` struct in schema.rs with method to validate configuration against schema.
- [X] T056 [P] Define configuration sections in schema.rs: `[onvif]`, `[network]`, `[device]`, `[server]`, `[logging]`, `[media]`, `[ptz]`, `[imaging]`.
- [X] T057 [P] Define `[stream_profile_N]` sections (1-4) in schema.rs with resolution, framerate, bitrate parameters.
- [X] T058 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/config/runtime.rs` with `ConfigRuntime` struct containing `Arc<RwLock<ApplicationConfig>>`.
- [X] T059 [P] Implement thread-safe `get_int()`, `set_int()` methods in ConfigRuntime.
- [X] T060 [P] Implement thread-safe `get_string()`, `set_string()` methods in ConfigRuntime.
- [X] T061 [P] Implement thread-safe `get_bool()`, `set_bool()` methods in ConfigRuntime.
- [X] T062 [P] Implement `generation` counter in ConfigRuntime for change detection using `AtomicU64`.
- [X] T063 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/config/storage.rs` with TOML file loading using `toml` crate.
- [X] T064 [P] Implement atomic TOML file saving in storage.rs (temp file + rename pattern).
- [X] T065 [P] Implement default value fallback when loading configuration in storage.rs.

### Logging System

- [X] T066 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/logging/mod.rs` with `init_logging()` function.
- [X] T067 [P] Configure tracing-subscriber with env-filter support in init_logging().
- [X] T068 [P] Initialize tracing-log bridge (`LogTracer`) to capture `log` crate messages from dependencies.
- [X] T069 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/logging/platform.rs` with `ak_print_wrapper` FFI function.
- [X] T070 [P] Implement C string to Rust string conversion in ak_print_wrapper with level mapping (ERROR, WARNING, INFO, DEBUG).
- [X] T071 [P] Add configurable log level filtering based on `[logging]` configuration section.

### Application Lifecycle

- [X] T071a [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/app.rs` with `Application` struct definition.
- [X] T071b [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/lifecycle/mod.rs` with module exports.
- [X] T071c [P] Define `StartupError` enum in lifecycle/mod.rs with variants for each initialization phase.
- [X] T071d [P] Define `ShutdownReport` struct with timing, errors, and component status.
- [X] T071e [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/lifecycle/startup.rs` with startup sequence.
- [X] T071f [P] Implement `Application::start()` with ordered initialization: Config → Platform → Services → Network.
- [X] T071g [P] Add startup logging at each phase for debugging.
- [X] T071h [P] Implement optional service initialization with degraded mode fallback.
- [X] T071i [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/lifecycle/shutdown.rs` with shutdown coordinator.
- [X] T071j [P] Implement `ShutdownCoordinator` with broadcast channel for graceful shutdown.
- [X] T071k [P] Implement `Application::shutdown()` with reverse-order cleanup and timeout handling.
- [X] T071l [P] Add shutdown logging and timing metrics.
- [X] T071m [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/lifecycle/health.rs` with health status types.
- [X] T071n [P] Implement `Application::health()` returning component status and degraded services.
- [X] T071o [P] Implement `Application::run()` with signal handling (SIGINT, SIGTERM) and main event loop.
- [X] T071p [P] Add unit tests for startup sequence in isolation (mock platform).
- [X] T071q [P] Add unit tests for shutdown coordination with timeout scenarios.
- [X] T071r [P] Add integration test for full Application lifecycle (start → run briefly → shutdown).

### HTTP Request/Response Logging (NFR-011, NFR-012)

- [X] T552 [P] Implement HTTP request/response logging middleware in `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/server.rs` (or `src/logging/http.rs`) that logs method, path, status code, latency, and optionally truncated, sanitized SOAP payloads when `[logging] http_verbose = true` (implements NFR-012). **Implemented in `src/logging/http.rs` with `HttpLoggingMiddleware` using Tower Layer pattern. Includes password/token sanitization and body truncation.**
- [X] T553 [P] Add integration tests in `/home/kmk/anyka-dev/cross-compile/onvif-rust/tests/integration/logging_http.rs` verifying that verbose mode emits HTTP request/response metadata and that disabling `[logging] http_verbose` suppresses HTTP logging. **Implemented in `tests/logging_http.rs` with 8 integration tests covering verbose/non-verbose modes, password sanitization, error responses, body truncation, and response preservation.**

### WSDL-Based Type Generation

**WSDL Source Files** (located in `/home/kmk/anyka-dev/cross-compile/onvif/wsdl/`):
- `devicemgmt.wsdl` - Device Service operations and types
- `media.wsdl` - Media Service operations and types
- `ptz.wsdl` - PTZ Service operations and types
- `imaging.wsdl` - Imaging Service operations and types
- `onvif.xsd` - Common ONVIF schema types
- `common.xsd` - Common XSD types

**Requirement**: All ONVIF request/response structs MUST be generated from these WSDL/XSD files to ensure protocol compliance.

- [X] T072a [P] Evaluate and select WSDL-to-Rust code generation approach: Option A: `yaserde` with manual struct definitions validated against WSDL; Option B: Build script using `quick-xml` to parse WSDL and generate structs; Option C: Use `xsd-parser-rs` or similar XSD-to-Rust tool. **Decision: Hand-crafted types using `serde` + `quick-xml`, static and checked-in (Option A variant).**
- [N/A] T072b [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/build.rs` additions (or separate `codegen/` module) to generate Rust structs from WSDL files at build time or via script. **Skipped: Using hand-crafted static types instead.**
- [X] T072c [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/types/mod.rs` as the root module for all WSDL-derived types. **Note: Renamed from `generated/` to `types/` per design decision.**
- [X] T072d [P] Create Device Service types from `devicemgmt.wsdl` in `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/types/device.rs`.
- [X] T072e [P] Create Media Service types from `media.wsdl` in `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/types/media.rs`.
- [X] T072f [P] Create PTZ Service types from `ptz.wsdl` in `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/types/ptz.rs`.
- [X] T072g [P] Create Imaging Service types from `imaging.wsdl` in `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/types/imaging.rs`.
- [X] T072h [P] Create common ONVIF types from `onvif.xsd` in `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/types/common.rs`.
- [X] T072i [P] Add unit tests validating types can serialize/deserialize sample SOAP payloads from `/home/kmk/anyka-dev/cap/requests/` and `/home/kmk/anyka-dev/cap/responses/`. **Implemented in `tests/types_serialization.rs` with 28 tests covering Device, Media, PTZ, Imaging types, enumerations, edge cases, and roundtrip serialization.**
- [N/A] T072j [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/scripts/regenerate_types.sh` script to regenerate types when WSDL files change. **Skipped: Types are hand-crafted and static.**

### HTTP/SOAP Server Foundation

- [X] T072 Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/mod.rs` with module declarations for server, dispatcher, soap, error, device, media, ptz, imaging, generated.
- [X] T073 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/server.rs` with `OnvifServer` struct containing Router, ConfigRuntime, Platform references.
- [X] T074 [P] Implement `OnvifServer::new()` constructor in server.rs.
- [X] T075 [P] Implement `OnvifServer::start()` async method with tokio TcpListener binding.
- [X] T076 [P] Configure axum Router with service endpoint routes: `/onvif/device_service`, `/onvif/media_service`, `/onvif/ptz_service`, `/onvif/imaging_service`.
- [X] T077 [P] Add tower-http TimeoutLayer middleware for request timeouts.
- [X] T078 [P] Add axum DefaultBodyLimit middleware for payload size limiting (configurable, default 1MB).
- [X] T079 [P] Implement graceful shutdown signal handler in server.rs.
- [X] T080 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/soap.rs` with `SoapEnvelope<T>` generic struct using serde.
- [X] T081 [P] Define `SoapBody<T>` struct in soap.rs with `#[serde(flatten)]` content field.
- [X] T082 [P] Define `SoapHeader` struct in soap.rs for WS-Security header parsing.
- [X] T083 [P] Implement `parse_soap_request()` function in soap.rs using quick-xml deserializer.
- [X] T084 [P] Implement `build_soap_response()` function in soap.rs using quick-xml serializer.
- [X] T085 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/dispatcher.rs` with `ServiceDispatcher` struct.
- [X] T086 [P] Define `ServiceHandler` trait in dispatcher.rs with `handle_operation()` method signature.
- [X] T087 [P] Implement SOAP action header parsing in dispatcher.rs to extract operation name.
- [X] T088 [P] Implement service routing logic in dispatcher.rs mapping actions to handlers.

### Error Handling

- [X] T089 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/error.rs` with `OnvifError` enum using thiserror.
- [X] T090 [P] Add `OnvifError::ActionNotSupported(String)` variant for EC-001.
- [X] T091 [P] Add `OnvifError::WellFormed(String)` variant for EC-002.
- [X] T092 [P] Add `OnvifError::InvalidArgVal { subcode: String, reason: String }` variant for EC-003, EC-006.
- [X] T093 [P] Add `OnvifError::HardwareFailure(String)` variant for EC-005.
- [X] T094 [P] Add `OnvifError::NotAuthorized` variant for EC-011.
- [X] T095 [P] Add `OnvifError::MaxUsers` variant for EC-013.
- [X] T096 [P] Add `OnvifError::ConfigurationConflict(String)` variant for EC-015.
- [X] T097 [P] Implement `OnvifError::to_soap_fault()` method generating SOAP fault XML with proper codes.
- [X] T098 [P] Implement `OnvifError::http_status()` method returning appropriate HTTP status codes.
- [X] T099 [P] Implement `soap_fault()` helper function generating full SOAP fault envelope XML.

### Input Validation

- [X] T100 Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/utils/mod.rs` with module declarations for validation, memory.
- [X] T101 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/utils/validation.rs` with `ValidationError` enum.
- [X] T102 [P] Add `ValidationError::InvalidHttpMethod` variant.
- [X] T103 [P] Add `ValidationError::InvalidContentType` variant.
- [X] T104 [P] Add `ValidationError::InvalidSoapEnvelope` variant.
- [X] T105 [P] Add `ValidationError::MissingSoapAction` variant.
- [X] T106 [P] Add `ValidationError::InvalidCharacters(String)` variant.
- [X] T107 [P] Add `ValidationError::InputTooLong(usize)` variant.
- [X] T108 [P] Create `InputValidator` struct in validation.rs with configurable `max_string_length`.
- [X] T109 [P] Implement `InputValidator::validate_http_request()` checking method is POST and content-type is text/xml or application/soap+xml.
- [X] T110 [P] Implement `InputValidator::validate_soap_envelope()` checking for Envelope and Body elements.
- [X] T111 [P] Implement `InputValidator::sanitize_string()` removing control characters and detecting dangerous patterns (<!ENTITY, javascript:).

**Checkpoint**: Platform abstraction, configuration, logging, and SOAP routing operational.

---

## Phase 3: Authentication & Security

**Purpose**: Implement authentication mechanisms and security hardening required before user story services.

### WS-Security UsernameToken

- [X] T112 Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/auth/mod.rs` with module declarations for ws_security, http_digest, credentials.
- [X] T113 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/auth/ws_security.rs` with `WsSecurityValidator` struct.
- [X] T114 [P] Implement `WsSecurityValidator::new()` with configurable `max_timestamp_age_seconds`.
- [X] T115 [P] Implement `WsSecurityValidator::validate_digest()` computing SHA1(Nonce + Created + Password) and comparing with timing-safe comparison.
- [X] T116 [P] Implement `WsSecurityValidator::validate_timestamp()` checking Created timestamp is within acceptable window.
- [X] T117 [P] Implement `WsSecurityValidator::parse_username_token()` extracting Username, Nonce, Created, PasswordDigest from SOAP header.
- [X] T118 [P] Define `AuthError` enum in ws_security.rs with variants: InvalidNonce, InvalidTimestamp, InvalidDigest, ExpiredTimestamp.

### HTTP Digest Authentication

- [X] T119 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/auth/http_digest.rs` with `HttpDigestAuth` struct.
- [X] T120 [P] Implement `HttpDigestAuth::new()` with configurable realm and nonce_validity_seconds.
- [X] T121 [P] Implement `HttpDigestAuth::generate_challenge()` creating WWW-Authenticate header with realm, nonce, qop, algorithm.
- [X] T122 [P] Implement `HttpDigestAuth::generate_nonce()` using cryptographically secure random bytes.
- [X] T123 [P] Implement `HttpDigestAuth::validate_response()` computing HA1=MD5(username:realm:password), HA2=MD5(method:uri), response=MD5(HA1:nonce:nc:cnonce:qop:HA2).
- [X] T124 [P] Implement `HttpDigestAuth::parse_authorization_header()` extracting all digest parameters from Authorization header.
- [X] T125 [P] Implement nonce storage and expiration checking in http_digest.rs.

### Authentication Configuration

- [X] T126 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/auth/credentials.rs` with `AuthConfig` struct.
- [X] T127 [P] Implement `AuthConfig::should_authenticate()` returning `self.enabled` for auth bypass support (FR-048a).
- [X] T128 [P] Implement `AuthConfig::from_config()` loading `[server] auth_enabled` from configuration.
- [X] T129 [P] Create authentication middleware function for axum extracting and validating credentials.
- [X] T130 [P] Implement credential validation against user storage in middleware.

### Rate Limiting

- [X] T131 Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/security/mod.rs` with module declarations for rate_limit, brute_force, xml_security, audit.
- [X] T132 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/security/rate_limit.rs` with `RateLimiter` struct using DashMap.
- [X] T133 [P] Define `RequestCount` struct in rate_limit.rs with count and window_start fields.
- [X] T134 [P] Implement `RateLimiter::new()` with configurable `max_requests_per_minute`.
- [X] T135 [P] Implement `RateLimiter::check_rate_limit()` incrementing count and checking against threshold.
- [X] T136 [P] Implement window reset logic when `window_start.elapsed() > window_duration`.
- [X] T137 [P] Create rate limiting middleware for axum returning HTTP 429 when limit exceeded.

### Brute Force Protection

- [X] T138 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/security/brute_force.rs` with `BruteForceProtection` struct.
- [X] T139 [P] Define `FailureRecord` struct with count, first_failure, blocked_until fields.
- [X] T140 [P] Implement `BruteForceProtection::new()` with configurable max_failures (default 5), failure_window (default 60s), block_duration (default 300s).
- [X] T141 [P] Implement `BruteForceProtection::record_failure()` incrementing failure count and setting blocked_until when threshold exceeded.
- [X] T142 [P] Implement `BruteForceProtection::is_blocked()` checking if current time is before blocked_until.
- [X] T143 [P] Implement `BruteForceProtection::clear_failures()` for successful authentication.
- [X] T143a [P] Add integration test verifying brute force threshold blocks IP after 5 failures in 60 seconds (EC-012).
- [X] T143b [P] Add integration test verifying blocked IP receives HTTP 429 response.
- [X] T143c [P] Add integration test verifying block expires after configured duration (default 300s).

### XML Security

- [X] T144 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/security/xml_security.rs` with `XmlSecurityValidator` struct.
- [X] T145 [P] Define `XmlSecurityError` enum with variants: PayloadTooLarge, XxeDetected, XmlBombDetected.
- [X] T146 [P] Implement `XmlSecurityValidator::new()` with configurable max_entity_expansions and max_payload_size (default 1MB).
- [X] T147 [P] Implement `XmlSecurityValidator::validate()` checking payload size.
- [X] T148 [P] Implement XXE detection in validate() checking for `<!ENTITY`, `SYSTEM`, `PUBLIC` patterns.
- [X] T149 [P] Implement XML bomb detection in validate() counting entity declarations.
- [X] T149a [P] Add integration test detecting XXE injection attempts with <!ENTITY patterns (EC-017).
- [X] T149b [P] Add integration test detecting XML bomb (billion laughs) attack patterns (EC-017).
- [X] T149c [P] Add integration test verifying HTTP 400 response and connection termination on attack detection (EC-017).

### Security Audit Logging

- [X] T150 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/security/audit.rs` with security event logging functions.
- [X] T151 [P] Implement `log_auth_failure()` with structured logging: client_ip, username, reason.
- [X] T152 [P] Implement `log_ip_blocked()` with structured logging: client_ip, duration_secs.
- [X] T153 [P] Implement `log_attack_detected()` with structured logging: client_ip, attack_type.
- [X] T154 [P] Implement `log_rate_limit_exceeded()` with structured logging: client_ip, request_count.

**Checkpoint**: Authentication and security hardening operational.

### HTTP/SOAP Input Hardening (NFR-006)

- [X] T549 [P] Add path traversal validation helper in `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/utils/validation.rs` that canonicalizes request paths, rejects `..` segments, and enforces confinement to a configured root; integrate this into HTTP/SOAP routing (fulfills NFR-006 path traversal).
- [X] T550 [P] Extend `InputValidator::sanitize_string()` in `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/utils/validation.rs` to detect and reject common XSS-style payload patterns in SOAP headers and bodies, with unit tests (fulfills NFR-006 XSS).
- [X] T551 [P] Add integration tests in `/home/kmk/anyka-dev/cross-compile/onvif-rust/tests/integration/security_inputs.rs` that exercise XSS and path traversal scenarios and assert correct HTTP error responses and security log entries (maps to NFR-006 and EC-017).

---

## Phase 4: User Management (User Story 5)

**Purpose**: Implement user account storage, password security, and ONVIF user management operations.

### User Storage

- [ ] T155 Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/users/mod.rs` with module declarations for storage, password.
- [ ] T156 [P] Define `UserLevel` enum in `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/users/storage.rs` with variants: Administrator, Operator, User.
- [ ] T157 [P] Define `UserAccount` struct in storage.rs with fields: username, password_hash, level.
- [ ] T158 [P] Define `UserError` enum in storage.rs with variants: MaxUsersReached, UserExists, UserNotFound, InvalidCredentials.
- [ ] T159 [P] Create `UserStorage` struct with `RwLock<HashMap<String, UserAccount>>`.
- [ ] T160 [P] Implement `UserStorage::new()` with `MAX_USERS` constant set to 8.
- [ ] T161 [P] Implement `UserStorage::get_user()` returning Option<UserAccount>.
- [ ] T162 [P] Implement `UserStorage::create_user()` checking max users and duplicate username.
- [ ] T163 [P] Implement `UserStorage::delete_user()` removing user by username.
- [ ] T164 [P] Implement `UserStorage::update_user()` modifying existing user's password_hash or level.
- [ ] T165 [P] Implement `UserStorage::list_users()` returning Vec<UserAccount>.

### Password Security

- [ ] T166 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/users/password.rs` with `PasswordManager` struct.
- [ ] T167 [P] Define `PasswordError` enum with variants: HashingFailed, VerificationFailed, InvalidHash.
- [ ] T168 [P] Implement `PasswordManager::hash_password()` using Argon2id with SaltString from OsRng.
- [ ] T169 [P] Implement `PasswordManager::verify_password()` using Argon2::verify_password() for timing-safe comparison.

### User Persistence

- [ ] T170 [P] Implement `UserStorage::load_from_toml()` reading `[[users]]` array from users.toml file.
- [ ] T171 [P] Implement `UserStorage::save_to_toml()` writing users to TOML file with atomic write pattern.
- [ ] T172 [P] Add default admin user creation if users.toml doesn't exist.

### Tests for User Story 5

- [ ] T173 [P] [US5] Add unit tests for UserStorage CRUD operations in storage.rs.
- [ ] T174 [P] [US5] Add unit tests for PasswordManager hash/verify in password.rs.
- [ ] T175 [P] [US5] Add unit tests for max users limit enforcement (8 users).
- [ ] T176 [P] [US5] Create integration tests for user management in `/home/kmk/anyka-dev/cross-compile/onvif-rust/tests/onvif/user_management.rs`.

### Implementation for User Story 5

**WSDL Source**: `/home/kmk/anyka-dev/cross-compile/onvif/wsdl/devicemgmt.wsdl` (User management operations are part of Device Service)

**Requirement**: User management types MUST match the WSDL definitions. Use generated types from `src/onvif/generated/device_types.rs`.

- [ ] T177 [P] [US5] Verify GetUsersRequest/Response matches `devicemgmt.wsdl` definition; re-export in `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/device/user_types.rs`.
- [ ] T178 [P] [US5] Verify CreateUsersRequest/Response matches `devicemgmt.wsdl` definition (elements: User array with Username, Password, UserLevel).
- [ ] T179 [P] [US5] Verify DeleteUsersRequest/Response matches `devicemgmt.wsdl` definition (elements: Username array).
- [ ] T180 [P] [US5] Verify SetUserRequest/Response matches `devicemgmt.wsdl` definition (elements: User array).
- [ ] T181 [US5] Implement `handle_get_users()` in device service returning list of usernames and levels.
- [ ] T182 [US5] Implement `handle_create_users()` in device service validating admin privileges and creating accounts.
- [ ] T183 [US5] Implement `handle_delete_users()` in device service removing specified users.
- [ ] T184 [US5] Implement `handle_set_user()` in device service updating password or level.
- [ ] T185 [US5] Add authorization check for admin-only operations returning ter:NotAuthorized fault.

**Checkpoint**: User management fully operational with secure password storage.

---

## Phase 5: WS-Discovery Protocol

**Purpose**: Implement WS-Discovery for ONVIF device discovery per FR-049 to FR-052.

### Tests for WS-Discovery

- [ ] T186 [P] Add unit tests for Hello message generation in ws_discovery.rs.
- [ ] T187 [P] Add unit tests for Bye message generation in ws_discovery.rs.
- [ ] T188 [P] Add unit tests for ProbeMatch response generation in ws_discovery.rs.
- [ ] T189 [P] Add unit tests for discovery mode filtering (NonDiscoverable silently ignores probes).

### Implementation for WS-Discovery

- [ ] T190 Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/discovery/mod.rs` with module declarations.
- [ ] T191 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/discovery/ws_discovery.rs` with `WsDiscovery` struct.
- [ ] T192 [P] Define constants: `WS_DISCOVERY_MULTICAST` (239.255.255.250), `WS_DISCOVERY_PORT` (3702).
- [ ] T193 [P] Implement `WsDiscovery::new()` creating UDP socket with multicast support using socket2.
- [ ] T194 [P] Implement socket binding and multicast group joining in WsDiscovery::new().
- [ ] T195 [P] Implement `WsDiscovery::build_hello_message()` generating WS-Discovery Hello SOAP envelope.
- [ ] T196 [P] Implement `WsDiscovery::build_bye_message()` generating WS-Discovery Bye SOAP envelope.
- [ ] T197 [P] Implement `WsDiscovery::build_probe_match()` generating ProbeMatch response with device UUID, scopes, XAddrs.
- [ ] T198 [P] Implement `WsDiscovery::is_probe_message()` checking for Probe action in incoming message.
- [ ] T199 [P] Implement `WsDiscovery::send_hello()` async method sending Hello on startup.
- [ ] T200 [P] Implement `WsDiscovery::send_bye()` async method sending Bye on shutdown.
- [ ] T201 [P] Implement `WsDiscovery::run()` async loop receiving Probe messages and sending ProbeMatch responses.
- [ ] T202 [P] Add discoverable flag to WsDiscovery with `set_discovery_mode()` method for NonDiscoverable support (EC-014).
- [ ] T203 [P] Implement scopes configuration from device configuration.

**Checkpoint**: WS-Discovery operational with Hello/Bye/Probe handling.

---

## Phase 6: User Story 1 - Device Service (Priority: P1)

**Purpose**: Deliver Device Service endpoints for device discovery and information retrieval.

**Independent Test**: POST SOAP requests for Device Service operations to `/onvif/device_service` and verify ONVIF-compliant XML responses.

**Dependencies**: T218 (DeviceService module creation) MUST complete before Phase 7 T284, Phase 8 T397, and Phase 9 T470 can start, as they follow the same module structure pattern.

### Tests for User Story 1

- [ ] T204 [P] [US1] Add unit test for GetDeviceInformation handler in `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/device/mod.rs`.
- [ ] T205 [P] [US1] Add unit test for GetCapabilities handler returning all service endpoints.
- [ ] T206 [P] [US1] Add unit test for GetServices handler with namespace URIs and XAddr URLs.
- [ ] T207 [P] [US1] Add unit test for GetSystemDateAndTime handler with UTC and local timezone.
- [ ] T208 [P] [US1] Add unit test for SystemReboot handler.
- [ ] T209 [P] [US1] Add unit test for GetHostname handler.
- [ ] T210 [P] [US1] Add unit test for SetHostname handler with validation.
- [ ] T211 [P] [US1] Add unit test for GetNetworkInterfaces handler.
- [ ] T212 [P] [US1] Add unit test for GetScopes handler.
- [ ] T213 [P] [US1] Add unit test for SetScopes handler.
- [ ] T214 [P] [US1] Add unit test for AddScopes handler.
- [ ] T215 [P] [US1] Add unit test for GetDiscoveryMode handler.
- [ ] T216 [P] [US1] Add unit test for SetDiscoveryMode handler.
- [ ] T217 [P] [US1] Create ONVIF Base Device integration tests in `/home/kmk/anyka-dev/cross-compile/onvif-rust/tests/onvif/device_service.rs`.

### Implementation for User Story 1 - Types

**WSDL Source**: `/home/kmk/anyka-dev/cross-compile/onvif/wsdl/devicemgmt.wsdl`

**Requirement**: All Device Service types MUST be generated from or validated against `devicemgmt.wsdl`. Use the generated types from `src/onvif/generated/device_types.rs` (created in T072d) and re-export or extend them in the device module.

- [ ] T218 [P] [US1] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/device/mod.rs` with DeviceService struct, re-exporting generated types from `super::generated::device_types`.
- [ ] T219 [P] [US1] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/device/types.rs` re-exporting and extending WSDL-generated types with any implementation-specific additions.
- [ ] T220 [P] [US1] Verify GetDeviceInformationRequest/Response matches `devicemgmt.wsdl` definition (elements: Manufacturer, Model, FirmwareVersion, SerialNumber, HardwareId).
- [ ] T221 [P] [US1] Verify GetCapabilitiesRequest/Response matches `devicemgmt.wsdl` definition (elements: Analytics, Device, Events, Imaging, Media, PTZ capabilities).
- [ ] T222 [P] [US1] Verify GetServicesRequest/Response matches `devicemgmt.wsdl` definition (elements: Service array with Namespace, XAddr, Version, Capabilities).
- [ ] T223 [P] [US1] Verify GetSystemDateAndTimeRequest/Response matches `devicemgmt.wsdl` definition (elements: DateTimeType, DaylightSavings, TimeZone, UTCDateTime, LocalDateTime).
- [ ] T224 [P] [US1] Verify SystemRebootRequest/Response matches `devicemgmt.wsdl` definition (elements: Message).
- [ ] T225 [P] [US1] Verify GetHostnameRequest/Response matches `devicemgmt.wsdl` definition (elements: FromDHCP, Name).
- [ ] T226 [P] [US1] Verify SetHostnameRequest/Response matches `devicemgmt.wsdl` definition (elements: Name).
- [ ] T227 [P] [US1] Verify GetNetworkInterfacesRequest/Response matches `devicemgmt.wsdl` definition (elements: NetworkInterface array per tt:NetworkInterface).
- [ ] T228 [P] [US1] Verify GetScopesRequest/Response matches `devicemgmt.wsdl` definition (elements: Scopes array per tt:Scope).
- [ ] T229 [P] [US1] Verify SetScopesRequest/Response matches `devicemgmt.wsdl` definition (elements: Scopes array).
- [ ] T230 [P] [US1] Verify AddScopesRequest/Response matches `devicemgmt.wsdl` definition (elements: ScopeItem).
- [ ] T231 [P] [US1] Verify GetDiscoveryModeRequest/Response matches `devicemgmt.wsdl` definition (elements: DiscoveryMode enum - Discoverable/NonDiscoverable).
- [ ] T232 [P] [US1] Verify SetDiscoveryModeRequest/Response matches `devicemgmt.wsdl` definition (elements: DiscoveryMode).
- [ ] T233 [P] [US1] Verify SetSystemDateAndTimeRequest/Response matches `devicemgmt.wsdl` definition.
- [ ] T234 [P] [US1] Verify GetDNSRequest/Response matches `devicemgmt.wsdl` definition (conditional).
- [ ] T235 [P] [US1] Verify SetDNSRequest/Response matches `devicemgmt.wsdl` definition (conditional).
- [ ] T236 [P] [US1] Verify GetNTPRequest/Response matches `devicemgmt.wsdl` definition (conditional).
- [ ] T237 [P] [US1] Verify SetNTPRequest/Response matches `devicemgmt.wsdl` definition (conditional).
- [ ] T238 [P] [US1] Verify SetNetworkInterfacesRequest/Response matches `devicemgmt.wsdl` definition.
- [ ] T239 [P] [US1] Verify SetSystemFactoryDefaultRequest/Response matches `devicemgmt.wsdl` definition (conditional).
- [ ] T240 [P] [US1] Verify GetServiceCapabilitiesRequest/Response matches `devicemgmt.wsdl` definition (elements: DeviceServiceCapabilities).

### Implementation for User Story 1 - Handlers

- [ ] T241 [US1] Implement `DeviceService::new()` constructor with ConfigRuntime and Platform references.
- [ ] T242 [US1] Implement `handle_get_device_information()` returning device info from platform and configuration.
- [ ] T243 [US1] Implement `handle_get_capabilities()` returning all service endpoints and feature flags.
- [ ] T244 [US1] Implement `handle_get_services()` returning namespace URIs and XAddr URLs for Device, Media, PTZ, Imaging.
- [ ] T245 [US1] Implement `handle_get_system_date_and_time()` returning UTC and local time from system clock.
- [ ] T246 [US1] Implement `handle_set_system_date_and_time()` updating system time.
- [ ] T247 [US1] Implement `handle_system_reboot()` initiating system reboot via platform.
- [ ] T248 [US1] Implement `handle_get_hostname()` returning current hostname from configuration.
- [ ] T249 [US1] Implement `handle_set_hostname()` updating hostname in configuration.
- [ ] T250 [US1] Implement `handle_get_network_interfaces()` returning network interface configurations.
- [ ] T251 [US1] Implement `handle_set_network_interfaces()` updating network configuration.
- [ ] T252 [US1] Implement `handle_get_scopes()` returning device scopes from configuration.
- [ ] T253 [US1] Implement `handle_set_scopes()` replacing device scopes in configuration.
- [ ] T254 [US1] Implement `handle_add_scopes()` appending scope to existing scopes.
- [ ] T255 [US1] Implement `handle_get_discovery_mode()` returning current discovery mode.
- [ ] T256 [US1] Implement `handle_set_discovery_mode()` updating discovery mode (Discoverable/NonDiscoverable).
- [ ] T257 [US1] Implement `handle_get_dns()` returning DNS configuration (conditional).
- [ ] T258 [US1] Implement `handle_set_dns()` updating DNS configuration (conditional).
- [ ] T259 [US1] Implement `handle_get_ntp()` returning NTP configuration (conditional).
- [ ] T260 [US1] Implement `handle_set_ntp()` updating NTP configuration (conditional).
- [ ] T261 [US1] Implement `handle_set_system_factory_default()` resetting to factory defaults (conditional).
- [ ] T262 [US1] Implement `handle_get_service_capabilities()` returning device service capabilities.

### Implementation for User Story 1 - Faults

- [ ] T263 [P] [US1] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/device/faults.rs` with device-specific fault mappings.
- [ ] T264 [US1] Implement fault for invalid hostname format.
- [ ] T265 [US1] Implement fault for unsupported network configuration.
- [ ] T266 [US1] Implement fault for invalid scope format.

### Implementation for User Story 1 - Registration

- [ ] T267 [US1] Register DeviceService routes in server.rs for `/onvif/device_service`.
- [ ] T268 [US1] Implement SOAPAction dispatch for all Device Service operations.

**Checkpoint**: Device discovery works end-to-end (MVP scope).

---

## Phase 7: User Story 2 - Media Service (Priority: P1)

**Purpose**: Provide Media Service operations for profile and stream configuration.

**Independent Test**: Invoke Media Service SOAP actions and verify profile tokens, stream URIs, and configurations.

### Tests for User Story 2

- [ ] T269 [P] [US2] Add unit test for GetProfiles handler.
- [ ] T270 [P] [US2] Add unit test for GetProfile handler with specific token.
- [ ] T271 [P] [US2] Add unit test for CreateProfile handler.
- [ ] T272 [P] [US2] Add unit test for DeleteProfile handler.
- [ ] T273 [P] [US2] Add unit test for GetStreamUri handler with RTSP URI composition.
- [ ] T274 [P] [US2] Add unit test for GetSnapshotUri handler.
- [ ] T275 [P] [US2] Add unit test for GetVideoSources handler.
- [ ] T276 [P] [US2] Add unit test for GetAudioSources handler.
- [ ] T277 [P] [US2] Add unit test for GetVideoSourceConfigurations handler.
- [ ] T278 [P] [US2] Add unit test for GetVideoEncoderConfigurations handler.
- [ ] T279 [P] [US2] Add unit test for GetVideoEncoderConfigurationOptions handler.
- [ ] T280 [P] [US2] Add unit test for GetAudioSourceConfigurations handler.
- [ ] T281 [P] [US2] Add unit test for GetAudioEncoderConfigurations handler.
- [ ] T282 [P] [US2] Add unit test for invalid profile token returning ter:NoProfile fault.
- [ ] T283 [P] [US2] Create Media Device integration tests in `/home/kmk/anyka-dev/cross-compile/onvif-rust/tests/onvif/media_service.rs`.

### Implementation for User Story 2 - Types

**WSDL Source**: `/home/kmk/anyka-dev/cross-compile/onvif/wsdl/media.wsdl`
**Schema Source**: `/home/kmk/anyka-dev/cross-compile/onvif/wsdl/onvif.xsd` (for tt:* types)

**Requirement**: All Media Service types MUST be generated from or validated against `media.wsdl` and `onvif.xsd`. Use the generated types from `src/onvif/generated/media_types.rs` (created in T072e) and re-export or extend them in the media module.

- [ ] T284 [P] [US2] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/media/mod.rs` with MediaService struct, re-exporting generated types from `super::generated::media_types`.
- [ ] T285 [P] [US2] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/media/types.rs` re-exporting and extending WSDL-generated types.
- [ ] T286 [P] [US2] Verify `MediaProfile` struct matches `media.wsdl` tt:Profile definition (elements: token, Name, VideoSourceConfiguration, VideoEncoderConfiguration, AudioSourceConfiguration, AudioEncoderConfiguration, PTZConfiguration).
- [ ] T287 [P] [US2] Verify `VideoSourceConfiguration` struct matches `onvif.xsd` tt:VideoSourceConfiguration (elements: token, SourceToken, Bounds).
- [ ] T288 [P] [US2] Verify `VideoEncoderConfiguration` struct matches `onvif.xsd` tt:VideoEncoderConfiguration (elements: token, Name, Encoding, Resolution, Quality, RateControl, H264, MPEG4).
- [ ] T289 [P] [US2] Verify `AudioSourceConfiguration` struct matches `onvif.xsd` tt:AudioSourceConfiguration (elements: token, SourceToken).
- [ ] T290 [P] [US2] Verify `AudioEncoderConfiguration` struct matches `onvif.xsd` tt:AudioEncoderConfiguration (elements: token, Name, Encoding, Bitrate, SampleRate).
- [ ] T291 [P] [US2] Verify `VideoSource` struct matches `onvif.xsd` tt:VideoSource (elements: token, Framerate, Resolution).
- [ ] T292 [P] [US2] Verify `AudioSource` struct matches `onvif.xsd` tt:AudioSource (elements: token, Channels).
- [ ] T293 [P] [US2] Verify GetProfilesRequest/Response matches `media.wsdl` definition.
- [ ] T294 [P] [US2] Verify GetProfileRequest/Response matches `media.wsdl` definition (elements: ProfileToken).
- [ ] T295 [P] [US2] Verify CreateProfileRequest/Response matches `media.wsdl` definition.
- [ ] T296 [P] [US2] Verify DeleteProfileRequest/Response matches `media.wsdl` definition.
- [ ] T297 [P] [US2] Verify GetStreamUriRequest/Response matches `media.wsdl` definition (elements: StreamSetup, ProfileToken, MediaUri).
- [ ] T298 [P] [US2] Verify GetSnapshotUriRequest/Response matches `media.wsdl` definition (elements: ProfileToken, MediaUri).
- [ ] T299 [P] [US2] Verify GetVideoSourcesRequest/Response matches `media.wsdl` definition.
- [ ] T300 [P] [US2] Verify GetAudioSourcesRequest/Response matches `media.wsdl` definition.
- [ ] T301 [P] [US2] Verify GetVideoSourceConfigurationsRequest/Response matches `media.wsdl` definition.
- [ ] T302 [P] [US2] Verify GetVideoSourceConfigurationRequest/Response matches `media.wsdl` definition.
- [ ] T303 [P] [US2] Verify SetVideoSourceConfigurationRequest/Response matches `media.wsdl` definition.
- [ ] T304 [P] [US2] Verify GetVideoEncoderConfigurationsRequest/Response matches `media.wsdl` definition.
- [ ] T305 [P] [US2] Verify GetVideoEncoderConfigurationRequest/Response matches `media.wsdl` definition.
- [ ] T306 [P] [US2] Verify SetVideoEncoderConfigurationRequest/Response matches `media.wsdl` definition.
- [ ] T307 [P] [US2] Define GetVideoEncoderConfigurationOptionsRequest/Response in types.rs.
- [ ] T308 [P] [US2] Define GetAudioSourceConfigurationsRequest/Response in types.rs.
- [ ] T309 [P] [US2] Define GetAudioSourceConfigurationRequest/Response in types.rs.
- [ ] T310 [P] [US2] Define SetAudioSourceConfigurationRequest/Response in types.rs.
- [ ] T307 [P] [US2] Verify GetVideoEncoderConfigurationOptionsRequest/Response matches `media.wsdl` definition.
- [ ] T308 [P] [US2] Verify GetAudioSourceConfigurationsRequest/Response matches `media.wsdl` definition.
- [ ] T309 [P] [US2] Verify GetAudioSourceConfigurationRequest/Response matches `media.wsdl` definition.
- [ ] T310 [P] [US2] Verify SetAudioSourceConfigurationRequest/Response matches `media.wsdl` definition.
- [ ] T311 [P] [US2] Verify GetAudioEncoderConfigurationsRequest/Response matches `media.wsdl` definition.
- [ ] T312 [P] [US2] Verify GetAudioEncoderConfigurationRequest/Response matches `media.wsdl` definition.
- [ ] T313 [P] [US2] Verify SetAudioEncoderConfigurationRequest/Response matches `media.wsdl` definition.
- [ ] T314 [P] [US2] Verify AddVideoSourceConfigurationRequest/Response matches `media.wsdl` definition.
- [ ] T315 [P] [US2] Verify RemoveVideoSourceConfigurationRequest/Response matches `media.wsdl` definition.
- [ ] T316 [P] [US2] Verify AddVideoEncoderConfigurationRequest/Response matches `media.wsdl` definition.
- [ ] T317 [P] [US2] Verify RemoveVideoEncoderConfigurationRequest/Response matches `media.wsdl` definition.
- [ ] T318 [P] [US2] Verify AddAudioSourceConfigurationRequest/Response matches `media.wsdl` definition.
- [ ] T319 [P] [US2] Verify RemoveAudioSourceConfigurationRequest/Response matches `media.wsdl` definition.
- [ ] T320 [P] [US2] Verify AddAudioEncoderConfigurationRequest/Response matches `media.wsdl` definition.
- [ ] T321 [P] [US2] Verify RemoveAudioEncoderConfigurationRequest/Response matches `media.wsdl` definition.
- [ ] T322 [P] [US2] Verify GetCompatibleVideoSourceConfigurationsRequest/Response matches `media.wsdl` definition.
- [ ] T323 [P] [US2] Verify GetCompatibleVideoEncoderConfigurationsRequest/Response matches `media.wsdl` definition.
- [ ] T324 [P] [US2] Verify GetCompatibleAudioSourceConfigurationsRequest/Response matches `media.wsdl` definition.
- [ ] T325 [P] [US2] Verify GetCompatibleAudioEncoderConfigurationsRequest/Response matches `media.wsdl` definition.
- [ ] T326 [P] [US2] Verify GetMetadataConfigurationsRequest/Response matches `media.wsdl` definition.
- [ ] T327 [P] [US2] Verify SetMetadataConfigurationRequest/Response matches `media.wsdl` definition.
- [ ] T328 [P] [US2] Verify StartMulticastStreamingRequest/Response matches `media.wsdl` definition.
- [ ] T329 [P] [US2] Verify StopMulticastStreamingRequest/Response matches `media.wsdl` definition.
- [ ] T330 [P] [US2] Verify GetServiceCapabilitiesRequest/Response matches `media.wsdl` definition (elements: trt:Capabilities).

### Implementation for User Story 2 - Profile Manager

- [ ] T331 [P] [US2] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/media/profile_manager.rs` with ProfileManager struct.
- [ ] T332 [US2] Implement `ProfileManager::new()` loading profiles from configuration.
- [ ] T333 [US2] Implement `ProfileManager::get_profiles()` returning all media profiles.
- [ ] T334 [US2] Implement `ProfileManager::get_profile()` returning profile by token.
- [ ] T335 [US2] Implement `ProfileManager::create_profile()` adding new profile with validation.
- [ ] T336 [US2] Implement `ProfileManager::delete_profile()` removing profile by token.
- [ ] T337 [US2] Implement profile token validation with ter:NoProfile fault.

### Implementation for User Story 2 - Handlers

- [ ] T338 [US2] Implement `MediaService::new()` constructor.
- [ ] T339 [US2] Implement `handle_get_profiles()` returning all configured profiles.
- [ ] T340 [US2] Implement `handle_get_profile()` returning specific profile by token.
- [ ] T341 [US2] Implement `handle_create_profile()` creating new profile.
- [ ] T342 [US2] Implement `handle_delete_profile()` deleting profile by token.
- [ ] T343 [US2] Implement `handle_get_stream_uri()` composing RTSP URI for profile.
- [ ] T344 [US2] Implement `handle_get_snapshot_uri()` returning HTTP URI for JPEG snapshot (FR-053).
- [ ] T345 [US2] Implement `handle_get_video_sources()` returning video sources from platform.
- [ ] T346 [US2] Implement `handle_get_audio_sources()` returning audio sources from platform.
- [ ] T347 [US2] Implement `handle_get_video_source_configurations()` returning video source configs.
- [ ] T348 [US2] Implement `handle_get_video_source_configuration()` returning specific config.
- [ ] T349 [US2] Implement `handle_set_video_source_configuration()` updating config.
- [ ] T350 [US2] Implement `handle_get_video_encoder_configurations()` returning encoder configs.
- [ ] T351 [US2] Implement `handle_get_video_encoder_configuration()` returning specific encoder config.
- [ ] T352 [US2] Implement `handle_set_video_encoder_configuration()` updating encoder config.
- [ ] T353 [US2] Implement `handle_get_video_encoder_configuration_options()` returning valid ranges.
- [ ] T354 [US2] Implement `handle_get_audio_source_configurations()` returning audio source configs.
- [ ] T355 [US2] Implement `handle_get_audio_source_configuration()` returning specific audio config.
- [ ] T356 [US2] Implement `handle_set_audio_source_configuration()` updating audio config.
- [ ] T357 [US2] Implement `handle_get_audio_encoder_configurations()` returning audio encoder configs.
- [ ] T358 [US2] Implement `handle_get_audio_encoder_configuration()` returning specific audio encoder.
- [ ] T359 [US2] Implement `handle_set_audio_encoder_configuration()` updating audio encoder config.
- [ ] T360 [US2] Implement `handle_add_video_source_configuration()` adding config to profile.
- [ ] T361 [US2] Implement `handle_remove_video_source_configuration()` removing config from profile.
- [ ] T362 [US2] Implement `handle_add_video_encoder_configuration()` adding encoder to profile.
- [ ] T363 [US2] Implement `handle_remove_video_encoder_configuration()` removing encoder from profile.
- [ ] T364 [US2] Implement `handle_add_audio_source_configuration()` adding audio config to profile.
- [ ] T365 [US2] Implement `handle_remove_audio_source_configuration()` removing audio config from profile.
- [ ] T366 [US2] Implement `handle_add_audio_encoder_configuration()` adding audio encoder to profile.
- [ ] T367 [US2] Implement `handle_remove_audio_encoder_configuration()` removing audio encoder from profile.
- [ ] T368 [US2] Implement `handle_get_compatible_video_source_configurations()`.
- [ ] T369 [US2] Implement `handle_get_compatible_video_encoder_configurations()`.
- [ ] T370 [US2] Implement `handle_get_compatible_audio_source_configurations()`.
- [ ] T371 [US2] Implement `handle_get_compatible_audio_encoder_configurations()`.
- [ ] T372 [US2] Implement `handle_get_metadata_configurations()`.
- [ ] T373 [US2] Implement `handle_set_metadata_configuration()`.
- [ ] T374 [US2] Implement `handle_start_multicast_streaming()`.
- [ ] T375 [US2] Implement `handle_stop_multicast_streaming()`.
- [ ] T376 [US2] Implement `handle_get_service_capabilities()` for media service.

### Implementation for User Story 2 - Registration

- [ ] T377 [US2] Register MediaService routes in server.rs for `/onvif/media_service`.
- [ ] T378 [US2] Implement SOAPAction dispatch for all Media Service operations.

**Checkpoint**: Media profiles discoverable and stream URIs retrievable.

---

## Phase 8: User Story 3 - PTZ Service (Priority: P2)

**Purpose**: Implement PTZ operations for camera movement and preset management.

**Independent Test**: Send PTZ SOAP requests with stubbed hardware responses to validate movement and preset flows.

### Tests for User Story 3

- [ ] T379 [P] [US3] Add unit test for GetNodes handler.
- [ ] T380 [P] [US3] Add unit test for GetNode handler.
- [ ] T381 [P] [US3] Add unit test for GetConfigurations handler.
- [ ] T382 [P] [US3] Add unit test for GetConfiguration handler.
- [ ] T383 [P] [US3] Add unit test for SetConfiguration handler.
- [ ] T384 [P] [US3] Add unit test for GetConfigurationOptions handler.
- [ ] T385 [P] [US3] Add unit test for AbsoluteMove handler with pan/tilt/zoom coordinates.
- [ ] T386 [P] [US3] Add unit test for RelativeMove handler with deltas.
- [ ] T387 [P] [US3] Add unit test for ContinuousMove handler with speed parameters.
- [ ] T388 [P] [US3] Add unit test for Stop handler.
- [ ] T389 [P] [US3] Add unit test for GetStatus handler returning position and movement status.
- [ ] T390 [P] [US3] Add unit test for GetPresets handler.
- [ ] T391 [P] [US3] Add unit test for SetPreset handler.
- [ ] T392 [P] [US3] Add unit test for GotoPreset handler.
- [ ] T393 [P] [US3] Add unit test for RemovePreset handler.
- [ ] T394 [P] [US3] Add unit test for GotoHomePosition handler.
- [ ] T395 [P] [US3] Add unit test for SetHomePosition handler.
- [ ] T396 [P] [US3] Create PTZ integration tests in `/home/kmk/anyka-dev/cross-compile/onvif-rust/tests/onvif/ptz_service.rs`.

### Implementation for User Story 3 - Types

**WSDL Source**: `/home/kmk/anyka-dev/cross-compile/onvif/wsdl/ptz.wsdl`
**Schema Source**: `/home/kmk/anyka-dev/cross-compile/onvif/wsdl/onvif.xsd` (for tt:* types)

**Requirement**: All PTZ Service types MUST be generated from or validated against `ptz.wsdl` and `onvif.xsd`. Use the generated types from `src/onvif/generated/ptz_types.rs` (created in T072f) and re-export or extend them in the ptz module.

- [ ] T397 [P] [US3] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/ptz/mod.rs` with PTZService struct, re-exporting generated types from `super::generated::ptz_types`.
- [ ] T398 [P] [US3] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/ptz/types.rs` re-exporting and extending WSDL-generated types.
- [ ] T399 [P] [US3] Verify `PTZNode` struct matches `onvif.xsd` tt:PTZNode definition (elements: token, Name, SupportedPTZSpaces, MaximumNumberOfPresets, HomeSupported, AuxiliaryCommands).
- [ ] T400 [P] [US3] Verify `PTZConfiguration` struct matches `onvif.xsd` tt:PTZConfiguration (elements: token, Name, NodeToken, DefaultPTZSpeed, DefaultPTZTimeout, PanTiltLimits, ZoomLimits).
- [ ] T401 [P] [US3] Verify `PTZPreset` struct matches `onvif.xsd` tt:PTZPreset (elements: token, Name, PTZPosition).
- [ ] T402 [P] [US3] Verify `PTZPosition` struct matches `onvif.xsd` tt:PTZVector (elements: PanTilt, Zoom).
- [ ] T403 [P] [US3] Verify `PTZSpeed` struct matches `onvif.xsd` tt:PTZSpeed (elements: PanTilt, Zoom).
- [ ] T404 [P] [US3] Verify `PTZStatus` struct matches `onvif.xsd` tt:PTZStatus (elements: Position, MoveStatus, Error, UtcTime).
- [ ] T405 [P] [US3] Verify GetNodesRequest/Response matches `ptz.wsdl` definition.
- [ ] T406 [P] [US3] Verify GetNodeRequest/Response matches `ptz.wsdl` definition (elements: NodeToken).
- [ ] T407 [P] [US3] Verify GetConfigurationsRequest/Response matches `ptz.wsdl` definition.
- [ ] T408 [P] [US3] Verify GetConfigurationRequest/Response matches `ptz.wsdl` definition (elements: PTZConfigurationToken).
- [ ] T409 [P] [US3] Verify SetConfigurationRequest/Response matches `ptz.wsdl` definition (elements: PTZConfiguration, ForcePersistence).
- [ ] T410 [P] [US3] Verify GetConfigurationOptionsRequest/Response matches `ptz.wsdl` definition (elements: ConfigurationToken).
- [ ] T411 [P] [US3] Verify AbsoluteMoveRequest/Response matches `ptz.wsdl` definition (elements: ProfileToken, Position, Speed).
- [ ] T412 [P] [US3] Verify RelativeMoveRequest/Response matches `ptz.wsdl` definition (elements: ProfileToken, Translation, Speed).
- [ ] T413 [P] [US3] Verify ContinuousMoveRequest/Response matches `ptz.wsdl` definition (elements: ProfileToken, Velocity, Timeout).
- [ ] T414 [P] [US3] Verify StopRequest/Response matches `ptz.wsdl` definition (elements: ProfileToken, PanTilt, Zoom).
- [ ] T415 [P] [US3] Verify GetStatusRequest/Response matches `ptz.wsdl` definition (elements: ProfileToken, PTZStatus).
- [ ] T416 [P] [US3] Verify GetPresetsRequest/Response matches `ptz.wsdl` definition (elements: ProfileToken, Preset array).
- [ ] T417 [P] [US3] Verify SetPresetRequest/Response matches `ptz.wsdl` definition (elements: ProfileToken, PresetName, PresetToken).
- [ ] T418 [P] [US3] Verify GotoPresetRequest/Response matches `ptz.wsdl` definition (elements: ProfileToken, PresetToken, Speed).
- [ ] T419 [P] [US3] Verify RemovePresetRequest/Response matches `ptz.wsdl` definition (elements: ProfileToken, PresetToken).
- [ ] T420 [P] [US3] Verify GotoHomePositionRequest/Response matches `ptz.wsdl` definition (elements: ProfileToken, Speed).
- [ ] T421 [P] [US3] Verify SetHomePositionRequest/Response matches `ptz.wsdl` definition (elements: ProfileToken).
- [ ] T422 [P] [US3] Verify GetCompatibleConfigurationsRequest/Response matches `ptz.wsdl` definition.
- [ ] T423 [P] [US3] Define SendAuxiliaryCommandRequest/Response in types.rs (conditional).
- [ ] T424 [P] [US3] Define GetServiceCapabilitiesRequest/Response in types.rs.

### Implementation for User Story 3 - State Manager

- [ ] T425 [P] [US3] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/ptz/state.rs` with PTZStateManager struct.
- [ ] T426 [US3] Implement `PTZStateManager::new()` with RwLock for concurrent access.
- [ ] T427 [US3] Implement `PTZStateManager::get_position()` returning current PTZ position.
- [ ] T428 [US3] Implement `PTZStateManager::set_position()` updating position atomically.
- [ ] T429 [US3] Implement `PTZStateManager::get_presets()` returning all saved presets.
- [ ] T430 [US3] Implement `PTZStateManager::set_preset()` saving current position as preset.
- [ ] T431 [US3] Implement `PTZStateManager::remove_preset()` deleting preset by token.
- [ ] T432 [US3] Implement `PTZStateManager::get_home_position()` returning home position.
- [ ] T433 [US3] Implement `PTZStateManager::set_home_position()` saving home position.
- [ ] T434 [US3] Implement preset persistence to configuration file.

### Implementation for User Story 3 - Handlers

- [ ] T435 [US3] Implement `PTZService::new()` constructor with platform PTZControl reference.
- [ ] T436 [US3] Implement `handle_get_nodes()` returning PTZ nodes from platform.
- [ ] T437 [US3] Implement `handle_get_node()` returning specific PTZ node.
- [ ] T438 [US3] Implement `handle_get_configurations()` returning PTZ configurations.
- [ ] T439 [US3] Implement `handle_get_configuration()` returning specific configuration.
- [ ] T440 [US3] Implement `handle_set_configuration()` updating PTZ configuration.
- [ ] T441 [US3] Implement `handle_get_configuration_options()` returning valid ranges.
- [ ] T442 [US3] Implement `handle_absolute_move()` calling platform PTZControl::move_to_position().
- [ ] T443 [US3] Implement `handle_relative_move()` calling platform with delta calculations.
- [ ] T444 [US3] Implement `handle_continuous_move()` calling platform PTZControl::continuous_move().
- [ ] T445 [US3] Implement `handle_stop()` calling platform PTZControl::stop().
- [ ] T446 [US3] Implement `handle_get_status()` returning current position and movement status.
- [ ] T447 [US3] Implement `handle_get_presets()` returning saved presets from state manager.
- [ ] T448 [US3] Implement `handle_set_preset()` saving current position as preset.
- [ ] T449 [US3] Implement `handle_goto_preset()` moving to saved preset position.
- [ ] T450 [US3] Implement `handle_remove_preset()` deleting preset.
- [ ] T451 [US3] Implement `handle_goto_home_position()` moving to home position.
- [ ] T452 [US3] Implement `handle_set_home_position()` saving current position as home.
- [ ] T453 [US3] Implement `handle_get_compatible_configurations()`.
- [ ] T454 [US3] Implement `handle_send_auxiliary_command()` (conditional).
- [ ] T455 [US3] Implement `handle_get_service_capabilities()` for PTZ service.

### Implementation for User Story 3 - Registration

- [ ] T456 [US3] Register PTZService routes in server.rs for `/onvif/ptz_service`.
- [ ] T457 [US3] Implement SOAPAction dispatch for all PTZ Service operations.

**Checkpoint**: PTZ controls operate independently against stub hardware.

---

## Phase 9: User Story 4 - Imaging Service (Priority: P2)

**Purpose**: Allow clients to view and adjust imaging parameters.

**Independent Test**: Execute Imaging Service SOAP actions and verify parameter retrieval and modification.

### Tests for User Story 4

- [ ] T458 [P] [US4] Add unit test for GetImagingSettings handler.
- [ ] T459 [P] [US4] Add unit test for SetImagingSettings handler with validation.
- [ ] T460 [P] [US4] Add unit test for GetOptions handler returning valid ranges.
- [ ] T461 [P] [US4] Add unit test for GetStatus handler.
- [ ] T462 [P] [US4] Add unit test for GetMoveOptions handler (conditional).
- [ ] T463 [P] [US4] Add unit test for Move handler (conditional).
- [ ] T464 [P] [US4] Add unit test for Stop handler (conditional).
- [ ] T465 [P] [US4] Add unit test for brightness parameter validation.
- [ ] T466 [P] [US4] Add unit test for contrast parameter validation.
- [ ] T467 [P] [US4] Add unit test for saturation parameter validation.
- [ ] T468 [P] [US4] Add unit test for day/night mode switching.
- [ ] T469 [P] [US4] Create Imaging integration tests in `/home/kmk/anyka-dev/cross-compile/onvif-rust/tests/onvif/imaging_service.rs`.

### Implementation for User Story 4 - Types

**WSDL Source**: `/home/kmk/anyka-dev/cross-compile/onvif/wsdl/imaging.wsdl`
**Schema Source**: `/home/kmk/anyka-dev/cross-compile/onvif/wsdl/onvif.xsd` (for tt:* types like ImagingSettings20)

**Requirement**: All Imaging Service types MUST be generated from or validated against `imaging.wsdl` and `onvif.xsd`. Use the generated types from `src/onvif/generated/imaging_types.rs` (created in T072g) and re-export or extend them in the imaging module.

- [ ] T470 [P] [US4] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/imaging/mod.rs` with ImagingService struct, re-exporting generated types from `super::generated::imaging_types`.
- [ ] T471 [P] [US4] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/imaging/types.rs` re-exporting and extending WSDL-generated types.
- [ ] T472 [P] [US4] Verify `ImagingSettings` struct matches `onvif.xsd` tt:ImagingSettings20 definition (elements: Brightness, Contrast, ColorSaturation, Sharpness, BacklightCompensation, Exposure, Focus, IrCutFilter, WhiteBalance, WideDynamicRange, Extension).
- [ ] T473 [P] [US4] Verify `ImagingOptions` struct matches `onvif.xsd` tt:ImagingOptions20 definition (elements: Brightness, Contrast, ColorSaturation, Sharpness ranges, etc.).
- [ ] T474 [P] [US4] Verify `ImagingStatus` struct matches `onvif.xsd` tt:ImagingStatus20 definition (elements: FocusStatus20, Extension).
- [ ] T475 [P] [US4] Verify `IrCutFilterMode` enum matches `onvif.xsd` tt:IrCutFilterMode (values: ON, OFF, AUTO).
- [ ] T476 [P] [US4] Verify `BacklightCompensation` struct matches `onvif.xsd` tt:BacklightCompensation20 (elements: Mode, Level).
- [ ] T477 [P] [US4] Verify `Exposure` struct matches `onvif.xsd` tt:Exposure20 (elements: Mode, Priority, Window, MinExposureTime, MaxExposureTime, MinGain, MaxGain, MinIris, MaxIris, ExposureTime, Gain, Iris).
- [ ] T478 [P] [US4] Verify `WhiteBalance` struct matches `onvif.xsd` tt:WhiteBalance20 (elements: Mode, CrGain, CbGain, Extension).
- [ ] T479 [P] [US4] Verify GetImagingSettingsRequest/Response matches `imaging.wsdl` definition (elements: VideoSourceToken, ImagingSettings).
- [ ] T480 [P] [US4] Verify SetImagingSettingsRequest/Response matches `imaging.wsdl` definition (elements: VideoSourceToken, ImagingSettings, ForcePersistence).
- [ ] T481 [P] [US4] Verify GetOptionsRequest/Response matches `imaging.wsdl` definition (elements: VideoSourceToken, ImagingOptions).
- [ ] T482 [P] [US4] Verify GetStatusRequest/Response matches `imaging.wsdl` definition (elements: VideoSourceToken, ImagingStatus).
- [ ] T483 [P] [US4] Verify GetMoveOptionsRequest/Response matches `imaging.wsdl` definition (conditional).
- [ ] T484 [P] [US4] Verify MoveRequest/Response matches `imaging.wsdl` definition (conditional).
- [ ] T485 [P] [US4] Verify StopRequest/Response matches `imaging.wsdl` definition (conditional).
- [ ] T486 [P] [US4] Verify GetServiceCapabilitiesRequest/Response matches `imaging.wsdl` definition (elements: timg:Capabilities).

### Implementation for User Story 4 - Settings Store

- [ ] T487 [P] [US4] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/onvif/imaging/settings_store.rs` with ImagingSettingsStore struct.
- [ ] T488 [US4] Implement `ImagingSettingsStore::new()` with RwLock for concurrent access.
- [ ] T489 [US4] Implement `ImagingSettingsStore::get_settings()` returning current imaging settings.
- [ ] T490 [US4] Implement `ImagingSettingsStore::set_settings()` with schema validation.
- [ ] T491 [US4] Implement `ImagingSettingsStore::validate_parameter()` checking min/max ranges.
- [ ] T492 [US4] Implement settings persistence to configuration file.

### Implementation for User Story 4 - Handlers

- [ ] T493 [US4] Implement `ImagingService::new()` constructor with platform ImagingControl reference.
- [ ] T494 [US4] Implement `handle_get_imaging_settings()` returning current settings from platform.
- [ ] T495 [US4] Implement `handle_set_imaging_settings()` validating and applying settings to platform.
- [ ] T496 [US4] Implement `handle_get_options()` returning valid parameter ranges from platform.
- [ ] T497 [US4] Implement `handle_get_status()` returning current imaging status.
- [ ] T498 [US4] Implement `handle_get_move_options()` returning focus move options (conditional).
- [ ] T499 [US4] Implement `handle_move()` executing focus move command (conditional).
- [ ] T500 [US4] Implement `handle_stop()` stopping focus movement (conditional).
- [ ] T501 [US4] Implement `handle_get_service_capabilities()` for imaging service.
- [ ] T502 [US4] Implement brightness setting via platform ImagingControl::set_brightness().
- [ ] T503 [US4] Implement contrast setting via platform ImagingControl::set_contrast().
- [ ] T504 [US4] Implement saturation setting via platform ImagingControl::set_saturation().
- [ ] T505 [US4] Implement sharpness setting via platform ImagingControl::set_sharpness().
- [ ] T506 [US4] Implement day/night mode setting via platform.

### Implementation for User Story 4 - Registration

- [ ] T507 [US4] Register ImagingService routes in server.rs for `/onvif/imaging_service`.
- [ ] T508 [US4] Implement SOAPAction dispatch for all Imaging Service operations.

**Checkpoint**: Imaging adjustments independently testable with stub hardware.

---

## Phase 10: Memory Management

**Purpose**: Implement memory monitoring and profiling for embedded target constraints.

### Memory Monitoring

- [ ] T509 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/src/utils/memory.rs` with memory constants.
- [ ] T510 [P] Define `MEMORY_SOFT_LIMIT` constant as 16MB (16 * 1024 * 1024).
- [ ] T511 [P] Define `MEMORY_HARD_LIMIT` constant as 24MB (24 * 1024 * 1024).
- [ ] T512 [P] Create `MemoryMonitor` struct with `AtomicUsize` for current_usage tracking.
- [ ] T513 [P] Define `MemoryError` enum with variants: HardLimitExceeded, WouldExceedLimit.
- [ ] T514 [P] Implement `MemoryMonitor::new()` constructor.
- [ ] T515 [P] Implement `MemoryMonitor::check_available()` validating request can be processed within limits.
- [ ] T516 [P] Implement `MemoryMonitor::record_allocation()` updating current usage.
- [ ] T517 [P] Implement `MemoryMonitor::record_deallocation()` decrementing usage.
- [ ] T518 [P] Add soft limit warning logging when usage exceeds 16MB.

### Memory Profiling (Feature-Gated)

- [ ] T519 [P] Create `AllocationTracker` struct with `#[cfg(feature = "memory-profiling")]` attribute.
- [ ] T520 [P] Define `AllocationInfo` struct with size, file, line fields.
- [ ] T521 [P] Implement `AllocationTracker::new()` with DashMap for concurrent tracking.
- [ ] T522 [P] Implement `AllocationTracker::track_allocation()` recording allocation with source location.
- [ ] T523 [P] Implement `AllocationTracker::track_deallocation()` removing tracking entry.
- [ ] T524 [P] Implement `AllocationTracker::log_stats()` outputting current and peak memory usage.
- [ ] T525 [P] Implement `AllocationTracker::get_peak_usage()` returning maximum recorded usage.

### Memory Integration

- [ ] T526 [P] Add memory check middleware to axum server returning HTTP 503 when hard limit exceeded (EC-016).
- [ ] T527 [P] Integrate memory monitoring with request handling.
- [ ] T527a [P] Add integration test simulating memory pressure approaching 24MB hard limit (EC-016).
- [ ] T527b [P] Add integration test verifying HTTP 503 Service Unavailable when memory limit would be exceeded (EC-016).
- [ ] T527c [P] Add integration test verifying existing connections maintained during memory pressure (EC-016).

**Checkpoint**: Memory monitoring operational with soft/hard limit enforcement.

---

## Phase 11: Polish & Validation

**Purpose**: Final verification, documentation, performance, and ONVIF compliance activities.

### Documentation

- [ ] T528 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/docs/architecture.md` documenting Rust implementation architecture.
- [ ] T529 [P] Document all module structures and dependencies in architecture.md.
- [ ] T530 [P] Document configuration format and schema in architecture.md.
- [ ] T531 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/docs/api.md` documenting ONVIF operations.
- [ ] T532 [P] Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/README.md` with build and usage instructions.

### Rust Quality Gates & Constitution Alignment

- [ ] T554 [P] Extend `/home/kmk/anyka-dev/cross-compile/onvif-rust/README.md` with a section documenting Rust quality gates (`cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --all-features`, `cargo doc --no-deps`, `cargo tarpaulin`) as the Rust-specific equivalents of `lint_code.sh`, `format_code.sh`, and `make test` defined in the main constitution and `.specify/memory/constitution-rust.md`.
- [ ] T555 [P] Add a validation checklist task to Phase 11 ensuring that the full Rust quality gate (fmt, clippy, test, doc, tarpaulin with ≥80% coverage) is executed and enforced in CI for `cross-compile/onvif-rust/` in alignment with `.specify/memory/constitution-rust.md`.

### Performance Optimization

- [ ] T533 [P] Profile hot paths in ONVIF service handlers.
- [ ] T534 [P] Optimize SOAP XML serialization/deserialization performance.
- [ ] T535 [P] Profile and optimize memory allocations in request handling.
- [ ] T536 [P] Add criterion benchmarks for critical paths in `/home/kmk/anyka-dev/cross-compile/onvif-rust/benches/`.

### Build & Deployment

- [X] T537 Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/scripts/build.sh` build script for cross-compilation.
- [X] T538 Create `/home/kmk/anyka-dev/cross-compile/onvif-rust/scripts/verify_binary.sh` for binary verification (architecture, size, dependencies).
- [ ] T539 [P] Verify binary runs on target ARMv5TE hardware.
- [ ] T540 [P] Verify binary size is comparable to C implementation.

### ONVIF Compliance Validation

- [ ] T541 [P] Run full unit test suite with `cargo test`.
- [ ] T542 [P] Run full integration test suite.
- [ ] T543 [P] Validate implementation against ONVIF Device Manager tool.
- [ ] T543a [P] Test ONVIF Device Manager compatibility with all implemented services (FR-020).
- [ ] T543b [P] Test VLC/ffplay compatibility with GetStreamUri responses (FR-020).
- [ ] T543c [P] Document any known compatibility issues with specific ONVIF clients (FR-020).
- [ ] T544 [P] Run ONVIF test tool validation for Base Device Test Specification.
- [ ] T545 [P] Run ONVIF test tool validation for Media Device Test Specification.
- [ ] T546 [P] Run ONVIF test tool validation for PTZ Device Test Specification.
- [ ] T547 [P] Run ONVIF test tool validation for Imaging Device Test Specification.
- [ ] T548 [P] Generate and review test coverage reports (target: 80% minimum).

**Checkpoint**: Full ONVIF compliance validated, documentation complete, release ready.

---

## Dependencies & Execution Order

```text
Phase 1 (Setup & CI/CD)
    ↓
Phase 2 (Foundational)
    ↓
Phase 3 (Authentication & Security)
    ↓
Phase 4 (User Management - US5)
    ↓
Phase 5 (WS-Discovery)
    ↓
┌───────────────────────────────────────┐
│ Phases 6-9 can run in parallel:       │
│  • Phase 6 (Device Service - US1)     │
│  • Phase 7 (Media Service - US2)      │
│  • Phase 8 (PTZ Service - US3)        │
│  • Phase 9 (Imaging Service - US4)    │
└───────────────────────────────────────┘
    ↓
Phase 10 (Memory Management)
    ↓
Phase 11 (Polish & Validation)
```

## User Story Priorities

- **US1 (P1)**: Device Discovery - MVP foundation
- **US2 (P1)**: Media Profile - Core streaming capability
- **US3 (P2)**: PTZ Control - Camera movement
- **US4 (P2)**: Imaging Settings - Image quality
- **US5 (P2)**: User Management - Security/access control

---

## Task Notation Legend

- `[P]` - Parallelizable with other `[P]` tasks in same phase
- `[US1]` through `[US5]` - User Story association
- All tasks include absolute file paths for LLM implementation clarity

---

## Validation

- All tasks follow `- [ ] T### [P?] [Story?] Description with absolute path` formatting.
- Each user story includes independent tests, implementation tasks, and checkpoints.
- Tasks align with plan.md architecture and spec.md functional requirements.
- Task granularity is atomic - one task per function/operation for LLM implementation.
- **WSDL Compliance**: All ONVIF types MUST be generated from or validated against WSDL files in `/home/kmk/anyka-dev/cross-compile/onvif/wsdl/`.
- Total tasks: 575

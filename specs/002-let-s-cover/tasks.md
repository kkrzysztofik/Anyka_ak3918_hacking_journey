# Tasks: ONVIF Service Gap Coverage

**Input**: Design artifacts from `/specs/002-let-s-cover/` (spec.md, plan.md, data-model.md, quickstart.md, contracts/*.md, research.md)  
**Prerequisites**: Complete Unified Configuration System foundation from Feature 001; branch `002-let-s-cover` checked out  
**Tests**: Unit and integration suites in `cross-compile/onvif/tests/` (CMocka) per [Testing Framework](../../../docs/agents/testing-framework.md)  
**Organization**: Tasks grouped by phase with user story mapping (US1 = Retrieve Accurate Service Data, US2 = Manage Configuration via ONVIF, US3 = Ensure Automated Regression Coverage)

## Format: `[ID] [P] [Story] Description`

- **[P]**: Task can run in parallel with others in the phase
- **[Story]**: US1, US2, or US3 as defined in spec.md §“User Scenarios & Testing”
- Descriptions reference exact file paths and relevant Functional Requirements (FR-###) from spec.md

## Path Conventions

- Source code: `cross-compile/onvif/src/**`
- Tests: `cross-compile/onvif/tests/src/**`
- Captured traffic: `cap/**`
- Documentation: `cross-compile/onvif/docs/**`

---

## Phase 0: Environment & Baseline Alignment (Blocking)

- [ ] T001 [US1] Checkout `002-let-s-cover`, sync submodules, and regenerate gSOAP artifacts via `make -C cross-compile/onvif generated` (quickstart.md §2.1).
- [ ] T002 [US1] Establish clean baseline by running `make -C cross-compile/onvif`, `make test`, and capturing `tests/OUT.log` for comparison (Testing Framework §Test Execution Commands).
- [ ] T003 [US1] Catalogue captured SOAP envelopes from `cap/` into a JSON index consumed by tests (`cross-compile/onvif/tests/src/data/onvif_capture_index.json`) covering GET/SET requests flagged in spec.md FR-001–FR-011.

---

## Phase 1: Shared Infrastructure Enhancements (Blocking for US1 & US2)

- [ ] T010 [P] [US1] Extend configuration read APIs in `cross-compile/onvif/src/core/config/config_runtime.c` to expose hostname, discovery mode, interfaces (with tokens), protocols, gateway, DNS, and NTP structures with default fallback (FR-001, FR-015, spec.md Edge Cases §legacy values).
- [ ] T011 [P] [US1] Add media, PTZ, and imaging profile accessors in `config_runtime.c` that deliver normalized tokens, encoder ranges, presets, and imaging bounds required by FR-006, FR-009, and FR-011 (data-model.md “Media Profile Definition”, “PTZ Preset Collection”, “Imaging Settings Bundle”).
- [ ] T012 [P] [US2] Implement configuration mutators in `config_runtime.c` and `config_runtime.h` for network, protocols, gateway, DNS, NTP, scopes, and device identity ensuring persistence queue flush within two seconds (FR-002, FR-005, FR-012).
- [ ] T013 [P] [US2] Integrate ONVIF user lifecycle helpers in `config_runtime.c` plus hashing via `utils/security/hash_utils.c` to satisfy FR-003 password policy and activation state requirements.
- [ ] T014 [US2] Add update-serialization guard in `config_runtime.c` (mutex or queue ordering) and log collisions to enforce deterministic final state under concurrent setters (spec.md Edge Cases §multiple administrators, FR-012).
- [ ] T015 [P] [US1] Refresh shared ONVIF token helpers in `cross-compile/onvif/src/services/common/onvif_types.c` and headers to align with contracts/*.md option tables, including relay channel count reporting (FR-004, media.md “Persistence & Validation”).
- [ ] T016 [P] [US1] Author unit tests in `cross-compile/onvif/tests/src/unit/core/config/test_config_runtime.c` validating new getters, default fallbacks, and token normalization (FR-001, FR-006, FR-009, FR-011).
- [ ] T017 [P] [US2] Add unit tests in `test_config_runtime.c` covering new setter helpers: persistence deadline, invalid parameter faults, DHCP-without-static rejection, and concurrent update ordering (FR-002, FR-003, FR-012, spec.md Edge Cases §DHCP disablement).

---

## Phase 2: User Story 1 – GET Operations (Priority P1)

### Tests First

- [ ] T020 [P] [US1] Expand `cross-compile/onvif/tests/src/unit/services/device/test_device_service.c` with `test_unit_device_get_*` cases covering ServiceCapabilities, DiscoveryMode, Hostname, Interfaces, Protocols, Gateway, DNS, NTP, Scopes, RelayOutputs, Users, and DeviceInformation success/fault paths (FR-001, FR-004, FR-015).
- [ ] T021 [P] [US1] Add media GET unit tests in `tests/src/unit/services/media/test_media_service.c` validating profile enumeration, encoder/source configurations, and stream URI construction (`test_unit_media_get_profiles_success`, etc.) using runtime data (FR-006, FR-015).
- [ ] T022 [P] [US1] Extend PTZ GET unit tests in `tests/src/unit/services/ptz/test_ptz_service.c` to cover Nodes, Node, Status retrieval, and preset listings with normalization checks (FR-009, FR-013).
- [ ] T023 [P] [US1] Update imaging GET unit tests in `tests/src/unit/services/imaging/test_imaging_service.c` for Settings, Options, and MoveOptions with invalid token coverage (FR-011, FR-013).
- [ ] T024 [US1] Create integration suite `cross-compile/onvif/tests/src/integration/media_device_get_requests.c` replaying representative GET envelopes from `cap/` and asserting schema-valid responses (spec.md User Story 1 Independent Test).

### Implementation

- [ ] T025 [P] [US1] Implement Device GET handlers in `cross-compile/onvif/src/services/device/onvif_device.c` using config runtime accessors, including `ONVIF_ERROR_NOT_SUPPORTED` responses for relay/certificate operations (FR-001, FR-004, FR-017).
- [ ] T026 [P] [US1] Update `onvif_media.c` to populate Profiles, Video/Audio sources, Encoder configs, and Stream/Snapshot URIs from runtime state with schema-compliant bounds (FR-006, FR-015, media.md tables).
- [ ] T027 [P] [US1] Enhance `cross-compile/onvif/src/services/ptz/onvif_ptz.c` to return normalized PTZ node/status data and preset collections, delegating to adapter getters (FR-009, FR-010, ptz.md Behavior Notes).
- [ ] T028 [P] [US1] Expand `cross-compile/onvif/src/services/imaging/onvif_imaging.c` GET flows to include options bundles and move capabilities aligned with hardware limits (FR-011, imaging.md Behavior Notes).
- [ ] T029 [US1] Wire GET operations through gSOAP layer (`cross-compile/onvif/src/protocol/gsoap/onvif_gsoap_device.c`, `*_media.c`, `*_ptz.c`, `*_imaging.c`) ensuring request parsing routes to updated handlers (FR-001, FR-006, FR-009, FR-011).
- [ ] T030 [US1] Update SOAP fault helpers in `cross-compile/onvif/src/protocol/gsoap/onvif_gsoap_response.c` to emit descriptive detail codes for unsupported parameters and missing tokens (FR-013, spec.md Edge Cases §optional fields).
- [ ] T031 [US1] Add device capability advertisement updates in `onvif_device.c` to declare zero relay channels and disabled certificate lifecycle (FR-004, FR-017).

---

## Phase 3: User Story 2 – SET Operations & Persistence (Priority P1)

### Tests First

- [ ] T040 [P] [US2] Extend `test_device_service.c` with setter cases (`test_unit_device_set_hostname_success`, `*_invalid`) covering validation, persistence, DHCP toggles, scope edits, and user CRUD (FR-002, FR-003, FR-012, FR-013).
- [ ] T041 [P] [US2] Add media setter unit tests in `tests/src/unit/services/media/test_media_service.c` validating configuration updates, token binding, and profile capacity enforcement (`CreateProfile`, `AddConfiguration`, etc.) (FR-007, FR-008, media.md Persistence & Validation).
- [ ] T042 [P] [US2] Expand PTZ setter unit tests in `tests/src/unit/services/ptz/test_ptz_service.c` covering ContinuousMove bounds, Stop flags, Preset lifecycle, Home position persistence, and error paths (FR-009, FR-010, ptz.md).
- [ ] T043 [P] [US2] Add imaging setter tests in `tests/src/unit/services/imaging/test_imaging_service.c` for `SetImagingSettings` and `Move`, including out-of-range rejection and persistence confirmation (FR-011, FR-012).
- [ ] T044 [US2] Introduce integration scenario `cross-compile/onvif/tests/src/integration/config_persistence_roundtrip.c` executing representative setter sequences then reloading runtime to assert durability and compliance with two-second SLA (spec.md User Story 2 Independent Test, FR-012, FR-016).

### Implementation

- [ ] T045 [P] [US2] Implement Device setters in `onvif_device.c` with validation utilities (`utils/validation/common_validation.h`), persistence via new config mutators, and descriptive faults for invalid IP/DNS/Scope inputs (FR-002, FR-005, FR-012, FR-013).
- [ ] T046 [P] [US2] Complete ONVIF user management operations (`CreateUsers`, `SetUser`, `DeleteUsers`) in `onvif_device.c`, using hashed passwords and enforcing limits from config schema (FR-003, FR-013).
- [ ] T047 [P] [US2] Update `onvif_media.c` setters (`SetVideoSourceConfiguration`, `SetVideoEncoderConfiguration`, `CreateProfile`, `DeleteProfile`, `AddConfiguration`, `RemoveConfiguration`) to validate cross-references, enforce ≤4 profiles, and persist via config runtime (FR-007, FR-008, FR-012).
- [ ] T048 [P] [US2] Implement PTZ movement and preset setters in `onvif_ptz.c`, ensuring adapter calls, bounds checking, and persistence of presets/home positions per profile (FR-009, FR-010, FR-012).
- [ ] T049 [P] [US2] Apply imaging setter logic in `onvif_imaging.c`, mapping incoming ranges to hardware limits, invoking platform adapters, and persisting updates (FR-011, FR-012).
- [ ] T050 [US2] Extend gSOAP request parsing (`onvif_gsoap_device.c`, `*_media.c`, `*_ptz.c`, `*_imaging.c`) for all setter operations to map SOAP inputs to internal structs with validation, including fault translations (FR-002–FR-011, FR-013).
- [ ] T051 [US2] Instrument setter logging in `cross-compile/onvif/src/utils/logging/onvif_logging.c` (or equivalent) to record actor, operation, parameters summary, and outcome per FR-014.
- [ ] T052 [US2] Ensure persistence queue flush scheduling meets two-second SLA by updating `config_runtime.c` timer/worker logic and adding metrics for missed deadlines (FR-012, FR-016).
- [ ] T053 [US2] Add safeguards in setters for unsupported options (IPv6-only, missing tokens, relay requests) returning `ONVIF_ERROR_NOT_SUPPORTED` without state change (FR-004, FR-013, spec.md Edge Cases).

---

## Phase 4: User Story 3 – Automated Regression Coverage (Priority P2)

- [ ] T060 [US3] Build composite integration suite `cross-compile/onvif/tests/src/integration/onvif_capture_replay_suite.c` to replay sequenced GET/SET envelopes from `cap/` verifying responses and persistence (spec.md User Story 3).
- [ ] T061 [US3] Update unified test runner registration (`tests/src/runner/test_runner.c`) to include new suites with `test_integration_*` naming per Testing Framework §Test Naming Convention.
- [ ] T062 [US3] Generate coverage targets by extending `Makefile` rules for `make test-coverage-html` to include new sources, confirming >90% coverage on added modules (Quality Gates §Success Criteria).
- [ ] T063 [US3] Automate OUT.log analysis script in `cross-compile/onvif/tests/scripts/verify_out_log.py` to assert absence of faults and capture regression artifacts (Testing Framework §OUT.log Output Mechanism).
- [ ] T064 [US3] Document regression workflow in `cross-compile/onvif/docs/testing/onvif_gap_coverage.md` linking captured scenarios, expected results, and replay commands (Development Standards §Documentation Standards).

---

## Phase 5: Quality, Documentation, and Release Checks (All Stories)

- [ ] T070 [P] [US1] Update Doxygen comments and module overviews in modified service, protocol, and config files (`@file`, `@brief`, etc.) per Development Standards §File Header Standards.
- [ ] T071 [P] [US2] Regenerate developer documentation via `make -C cross-compile/onvif docs` and update any ONVIF capability tables reflecting new behaviors (Development Standards §Documentation Standards).
- [ ] T072 [P] [US3] Run quality gates: `./cross-compile/onvif/scripts/lint_code.sh --check`, `./cross-compile/onvif/scripts/format_code.sh --check`, `make test`, `make test-unit`, `make test-integration`, `make test-coverage-html` ensuring zero failures (Quality Gates §Quality Assurance Checklist).
- [ ] T073 [US3] Perform self-review using `docs/agents/quality-gates.md` checklist, recording outcomes in `specs/002-let-s-cover/checklists/implementation.md`.
- [ ] T074 [US2] Package updated binaries into SD payload (`SD_card_contents/anyka_hack/`) and run device smoke test exercising representative GET/SET sequences, confirming audit logs and persistence (quickstart.md §6, spec.md Success Criteria SC-002/SC-004).

---

## Dependencies & Execution Order

- Phase 0 and Phase 1 tasks block all subsequent phases.
- Within each phase, tasks marked [P] can proceed in parallel once prerequisites are satisfied.
- Phase 2 GET implementations must complete before Phase 3 setters to ensure read-after-write validation (FR-016).
- Phase 3 persistence instrumentation (T052) must finish before Phase 4 replay tests.
- Quality tasks (Phase 5) execute after Phases 2–4 deliverables are merged.

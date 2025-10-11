# Tasks: Unified Configuration System for ONVIF Daemon

**Input**: Design documents from `/specs/001-unified-configuration-system/`
**Prerequisites**: plan.md (required), spec.md (required for user stories)

**Tests**: Tests are included as they are explicitly required by the project's development standards and testing framework.

**Organization**: Tasks are grouped by user story to enable independent implementation and testing of each story.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: Which user story this task belongs to (e.g., US1, US2, US3)
- Include exact file paths in descriptions

## Path Conventions

- **ONVIF Project**: `cross-compile/onvif/src/`, `cross-compile/onvif/tests/`
- All paths shown below use the ONVIF project structure

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Project initialization and basic structure

- [X] T001 Create unified configuration system branch and directory structure
- [X] T002 [P] Initialize new source files with proper headers and include guards
- [X] T003 [P] Configure build system integration for new modules

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core infrastructure that MUST be complete before ANY user story can be implemented

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [X] T004 [P] [US1] Create config_runtime.h with schema entry structures and core APIs
- [X] T005 [P] [US1] Create config_storage.h with INI storage interface definitions
- [X] T006 [P] [US1] Implement config_runtime.c with schema-driven runtime manager
- [X] T007 [P] [US1] Implement config_storage.c with atomic INI file operations
- [X] T008 [P] [US1] Create unit tests for config_runtime in tests/src/unit/core/config/test_config_runtime.c
- [X] T009 [P] [US1] Create unit tests for config_storage in tests/src/unit/core/config/test_config_storage.c
- [X] T010 [US1] Update build system to include new configuration modules
- [X] T011 [US1] Verify all new modules compile and pass linting

**Checkpoint**: Foundation ready - user story implementation can now begin in parallel

---

## Phase 3: User Story 1 - Single Source of Truth for Configuration (Priority: P1) 🎯 MVP

**Goal**: Establish unified configuration system where all subsystems read from single canonical source

**Independent Test**: Load configuration at daemon startup, query values from different subsystems (services, platform, networking), verify all receive identical values from unified manager

### Tests for User Story 1

**NOTE: Write these tests FIRST, ensure they FAIL before implementation**

- [X] T012 [P] [US1] Unit test for config_runtime_bootstrap with defaults in test_config_runtime.c
- [X] T013 [P] [US1] Unit test for config_runtime_get_int with validation in test_config_runtime.c
- [X] T014 [P] [US1] Unit test for config_storage_load with valid INI file in test_config_storage.c
- [X] T015 [P] [US1] Unit test for config_storage_load with missing file fallback in test_config_storage.c
- [X] T016 [P] [US1] Integration test for full configuration lifecycle in tests/src/integration/core/config/test_config_integration.c

### Implementation for User Story 1

- [X] T017 [P] [US1] Refactor config_runtime.c to implement getters/setters and apply defaults
- [ ] T018 [P] [US1] Refactor config.h to expose unified configuration interfaces
- [ ] T019 [US1] Update config_lifecycle.c to use config_runtime_bootstrap
- [ ] T020 [US1] Update platform_anyka.c to delegate to unified storage functions
- [ ] T021 [US1] Update platform.h to maintain API compatibility
- [ ] T022 [US1] Add configuration summary/introspection functionality
- [ ] T023 [US1] Implement fallback to validated defaults on load failure
- [ ] T024 [US1] Add structured error logging for configuration operations

**Checkpoint**: At this point, User Story 1 should be fully functional and testable independently

---

## Phase 4: User Story 2 - Schema-Driven Configuration Validation (Priority: P1)

**Goal**: Implement schema-driven validation that catches invalid values at load time

**Independent Test**: Provide various invalid configuration files (type mismatches, out-of-bounds values, missing required keys) and verify system rejects them with specific error messages

### Tests for User Story 2

- [ ] T025 [P] [US2] Unit test for schema validation with type mismatches in test_config_runtime.c
- [ ] T026 [P] [US2] Unit test for schema validation with out-of-bounds values in test_config_runtime.c
- [ ] T027 [P] [US2] Unit test for schema validation with missing required keys in test_config_runtime.c
- [ ] T028 [P] [US2] Unit test for config_runtime_set_value with validation in test_config_runtime.c
- [ ] T029 [P] [US2] Integration test for validation error handling in test_config_integration.c

### Implementation for User Story 2

- [ ] T030 [P] [US2] Extend config_runtime.c with comprehensive schema validation
- [ ] T031 [P] [US2] Implement typed getter/setter functions with validation
- [ ] T032 [US2] Add validation error reporting with structured logs
- [ ] T033 [US2] Integrate with utils/validation/common_validation.h
- [ ] T034 [US2] Implement config_runtime_apply_defaults functionality
- [ ] T035 [US2] Add runtime snapshot functionality with generation counter
- [ ] T036 [US2] Update config_storage.c to use schema validation during load

**Checkpoint**: At this point, User Stories 1 AND 2 should both work independently

---

## Phase 5: User Story 3 - Runtime Configuration Updates with Async Persistence (Priority: P2)

**Goal**: Enable runtime configuration updates with immediate in-memory changes and async persistence

**Independent Test**: Call configuration update API, verify immediate in-memory change, confirm operation returns quickly, then check persistence occurs within seconds and survives daemon restart

### Tests for User Story 3

- [ ] T037 [P] [US3] Unit test for config_runtime_set_value immediate update in test_config_runtime.c
- [ ] T038 [P] [US3] Unit test for async persistence queue operations in test_config_runtime.c
- [ ] T039 [P] [US3] Unit test for persistence queue coalescing in test_config_runtime.c
- [ ] T040 [P] [US3] Unit test for atomic file write operations in test_config_storage.c
- [ ] T041 [P] [US3] Unit test for persistence failure handling in test_config_storage.c
- [ ] T042 [P] [US3] Integration test for runtime updates with persistence in test_config_integration.c

### Implementation for User Story 3

- [ ] T043 [P] [US3] Implement async persistence queue in config_runtime.c
- [ ] T044 [P] [US3] Add thread-safe persistence queue management
- [ ] T045 [US3] Implement coalescing mechanism for rapid updates
- [ ] T046 [US3] Add atomic write operations in config_storage.c
- [ ] T047 [US3] Implement persistence failure handling and rollback
- [ ] T048 [US3] Add persistence status reporting functionality
- [ ] T049 [US3] Update config_runtime_set_value to queue persistence updates
- [ ] T050 [US3] Add background thread for processing persistence queue

**Checkpoint**: At this point, User Stories 1, 2, AND 3 should all work independently

---

## Phase 6: User Story 4 - Stream Profile Configuration Management (Priority: P2)

**Goal**: Support configuration of up to 4 video stream profiles with different resolutions, bitrates, and encoding settings

**Independent Test**: Define 4 stream profiles with varying settings, request streams via ONVIF GetProfiles and GetStreamUri, verify each profile delivers video with specified parameters

### Tests for User Story 4

- [ ] T051 [P] [US4] Unit test for stream profile schema validation in test_config_runtime.c
- [ ] T052 [P] [US4] Unit test for stream profile limit enforcement in test_config_runtime.c
- [ ] T053 [P] [US4] Unit test for stream profile parameter validation in test_config_runtime.c
- [ ] T054 [P] [US4] Integration test for media service profile integration in tests/src/integration/services/test_media_integration.c
- [ ] T055 [P] [US4] End-to-end test for ONVIF GetProfiles with configured profiles

### Implementation for User Story 4

- [ ] T056 [P] [US4] Extend config_runtime.h with stream profile schema sections
- [ ] T057 [P] [US4] Add stream profile parameter definitions to config_runtime.c
- [ ] T058 [US4] Implement config_runtime_get_stream_profile API
- [ ] T059 [US4] Implement config_runtime_set_stream_profile API
- [ ] T060 [US4] Implement config_runtime_validate_stream_profile API
- [ ] T061 [US4] Update onvif_media.c to use runtime stream profile configuration
- [ ] T062 [US4] Add profile limit enforcement (max 4 profiles)
- [ ] T063 [US4] Implement runtime profile updates via SetVideoEncoderConfiguration
- [ ] T064 [US4] Add seamless encoder updates without dropping connections

**Checkpoint**: At this point, User Stories 1, 2, 3, AND 4 should all work independently

---

## Phase 7: User Story 5 - User Credential Management (Priority: P2)

**Goal**: Support management of up to 8 user accounts with usernames and passwords for ONVIF authentication

**Independent Test**: Create multiple user accounts, authenticate ONVIF requests with different credentials, verify access is granted or denied appropriately

### Tests for User Story 5

- [ ] T065 [P] [US5] Unit test for user credential schema validation in test_config_runtime.c
- [ ] T066 [P] [US5] Unit test for user limit enforcement in test_config_runtime.c
- [ ] T067 [P] [US5] Unit test for password hashing with SHA256 in test_config_runtime.c
- [ ] T068 [P] [US5] Unit test for password verification in test_config_runtime.c
- [ ] T069 [P] [US5] Unit test for user management operations in test_config_runtime.c
- [ ] T070 [P] [US5] Integration test for authentication integration in tests/src/integration/networking/test_http_auth_integration.c
- [ ] T071 [P] [US5] End-to-end test for ONVIF authentication with managed users

### Implementation for User Story 5

- [ ] T072 [P] [US5] Extend config_runtime.h with user management schema sections
- [ ] T073 [P] [US5] Add user parameter definitions to config_runtime.c
- [ ] T074 [US5] Implement config_runtime_hash_password using utils/security/sha256.c
- [ ] T075 [US5] Implement config_runtime_verify_password API
- [ ] T076 [US5] Implement config_runtime_add_user API
- [ ] T077 [US5] Implement config_runtime_remove_user API
- [ ] T078 [US5] Implement config_runtime_update_user_password API
- [ ] T079 [US5] Add user limit enforcement (max 8 users)
- [ ] T080 [US5] Update http_auth.c to use runtime user management
- [ ] T081 [US5] Add user enumeration for management interfaces
- [ ] T082 [US5] Implement authentication attempt logging without credential exposure

**Checkpoint**: All user stories should now be independently functional

---

## Phase 8: Service Integration (Priority: P2)

**Purpose**: Update all ONVIF services to use unified configuration system

### Tests for Service Integration

- [ ] T083 [P] [INT] Integration test for device service configuration in tests/src/integration/services/test_device_integration.c
- [ ] T084 [P] [INT] Integration test for PTZ service configuration in tests/src/integration/services/test_ptz_integration.c
- [ ] T085 [P] [INT] Integration test for imaging service configuration in tests/src/integration/services/test_imaging_integration.c
- [ ] T086 [P] [INT] Integration test for snapshot service configuration in tests/src/integration/services/test_snapshot_integration.c
- [ ] T087 [P] [INT] Integration test for networking layer configuration in tests/src/integration/networking/test_network_integration.c

### Implementation for Service Integration

- [ ] T088 [P] [INT] Update onvif_service_handler.c to use runtime configuration
- [ ] T089 [P] [INT] Update onvif_device.c to use typed configuration getters
- [ ] T090 [P] [INT] Update onvif_ptz.c to use runtime configuration
- [ ] T091 [P] [INT] Update onvif_imaging.c to use runtime configuration
- [ ] T092 [P] [INT] Update onvif_snapshot.c to use runtime configuration
- [ ] T093 [INT] Update http_server.c to use runtime configuration for server settings
- [ ] T094 [INT] Update epoll_server.c to use runtime configuration
- [ ] T095 [INT] Update network_lifecycle.c to use runtime configuration
- [ ] T096 [INT] Implement runtime configuration updates via ONVIF operations
- [ ] T097 [INT] Add dynamic configuration updates without service restart

---

## Phase 9: Polish & Cross-Cutting Concerns

**Purpose**: Improvements that affect multiple user stories

- [ ] T098 [P] Update Doxygen documentation for all new APIs
- [ ] T099 [P] Update cross-compile/onvif/README.md with unified configuration flow
- [ ] T100 [P] Update docs/refactoring/06_code_quality.md with new patterns
- [ ] T101 [P] Remove deprecated platform*config*\* declarations from platform.h
- [ ] T102 [P] Update mock implementations in tests/src/mocks/
- [ ] T103 [P] Add comprehensive unit tests for edge cases
- [ ] T104 [P] Performance optimization and benchmarking
- [ ] T105 [P] Security hardening and vulnerability assessment
- [ ] T106 [P] Code cleanup and refactoring
- [ ] T107 [P] Update build system and Doxygen configuration
- [ ] T108 [P] Run comprehensive integration testing
- [ ] T109 [P] Validate ONVIF compliance with new configuration system

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies - can start immediately
- **Foundational (Phase 2)**: Depends on Setup completion - BLOCKS all user stories
- **User Stories (Phase 3+)**: All depend on Foundational phase completion
  - User stories can then proceed in parallel (if staffed)
  - Or sequentially in priority order (P1 → P2)
- **Service Integration (Phase 8)**: Depends on all user stories being complete
- **Polish (Phase 9)**: Depends on all implementation phases being complete

### User Story Dependencies

- **User Story 1 (P1)**: Can start after Foundational (Phase 2) - No dependencies on other stories
- **User Story 2 (P1)**: Can start after Foundational (Phase 2) - Builds on US1 foundation
- **User Story 3 (P2)**: Can start after Foundational (Phase 2) - Builds on US1/US2 foundation
- **User Story 4 (P2)**: Can start after Foundational (Phase 2) - Builds on US1/US2 foundation
- **User Story 5 (P2)**: Can start after Foundational (Phase 2) - Builds on US1/US2 foundation

### Within Each User Story

- Tests MUST be written and FAIL before implementation
- Core implementation before integration
- Story complete before moving to next priority

### Parallel Opportunities

- All Setup tasks marked [P] can run in parallel
- All Foundational tasks marked [P] can run in parallel (within Phase 2)
- Once Foundational phase completes, user stories can start in parallel (if team capacity allows)
- All tests for a user story marked [P] can run in parallel
- Different user stories can be worked on in parallel by different team members
- Service integration tasks marked [P] can run in parallel

---

## Parallel Example: User Story 1

```bash
# Launch all tests for User Story 1 together:
Task: "Unit test for config_runtime_bootstrap with defaults in test_config_runtime.c"
Task: "Unit test for config_runtime_get_int with validation in test_config_runtime.c"
Task: "Unit test for config_storage_load with valid INI file in test_config_storage.c"
Task: "Unit test for config_storage_load with missing file fallback in test_config_storage.c"
Task: "Integration test for full configuration lifecycle in test_config_integration.c"

# Launch all implementation tasks for User Story 1 together:
Task: "Refactor config.c to delegate to config_runtime APIs"
Task: "Refactor config.h to expose unified configuration interfaces"
Task: "Update config_lifecycle.c to use config_runtime_bootstrap"
Task: "Update platform_anyka.c to delegate to unified storage functions"
Task: "Update platform.h to maintain API compatibility"
```

---

## Implementation Strategy

### MVP First (User Stories 1 & 2 Only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational (CRITICAL - blocks all stories)
3. Complete Phase 3: User Story 1 (Single Source of Truth)
4. Complete Phase 4: User Story 2 (Schema-Driven Validation)
5. **STOP and VALIDATE**: Test both stories independently
6. Deploy/demo if ready

### Incremental Delivery

1. Complete Setup + Foundational → Foundation ready
2. Add User Story 1 → Test independently → Deploy/Demo (Core MVP!)
3. Add User Story 2 → Test independently → Deploy/Demo (Validation MVP!)
4. Add User Story 3 → Test independently → Deploy/Demo (Runtime Updates!)
5. Add User Story 4 → Test independently → Deploy/Demo (Stream Profiles!)
6. Add User Story 5 → Test independently → Deploy/Demo (User Management!)
7. Add Service Integration → Test independently → Deploy/Demo (Full System!)
8. Add Polish → Final validation → Production Ready!

### Parallel Team Strategy

With multiple developers:

1. Team completes Setup + Foundational together
2. Once Foundational is done:
   - Developer A: User Story 1 (Single Source of Truth)
   - Developer B: User Story 2 (Schema Validation)
3. Once P1 stories are done:
   - Developer A: User Story 3 (Runtime Updates)
   - Developer B: User Story 4 (Stream Profiles)
   - Developer C: User Story 5 (User Management)
4. All developers: Service Integration
5. All developers: Polish and final validation

---

## Notes

- [P] tasks = different files, no dependencies
- [Story] label maps task to specific user story for traceability
- Each user story should be independently completable and testable
- Verify tests fail before implementing
- Commit after each task or logical group
- Stop at any checkpoint to validate story independently
- Follow project coding standards: include ordering, global variable naming, return codes
- Use CMocka testing framework with \__wrap_ pattern for mocks
- Achieve >90% code coverage for all new modules
- All code must pass linting and formatting validation

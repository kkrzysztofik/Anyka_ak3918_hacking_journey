# Tasks Document

- [ ] 1. Introduce schema-driven runtime manager
  - Files: cross-compile/onvif/src/core/config/config_runtime.c, cross-compile/onvif/src/core/config/config_runtime.h
  - Create the schema-backed runtime manager that binds configuration entries to `struct application_config`, provides bootstrap/default helpers, and exposes typed getters/setters plus snapshot access.
  - Ensure Doxygen headers/documentation exist and reuse existing validation utilities for range/length checks.
  - _Leverage: cross-compile/onvif/src/core/config/config.c, cross-compile/onvif/src/utils/validation/common_validation.h_
  - _Requirements: Requirement 1, Requirement 2_
  - _Prompt: Implement the task for spec config-system-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Embedded C systems engineer focused on configuration infrastructure | Task: Build config_runtime.c/h with schema metadata, bootstrap, typed getters/setters, and runtime snapshot support while preserving INI-based defaults | Restrictions: Follow include-order rules, avoid magic return codes, document all public APIs with Doxygen, reuse existing validation helpers instead of duplicating logic | _Leverage: cross-compile/onvif/src/core/config/config.c, cross-compile/onvif/src/utils/validation/common_validation.h | _Requirements: Requirement 1, Requirement 2 | Success: New module compiles, passes lint/format checks, and config_runtime_bootstrap initializes application_config with defaults validated by unit tests | Instructions: Before you start, edit tasks.md to set this item to [-]; when you finish, change it to [x]._

- [ ] 2. Implement safe INI storage helpers
  - Files: cross-compile/onvif/src/core/config/config_storage.c, cross-compile/onvif/src/core/config/config_storage.h
  - Move/refine INI load/save logic into dedicated helpers that perform validation, checksum handling, and atomic temp-file writes before delegating to the runtime manager.
  - Emit structured logging on errors and ensure graceful fallback to defaults when loads fail.
  - _Leverage: cross-compile/onvif/src/core/config/config.c, cross-compile/onvif/src/platform/platform_anyka.c_
  - _Requirements: Requirement 1, Requirement 3_
  - _Prompt: Implement the task for spec config-system-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Firmware developer specializing in embedded persistence | Task: Create config_storage.c/h providing load/save/reload routines that reuse existing parsing logic, add checksum and temp-file handling, and integrate with config_runtime | Restrictions: Keep heap usage minimal, return ONVIF error constants only, ensure logging uses platform_log_* APIs, avoid duplicating schema definitions | _Leverage: cross-compile/onvif/src/core/config/config.c, cross-compile/onvif/src/platform/platform_anyka.c | _Requirements: Requirement 1, Requirement 3 | Success: Storage helpers compile, handle missing/corrupt files via defaults, and unit tests cover success/failure paths | Instructions: Before you start, edit tasks.md to set this item to [-]; when you finish, change it to [x]._

- [ ] 3. Refactor core config integration
  - Files: cross-compile/onvif/src/core/config/config.c, cross-compile/onvif/src/core/config/config.h
  - Update existing config APIs to delegate to config_runtime/config_storage, remove redundant global buffers, and expose any new helper prototypes needed by consumers.
  - Verify public headers reflect the new flow without breaking existing callers.
  - _Leverage: cross-compile/onvif/src/core/config/config_runtime.h, cross-compile/onvif/src/core/config/config_storage.h_
  - _Requirements: Requirement 1, Requirement 2_
  - _Prompt: Implement the task for spec config-system-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Systems refactoring specialist for embedded C codebases | Task: Rework config.c/h so config_init/load/validate/set/get use the runtime/storage modules, clean up legacy globals, and maintain ABI compatibility for services | Restrictions: Preserve existing function signatures unless design mandates changes, maintain documentation headers, ensure global variables stay at top with proper prefixes | _Leverage: cross-compile/onvif/src/core/config/config_runtime.h, cross-compile/onvif/src/core/config/config_storage.h | _Requirements: Requirement 1, Requirement 2 | Success: config.c compiles without duplicate logic, unit tests pass, and linting validates ordering/formatting | Instructions: Before you start, edit tasks.md to set this item to [-]; when you finish, change it to [x]._

- [ ] 4. Update lifecycle bootstrap to use runtime manager
  - Files: cross-compile/onvif/src/core/lifecycle/config_lifecycle.c
  - Replace manual initialization logic with calls to config_runtime_bootstrap and config_storage_load, ensuring generation counters and summaries remain accurate.
  - Handle fallback-to-default and error reporting according to new APIs.
  - _Leverage: cross-compile/onvif/src/core/config/config_runtime.h, cross-compile/onvif/src/core/config/config_storage.h_
  - _Requirements: Requirement 1_
  - _Prompt: Implement the task for spec config-system-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Embedded lifecycle engineer | Task: Wire config_lifecycle.c to bootstrap via config_runtime, invoke storage load/save helpers, and update logging/summary output accordingly | Restrictions: Keep allocation pattern unchanged, update only lifecycle logic needed for new APIs, ensure error paths free resources properly | _Leverage: cross-compile/onvif/src/core/config/config_runtime.h, cross-compile/onvif/src/core/config/config_storage.h | _Requirements: Requirement 1 | Success: Lifecycle build succeeds, configuration still loads defaults when INI missing, and integration tests compile | Instructions: Before you start, edit tasks.md to set this item to [-]; when you finish, change it to [x]._

- [ ] 5. Replace platform INI parsing with unified helpers
  - Files: cross-compile/onvif/src/platform/platform_anyka.c, cross-compile/onvif/src/platform/platform.h
  - Remove local buffer-based parser and call the new storage/runtime functions for load/save/get operations, keeping the platform API stable for callers.
  - Ensure logging and error codes comply with platform conventions.
  - _Leverage: cross-compile/onvif/src/core/config/config_runtime.h, cross-compile/onvif/src/core/config/config_storage.h_
  - _Requirements: Requirement 1, Requirement 3_
  - _Prompt: Implement the task for spec config-system-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Embedded platform engineer for Anyka systems | Task: Update platform_anyka.c/h to delegate configuration load/save/get to the unified manager, removing duplicate parsing and preserving API semantics | Restrictions: Maintain thread safety expectations, keep global state declarations at file top, reuse existing logging macros, do not alter unrelated platform subsystems | _Leverage: cross-compile/onvif/src/core/config/config_runtime.h, cross-compile/onvif/src/core/config/config_storage.h | _Requirements: Requirement 1, Requirement 3 | Success: Platform build passes, unit tests using mocks succeed, and no direct INI parsing remains in platform_anyka.c | Instructions: Before you start, edit tasks.md to set this item to [-]; when you finish, change it to [x]._

- [ ] 6. Propagate runtime config to services and networking
  - Files: cross-compile/onvif/src/protocol/response/onvif_service_handler.c, cross-compile/onvif/src/protocol/response/onvif_service_handler.h, cross-compile/onvif/src/networking/http/http_server.c, cross-compile/onvif/src/networking/common/epoll_server.c, cross-compile/onvif/src/core/lifecycle/network_lifecycle.c, cross-compile/onvif/src/core/lifecycle/service_manager.c, cross-compile/onvif/src/core/lifecycle/video_lifecycle.c, cross-compile/onvif/src/services/device/onvif_device.c, cross-compile/onvif/src/services/media/onvif_media.c, cross-compile/onvif/src/services/ptz/onvif_ptz.c, cross-compile/onvif/src/services/imaging/onvif_imaging.c, cross-compile/onvif/src/services/snapshot/onvif_snapshot.c
  - Update all service and networking layers to obtain configuration data through `config_runtime` snapshots or typed getters, removing direct dependencies on legacy manager globals and ensuring consistent refresh paths when configuration changes.
  - Verify dispatcher wiring continues to pass the correct manager pointer and that logging continues to reflect current configuration values.
  - _Leverage: cross-compile/onvif/src/protocol/response/onvif_service_handler.c, cross-compile/onvif/src/core/config/config_runtime.h_
  - _Requirements: Requirement 1, Requirement 2, Requirement 3_
  - _Prompt: Implement the task for spec config-system-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF services specialist | Task: Refactor service handler, lifecycle, and networking modules to pull configuration via the new runtime manager snapshot/getter APIs, ensuring no stale copies or legacy access paths remain | Restrictions: Preserve existing service handler contracts, keep function ordering guidelines, update Doxygen comments where signatures change, avoid duplicating config structs | _Leverage: cross-compile/onvif/src/protocol/response/onvif_service_handler.c, cross-compile/onvif/src/core/config/config_runtime.h | _Requirements: Requirement 1, Requirement 2, Requirement 3 | Success: Services and lifecycle modules compile against the new runtime API, integration builds succeed, and manual inspection confirms no direct references to legacy globals or platform_config helpers | Instructions: Before you start, edit tasks.md to set this item to [-]; when you finish, change it to [x]._

- [ ] 7. Remove legacy platform config API from headers and mocks
  - Files: cross-compile/onvif/src/platform/platform.h, cross-compile/onvif/tests/src/mocks/platform_mock.c, cross-compile/onvif/tests/src/mocks/platform_mock.h, cross-compile/onvif/README.md, README.md
  - Drop deprecated `platform_config_*` declarations, replace mock implementations with runtime-aware stubs, and update developer documentation/examples to reflect the unified configuration APIs.
  - Ensure tests compile with the new interfaces and that mock helpers expose any new persistence hooks required by the runtime manager.
  - _Leverage: cross-compile/onvif/src/core/config/config_runtime.h, cross-compile/onvif/src/core/config/config_storage.h_
  - _Requirements: Requirement 1, Requirement 3_
  - _Prompt: Implement the task for spec config-system-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Embedded platform/test infrastructure engineer | Task: Remove the legacy platform_config API from public headers and mocks, replace with runtime/persistence shims, and refresh documentation snippets accordingly | Restrictions: Maintain backward-compatible include guards, keep mock behaviour thread-safe, update unit tests that relied on old helpers | _Leverage: cross-compile/onvif/src/core/config/config_runtime.h, cross-compile/onvif/src/core/config/config_storage.h | _Requirements: Requirement 1, Requirement 3 | Success: platform.h no longer exposes platform_config_* symbols, mocks/tests build and pass, docs accurately describe the new configuration entry points | Instructions: Before you start, edit tasks.md to set this item to [-]; when you finish, change it to [x]._

- [ ] 8. Add configuration unit tests
  - Files: cross-compile/onvif/tests/src/unit/core/config/test_config_runtime.c, cross-compile/onvif/tests/src/unit/core/config/test_config_storage.c
  - Write CMocka tests covering defaults/bootstrap, getter/setter validation, load/save success, corrupt file fallback, and atomic write failure handling using mocks/stubs.
  - Update test CMake/Makefile entries if needed for new sources.
  - _Leverage: cross-compile/onvif/tests/src/mocks/platform_mock.c, cross-compile/onvif/tests/src/unit/core/config/test_config.c_
  - _Requirements: Requirement 2, Requirement 3_
  - _Prompt: Implement the task for spec config-system-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Embedded QA engineer using CMocka | Task: Create unit tests for config_runtime and config_storage covering success/error paths, mocking file I/O and validation edge cases | Restrictions: Follow test directory structure, ensure tests free allocated memory, keep assertions aligned with ONVIF error codes | _Leverage: cross-compile/onvif/tests/src/mocks/platform_mock.c, cross-compile/onvif/tests/src/unit/core/config/test_config.c | _Requirements: Requirement 2, Requirement 3 | Success: Tests compile and pass locally via `make test`, coverage shows new functions exercised | Instructions: Before you start, edit tasks.md to set this item to [-]; when you finish, change it to [x]._

- [ ] 9. Wire new modules into build and documentation
  - Files: cross-compile/onvif/Makefile, cross-compile/onvif/Doxyfile
  - Register new source files with the build system and ensure Doxygen captures the added headers/modules, updating any module lists or grouping tags.
  - Confirm formatting/lint scripts recognize the new paths.
  - _Leverage: cross-compile/onvif/Makefile, cross-compile/onvif/Doxyfile_
  - _Requirements: Requirement 1, Requirement 2, Requirement 3_
  - _Prompt: Implement the task for spec config-system-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Build and documentation engineer | Task: Update build scripts and Doxygen configuration to include config_runtime/storage sources and ensure tooling recognizes them | Restrictions: Do not regress other targets, keep Makefile ordering consistent, ensure Doxygen warnings are resolved | _Leverage: cross-compile/onvif/Makefile, cross-compile/onvif/Doxyfile | _Requirements: Requirement 1, Requirement 2, Requirement 3 | Success: `make -C cross-compile/onvif` builds with new modules, `doxygen Doxyfile` documents them without warnings | Instructions: Before you start, edit tasks.md to set this item to [-]; when you finish, change it to [x]._

- [ ] 10. Refresh configuration documentation and release notes
  - Files: docs/refactoring/06_code_quality.md, cross-compile/onvif/README.md
  - Document the unified configuration flow, new APIs, and persistence behavior for developers and testers.
  - Include guidance for SD-card tooling about atomic write expectations and fallback behavior.
  - _Leverage: .spec-workflow/specs/config-system-unification/design.md, docs/refactoring/02_architecture_analysis.md_
  - _Requirements: Requirement 3_
  - _Prompt: Implement the task for spec config-system-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Technical writer for embedded firmware | Task: Update developer documentation to explain the unified configuration manager, runtime access patterns, and persistence guarantees | Restrictions: Maintain existing doc tone, ensure code snippets stay in sync with headers, avoid duplicating information across docs | _Leverage: .spec-workflow/specs/config-system-unification/design.md, docs/refactoring/02_architecture_analysis.md | _Requirements: Requirement 3 | Success: Documentation changes reviewed with lint, readers understand new config flow, and release notes highlight migration steps | Instructions: Before you start, edit tasks.md to set this item to [-]; when you finish, change it to [x]._

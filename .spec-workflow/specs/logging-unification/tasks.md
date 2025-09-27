# Tasks Document

- [ ] 1. Create logging API header
  - File: src/utils/logging/logging.h
  - Define function-based logging API with error/warning/info/debug levels
  - Create log_context_t structure for service and operation tracking
  - Purpose: Establish unified logging interface for the ONVIF project
  - _Leverage: src/core/config/config.h, src/utils/string/string_shims.h_
  - _Requirements: 1, 3
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Systems Programming Developer specializing in C embedded systems | Task: Design the unified logging header exposing function-based APIs and context helpers mapped to requirements 1 and 3 | Restrictions: Use C99, avoid macro-heavy patterns, keep thread safety considerations, do not mutate existing config structures | Success: Header compiles cleanly, exports required prototypes, includes Doxygen docs, passes clang-tidy_

- [ ] 2. Implement logging core
  - File: src/utils/logging/logging.c
  - Implement log level functions with standardized format output
  - Generate timestamps with millisecond precision
  - Build hostname and component paths for formatted messages
  - Purpose: Provide logging implementation that meets performance and format targets
  - _Leverage: src/platform/platform.h, src/utils/memory/memory_manager.h, src/utils/string/string_shims.h_
  - _Requirements: 1, 5, 7
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Embedded Systems Performance Developer | Task: Build the logging core that formats messages per requirement 7 while meeting performance targets from requirements 1 and 5 | Restrictions: Achieve <1% CPU overhead, use fixed buffers, ensure thread safety with minimal locking, output to stdout | Success: Logging functions emit the exact format, performance benchmarks pass, thread-safety validated_

- [ ] 3. Add contextual logging support
  - File: src/utils/logging/logging.c
  - Implement context creation/destruction helpers
  - Expose log_*_ctx functions with service/operation tagging
  - Add optional session ID support for request tracing
  - Purpose: Provide service-aware logging capabilities
  - _Leverage: src/utils/memory/memory_manager.h, existing service patterns_
  - _Requirements: 1, 4
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Context Management Developer | Task: Extend the logging core with context-aware APIs satisfying requirements 1 and 4 | Restrictions: Prevent leaks, keep context thread-safe, maintain component.path.identifier formatting | Success: Context APIs manage lifecycle correctly and contextual messages include service/operation identifiers_

- [ ] 4. Implement logging sanitization helpers
  - File: src/utils/logging/logging_sanitizer.c
  - Create sanitization routines for control characters and UTF-8 validation
  - Add sensitive token redaction helpers
  - Enforce maximum message length with truncation reporting
  - Purpose: Provide reusable sanitization utilities for the logging pipeline
  - _Leverage: src/utils/string/string_shims.h, src/utils/error/error_handling.h_
  - _Requirements: 7, 8
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Security-Focused Systems Developer | Task: Implement the sanitization layer fulfilling requirements 7 and 8 with message length limits, control filtering, and sensitive-data redaction | Restrictions: Use bounded buffers only, avoid heap allocations, keep helpers reusable and documented | Success: Sanitizer enforces policies, exposes testable APIs, documented in Doxygen_

- [ ] 5. Wire sanitization into logging pipeline
  - File: src/utils/logging/logging.c
  - Invoke sanitization helpers before formatting
  - Reject disallowed format specifiers and enforce bounded vsnprintf
  - Emit truncation warnings once per component
  - Purpose: Integrate sanitization layer with logging output
  - _Leverage: src/utils/logging/logging_sanitizer.h_
  - _Requirements: 1, 7, 8
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Security-Oriented Logging Developer | Task: Integrate the sanitization helpers into the logging pipeline to meet requirements 1, 7, and 8 | Restrictions: Preserve performance constraints, ensure warning throttling, maintain thread safety | Success: All log outputs pass through sanitizer, `%n` rejected, truncation warnings throttled_

- [ ] 6. Create sanitization unit tests
  - File: tests/utils/logging_sanitizer_tests.c
  - Test truncation, control filtering, and UTF-8 replacement
  - Validate sensitive token redaction
  - Verify `%n` rejection and bounded formatting
  - Purpose: Ensure sanitization helpers behave as specified
  - _Leverage: CMocka test harness_
  - _Requirements: 8
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Security Test Engineer | Task: Author unit tests covering sanitization acceptance criteria for requirement 8 | Restrictions: Use existing test framework, cover success and failure cases, avoid flaky timing checks | Success: Tests cover all sanitization rules and run cleanly in CI_

- [ ] 7. Extend logging configuration structures
  - File: src/core/config/config.h
  - Add unified logging settings including min level and hostname
  - Document new configuration fields
  - Purpose: Expose unified logging settings through configuration structures
  - _Leverage: Existing configuration patterns_
  - _Requirements: 6, 9
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Configuration Systems Developer | Task: Extend logging_settings to capture the unified logger parameters meeting requirements 6 and 9 | Restrictions: Maintain ABI compatibility, update Doxygen comments, avoid altering unrelated structs | Success: Structures compile, configuration docs updated, no regressions in existing config consumers_

- [ ] 8. Implement configuration runtime support
  - File: src/utils/logging/logging.c
  - Implement log_config_init/log_config_cleanup
  - Cache log level and hostname at startup
  - Add stdout-to-stderr fallback handler
  - Purpose: Complete configuration integration and performance optimisation
  - _Leverage: src/core/lifecycle/config_lifecycle.c, src/platform/platform.h_
  - _Requirements: 5, 6, 9
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Performance-focused C Developer | Task: Implement configuration runtime handling satisfying requirements 5, 6, and 9 including fallback logging and clamping | Restrictions: Ensure constant-time level checks, avoid runtime heap usage after init, throttle fallback error | Success: Configuration initialises correctly, disabled levels return immediately, fallback and clamping behaviour verified_

- [ ] 9. Add format validation tests
  - File: tests/utils/logging_format_tests.c
  - Verify rendered lines match the exact required format
  - Ensure no control characters appear in output
  - Check hostname and component formatting
  - Purpose: Prove compliance with standardized log format
  - _Leverage: Existing unit test infrastructure_
  - _Requirements: 7
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Logging Validation Engineer | Task: Build automated tests asserting the standard log format for requirement 7 | Restrictions: Use deterministic timestamps via dependency injection, avoid external process dependencies | Success: Tests detect format regressions and run quickly in CI_

- [ ] 10. Add logging performance microbenchmarks
  - File: tests/perf/logging_perf_tests.c
  - Benchmark enabled vs disabled level overhead
  - Measure throughput under concurrent logging
  - Purpose: Validate performance requirements
  - _Leverage: Existing perf test harness_
  - _Requirements: 5
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Embedded Performance Engineer | Task: Create microbenchmarks verifying requirement 5 performance targets | Restrictions: Use fixed-size datasets, avoid hardware-specific timers, capture baseline metrics | Success: Benchmarks confirm <1% CPU overhead and constant-time disabled level checks_

- [ ] 11. Migrate platform logging
  - File: src/platform/platform_anyka.c
  - Replace platform_log_* calls with unified log_*
  - Attach platform context metadata
  - Purpose: Begin legacy system migration
  - _Leverage: src/utils/logging/logging.h_
  - _Requirements: 2, 4
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Platform Integration Developer | Task: Replace platform logging calls with unified APIs while adding platform context per requirements 2 and 4 | Restrictions: Preserve behaviour, maintain error handling, ensure compilation | Success: Platform module builds with unified logging and context metadata_

- [ ] 12. Migrate lifecycle logging
  - File: src/core/lifecycle/
  - Replace lifecycle logging macros with unified APIs
  - Add lifecycle stage context
  - Purpose: Migrate lifecycle subsystem to unified logging
  - _Leverage: src/utils/logging/logging.h_
  - _Requirements: 2, 4
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Systems Integration Developer | Task: Migrate lifecycle modules to the unified logger with context per requirements 2 and 4 | Restrictions: Preserve startup/shutdown sequencing, maintain error propagation | Success: Lifecycle logs use unified API with stage context_

- [ ] 13. Migrate device service logging
  - File: src/services/device/onvif_device.c
  - Replace service_log_* calls
  - Add contextual tokens for device operations
  - Purpose: Bring device service onto unified logging
  - _Leverage: src/utils/logging/logging.h_
  - _Requirements: 2, 4
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Device Developer | Task: Migrate device service logging to unified API with contextual metadata | Restrictions: Maintain ONVIF compliance, keep error flows intact | Success: Device service builds with unified logger and passes tests_

- [ ] 14. Migrate media service logging
  - File: src/services/media/onvif_media.c
  - Replace service logging calls
  - Provide media profile context
  - Purpose: Align media service logging
  - _Leverage: src/utils/logging/logging.h_
  - _Requirements: 2, 4
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Media Developer | Task: Adopt unified logging within media service with contextual information | Restrictions: Preserve streaming configuration behaviour | Success: Media service logs through unified API with context_

- [ ] 15. Migrate PTZ service logging
  - File: src/services/ptz/onvif_ptz.c
  - Replace legacy PTZ logging
  - Add PTZ movement context
  - Purpose: Unify PTZ logging
  - _Leverage: src/utils/logging/logging.h_
  - _Requirements: 2, 4
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: PTZ Control Developer | Task: Replace PTZ logging with unified API and contextual metadata | Restrictions: Maintain movement accuracy, keep presets intact | Success: PTZ logs use unified API with contextual data_

- [ ] 16. Migrate imaging service logging
  - File: src/services/imaging/onvif_imaging.c
  - Replace legacy imaging logging
  - Include imaging parameter context
  - Purpose: Unify imaging logging
  - _Leverage: src/utils/logging/logging.h_
  - _Requirements: 2, 4
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Imaging Service Developer | Task: Adopt unified logging within imaging service with contextual metadata | Restrictions: Preserve imaging pipeline behaviour | Success: Imaging service logs through unified API_

- [ ] 17. Migrate snapshot service logging
  - File: src/services/snapshot/onvif_snapshot.c
  - Replace snapshot logging
  - Add snapshot context
  - Purpose: Unify snapshot logging
  - _Leverage: src/utils/logging/logging.h_
  - _Requirements: 2, 4
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Snapshot Service Developer | Task: Migrate snapshot service logging to unified API with contextual details | Restrictions: Maintain snapshot performance | Success: Snapshot service uses unified logging_

- [ ] 18. Migrate HTTP server logging
  - File: src/networking/http/http_server.c
  - Replace HTTP logging calls
  - Add request context (method, URI, client)
  - Purpose: Migrate HTTP layer
  - _Leverage: src/utils/logging/logging.h_
  - _Requirements: 2, 4
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: HTTP Networking Developer | Task: Adopt unified logging in HTTP server with contextual metadata | Restrictions: Maintain performance, avoid blocking | Success: HTTP server logs use unified API with context_

- [ ] 19. Migrate HTTP parser logging
  - File: src/networking/http/http_parser.c
  - Replace parser logging
  - Log parsing errors with context
  - Purpose: Complete HTTP parser migration
  - _Leverage: src/utils/logging/logging.h_
  - _Requirements: 2, 4
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Protocol Parser Developer | Task: Replace HTTP parser logging with unified API including request context | Restrictions: Maintain parsing accuracy | Success: Parser logs align with unified system_

- [ ] 20. Migrate RTSP logging
  - File: src/networking/rtsp/
  - Replace RTSP logging
  - Add session identifiers
  - Purpose: Unify RTSP logging
  - _Leverage: src/utils/logging/logging.h_
  - _Requirements: 2, 4
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: RTSP Streaming Developer | Task: Update RTSP components to use unified logging with session context | Restrictions: Maintain streaming performance | Success: RTSP logs use unified API with context_

- [ ] 21. Migrate WS-Discovery logging
  - File: src/networking/discovery/ws_discovery.c
  - Replace discovery logging
  - Add network context
  - Purpose: Unify discovery logging
  - _Leverage: src/utils/logging/logging.h_
  - _Requirements: 2, 4
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Network Discovery Developer | Task: Adopt unified logging in discovery module with contextual network data | Restrictions: Maintain multicast behaviour | Success: Discovery logging uses unified API_

- [ ] 22. Migrate connection infrastructure logging
  - File: src/networking/common/
  - Replace connection manager logging
  - Add client context and metrics
  - Purpose: Unify network infrastructure logging
  - _Leverage: src/utils/logging/logging.h_
  - _Requirements: 2, 4, 5
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Network Infrastructure Developer | Task: Migrate connection management, epoll, and buffer pool logging to unified API with contextual data while preserving performance | Restrictions: Maintain throughput, avoid introducing locks | Success: Infrastructure components log through unified system without regressions_

- [ ] 23. Migrate ONVIF protocol response logging
  - File: src/protocol/response/onvif_service_handler.c
  - Replace protocol logging
  - Include SOAP operation context
  - Purpose: Unify protocol layer logging
  - _Leverage: src/utils/logging/logging.h_
  - _Requirements: 2, 4
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Protocol Developer | Task: Update protocol response handler logging to unified API with contextual details | Restrictions: Maintain protocol behaviour | Success: Protocol handler logs unified_

- [ ] 24. Migrate error handling logging
  - File: src/utils/error/error_handling.c
  - Replace error logging
  - Include error context
  - Purpose: Unify error handling logging
  - _Leverage: src/utils/logging/logging.h_
  - _Requirements: 2, 3, 4
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Error Handling Specialist | Task: Replace error handling logging with unified API while supporting debugging requirement 3 | Restrictions: Keep error propagation and return codes intact | Success: Error handling uses unified API and integrates with debugging workflow_

- [ ] 25. Migrate memory manager logging
  - File: src/utils/memory/memory_manager.c
  - Replace memory logging
  - Add allocation context
  - Purpose: Unify memory management logging
  - _Leverage: src/utils/logging/logging.h_
  - _Requirements: 2, 4
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Memory Management Developer | Task: Update memory manager logging to unified API with contextual details | Restrictions: Maintain memory performance | Success: Memory manager logs unified_

- [ ] 26. Migrate security hardening logging
  - File: src/utils/security/security_hardening.c
  - Replace security logging
  - Include audit context
  - Purpose: Unify security logging
  - _Leverage: src/utils/logging/logging.h_
  - _Requirements: 2, 4, 8
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Security Engineer | Task: Adopt unified logging in security module with sanitisation considerations | Restrictions: Maintain security posture, avoid leaking sensitive data | Success: Security logging uses unified API with redaction_

- [ ] 27. Remove legacy platform logging implementation
  - File: src/utils/logging/platform_logging.c
  - Delete legacy platform logging sources
  - Clean include references
  - Purpose: Eliminate legacy logging code
  - _Leverage: Project build system_
  - _Requirements: 2
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Build System Maintainer | Task: Remove legacy platform logging files after migration | Restrictions: Ensure no remaining references, maintain build stability | Success: Legacy files removed and build succeeds_

- [ ] 28. Remove legacy service logging implementation
  - File: src/utils/logging/service_logging.c
  - Delete service logging sources
  - Update references
  - Purpose: Complete legacy cleanup
  - _Leverage: Project build system_
  - _Requirements: 2
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Build System Maintainer | Task: Remove service logging legacy code | Restrictions: Verify migration complete before deletion | Success: Legacy service logging removed_

- [ ] 29. Update build system for unified logging
  - File: cross-compile/onvif/Makefile
  - Add logging.c and logging_sanitizer.c to build
  - Remove legacy logging objects
  - Purpose: Ensure build only includes unified logging
  - _Leverage: Existing Makefile patterns_
  - _Requirements: 2, 10
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Build Engineer | Task: Update build to include unified logging and exclude legacy code | Restrictions: Maintain build options and dependencies | Success: Build compiles unified logging without legacy artefacts_

- [ ] 30. Create logging migration script
  - File: tools/logging_migration.py
  - Automate conversion of platform/service logging calls
  - Detect printf-style logging
  - Purpose: Provide migration automation
  - _Leverage: Python tooling, regex patterns_
  - _Requirements: 10, 2
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Automation Engineer | Task: Develop migration script to rewrite legacy logging to unified API | Restrictions: Preserve include ordering, support dry-run mode | Success: Script rewrites majority of legacy calls and outputs report_

- [ ] 31. Run migration script and resolve leftovers
  - File: tools/logging_migration_report.md
  - Execute migration script
  - Manually update remaining logging calls
  - Document results
  - Purpose: Ensure clean break from legacy logging
  - _Leverage: tools/logging_migration.py_
  - _Requirements: 2, 10
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Maintenance Engineer | Task: Run the migration tooling and resolve remaining manual conversions, documenting outcomes | Restrictions: Verify compilation after changes, keep report updated | Success: No legacy logging references remain and report filed_

- [ ] 32. Update documentation and generate docs
  - File: docs/ and Doxygen
  - Include new logging module in Doxygen
  - Update developer guidance
  - Purpose: Document the unified logging system
  - _Leverage: Existing documentation workflow_
  - _Requirements: 1, 7, 8, 9
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Technical Documentation Specialist | Task: Refresh documentation to cover unified logging features | Restrictions: Maintain documentation standards, regenerate HTML | Success: Documentation builds cleanly and reflects new logging system_

- [ ] 33. Create comprehensive unit tests
  - File: tests/utils/test_logging.c
  - Test logging API happy paths
  - Cover error handling and fallback
  - Purpose: Ensure reliability via unit tests
  - _Leverage: Existing test framework_
  - _Requirements: 1, 4, 5, 6, 7, 8, 9
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Unit Test Engineer | Task: Develop unit tests covering unified logging functionality | Restrictions: Avoid flaky timing assumptions, cover sanitisation and configuration | Success: Unit tests pass and provide high coverage_

- [ ] 34. Perform integration testing
  - File: tests/integration/logging_integration_tests.c
  - Validate ONVIF workflows with unified logging
  - Check log format and performance
  - Verify sanitisation under realistic traffic
  - Purpose: Confirm end-to-end behaviour
  - _Leverage: Integration test suite_
  - _Requirements: 1, 2, 4, 5, 6, 7, 8, 9
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Integration Test Engineer | Task: Execute integration tests ensuring unified logging meets requirements | Restrictions: Use existing test harness, capture metrics | Success: Integration tests pass with format/performance compliance_

- [ ] 35. Validate production readiness
  - File: Deployment tests
  - Cross-compile and deploy to device
  - Verify runtime logging on hardware
  - Monitor performance and stability
  - Purpose: Final production validation
  - _Leverage: SD card deployment workflow_
  - _Requirements: 1, 2, 4, 5, 6, 7, 8, 9, 10
  - _Prompt: Implement the task for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Production Validation Engineer | Task: Perform hardware validation to ensure unified logging is production-ready | Restrictions: Test on Anyka hardware, monitor memory/CPU usage | Success: Hardware tests pass with logging functioning and no regressions_

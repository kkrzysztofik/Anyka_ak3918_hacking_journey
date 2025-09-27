# Tasks Document

- [ ] 1. Create core logging implementation in src/utils/logging/logging.h
  - File: src/utils/logging/logging.h
  - Define function-based logging API with log levels (error, warning, info, debug)
  - Create log context structure for service and operation tracking
  - Purpose: Establish unified logging interface for entire ONVIF project
  - _Leverage: src/core/config/config.h, src/utils/string/string_shims.h_
  - _Requirements: 1.1, 1.2, 1.3_
  - _Prompt: Role: Systems Programming Developer specializing in C embedded systems and logging frameworks | Task: Implement core logging header file for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Create comprehensive logging API following requirements 1.1, 1.2, and 1.3, defining function-based interface with log levels and context structures, leveraging existing configuration patterns from src/core/config/config.h and string utilities from src/utils/string/string_shims.h | Restrictions: Must use C99 standards, avoid macro-heavy approaches, ensure thread safety, do not modify existing config structures | Success: Header compiles without errors, provides complete logging API, follows project coding standards from AGENTS.md, includes comprehensive Doxygen documentation_

- [ ] 2. Implement core logging functionality in src/utils/logging/logging.c
  - File: src/utils/logging/logging.c
  - Implement log level functions with standardized format output
  - Add timestamp generation with millisecond precision
  - Add hostname resolution and component path building
  - Purpose: Provide complete logging implementation with performance optimization
  - _Leverage: src/platform/platform.h, src/utils/memory/memory_manager.h, src/utils/string/string_shims.h_
  - _Requirements: 1.1, 1.4, 1.7_
  - _Prompt: Role: Systems Programming Developer with expertise in C performance optimization and embedded systems | Task: Implement logging functionality for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Create complete logging implementation following requirements 1.1, 1.4, and 1.7, implementing standardized log format "YYYY-MM-DD HH:MM:SS,mmm LEVEL [HOSTNAME] component.path.identifier Message text", using platform utilities for hostname and memory management patterns | Restrictions: Must achieve <1% CPU overhead, use fixed memory buffers only, ensure thread safety with minimal locking, output only to stdout | Success: All logging functions work correctly, format matches specification exactly, performance requirements met, thread-safe operation verified_

- [ ] 3. Implement contextual logging support in src/utils/logging/logging.c
  - File: src/utils/logging/logging.c (continue from task 2)
  - Add context creation and management functions
  - Implement contextual logging functions with service/operation tracking
  - Add session ID support for request tracing
  - Purpose: Enable service-aware logging for ONVIF operations
  - _Leverage: src/utils/memory/memory_manager.h, existing service patterns_
  - _Requirements: 1.2, 1.4_
  - _Prompt: Role: Systems Programming Developer with expertise in C context management and service architectures | Task: Implement contextual logging for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Add context management and contextual logging functions following requirements 1.2 and 1.4, enabling service and operation tracking for ONVIF services, using existing memory management patterns | Restrictions: Must handle context lifecycle properly, avoid memory leaks, ensure context thread safety, maintain component.path.identifier format | Success: Context creation/destruction works correctly, contextual logging includes proper service identification, memory management is leak-free, supports concurrent contexts_

- [ ] 4. Integrate with configuration system in src/core/config/config.h
  - File: src/core/config/config.h (modify existing)
  - Extend logging_settings structure for unified logging configuration
  - Add log level and hostname configuration options
  - Purpose: Enable configuration-driven logging behavior
  - _Leverage: existing configuration patterns, src/core/lifecycle/config_lifecycle.c_
  - _Requirements: 1.6_
  - _Prompt: Role: Configuration Systems Developer with expertise in C configuration management and lifecycle patterns | Task: Integrate logging configuration for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Extend configuration system following requirement 1.6, adding unified logging settings to existing logging_settings structure, integrating with configuration lifecycle management | Restrictions: Must maintain backward compatibility during migration, follow existing configuration patterns, do not break existing config loading | Success: Configuration loading works correctly, logging settings are properly applied, integration with lifecycle management is seamless_

- [ ] 5. Add configuration integration in src/utils/logging/logging.c
  - File: src/utils/logging/logging.c (continue from previous tasks)
  - Implement log_config_init() and log_config_cleanup() functions
  - Add log level filtering with early return optimization
  - Add hostname caching for performance
  - Purpose: Complete configuration integration and performance optimization
  - _Leverage: src/core/config/config.h, src/core/lifecycle/config_lifecycle.c_
  - _Requirements: 1.6, 1.5_
  - _Prompt: Role: Performance-focused C Developer with expertise in embedded systems optimization | Task: Complete configuration integration for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Implement configuration functions and performance optimizations following requirements 1.6 and 1.5, adding early return optimization for disabled log levels and hostname caching | Restrictions: Must achieve zero-cost for disabled levels, maintain thread safety, ensure graceful fallback for configuration errors | Success: Configuration loading is robust, disabled log levels have zero performance impact, hostname caching works correctly, early return optimization verified_

- [ ] 6. Migrate platform logging in src/platform/platform_anyka.c
  - File: src/platform/platform_anyka.c
  - Replace platform_log_* calls with unified log_* functions
  - Update include statements to use new logging API
  - Add appropriate context for platform operations
  - Purpose: Begin systematic migration from legacy logging systems
  - _Leverage: src/utils/logging/logging.h, existing platform abstraction patterns_
  - _Requirements: 2.1_
  - _Prompt: Role: Platform Integration Developer with expertise in hardware abstraction and migration strategies | Task: Migrate platform logging for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Replace legacy platform logging calls following requirement 2.1, updating all platform_log_* calls to use unified logging API with appropriate platform context | Restrictions: Must maintain existing functionality, do not change platform operation behavior, ensure proper error handling | Success: All platform logging migrated correctly, functionality unchanged, new logging format applied, no compilation errors_

- [ ] 7. Migrate lifecycle management logging
  - Files: src/core/lifecycle/config_lifecycle.c, src/core/lifecycle/network_lifecycle.c, src/core/lifecycle/signal_lifecycle.c, src/core/lifecycle/service_manager.c
  - Replace platform logging calls with unified logging
  - Add lifecycle context to log messages
  - Update error handling to use new logging format
  - Purpose: Complete platform layer migration
  - _Leverage: src/utils/logging/logging.h, lifecycle management patterns_
  - _Requirements: 2.1_
  - _Prompt: Role: Systems Integration Developer with expertise in lifecycle management and service orchestration | Task: Migrate lifecycle logging for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Update all lifecycle modules following requirement 2.1, replacing legacy logging with unified API and adding appropriate lifecycle context to messages | Restrictions: Must preserve lifecycle behavior, maintain error propagation, ensure startup/shutdown logging continuity | Success: All lifecycle modules use unified logging, lifecycle events properly logged with context, system startup/shutdown behavior unchanged_

- [ ] 8. Migrate ONVIF Device Service logging in src/services/device/onvif_device.c
  - File: src/services/device/onvif_device.c
  - Replace service logging calls with contextual unified logging
  - Add device service context for all operations
  - Update capability request logging with operation tracking
  - Purpose: Begin service layer migration with contextual logging
  - _Leverage: src/utils/logging/logging.h, existing device service patterns_
  - _Requirements: 2.2_
  - _Prompt: Role: ONVIF Protocol Developer with expertise in device services and compliance | Task: Migrate device service logging for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Update device service logging following requirement 2.2, replacing service-specific logging with contextual unified logging for all device operations | Restrictions: Must maintain ONVIF compliance, preserve service functionality, ensure proper error reporting | Success: Device service uses contextual logging, ONVIF operations properly traced, service functionality unchanged, compliance maintained_

- [ ] 9. Migrate ONVIF Media Service logging in src/services/media/onvif_media.c
  - File: src/services/media/onvif_media.c
  - Replace service logging with contextual unified logging
  - Add media service context for profile and streaming operations
  - Update video/audio configuration logging with session tracking
  - Purpose: Complete media service migration with operation tracing
  - _Leverage: src/utils/logging/logging.h, media service patterns_
  - _Requirements: 2.2_
  - _Prompt: Role: Media Streaming Developer with expertise in ONVIF media services and video processing | Task: Migrate media service logging for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Update media service logging following requirement 2.2, implementing contextual logging for all media operations including profile management and streaming | Restrictions: Must not affect streaming performance, maintain media operation timing, preserve video/audio quality | Success: Media service uses contextual logging, streaming operations properly traced, performance impact minimal, media functionality unchanged_

- [ ] 10. Migrate ONVIF PTZ Service logging in src/services/ptz/onvif_ptz.c
  - File: src/services/ptz/onvif_ptz.c
  - Replace service logging with contextual unified logging
  - Add PTZ service context for movement and preset operations
  - Update continuous movement logging with session tracking
  - Purpose: Complete PTZ service migration with operation context
  - _Leverage: src/utils/logging/logging.h, PTZ service patterns_
  - _Requirements: 2.2_
  - _Prompt: Role: PTZ Control Developer with expertise in camera movement systems and ONVIF PTZ services | Task: Migrate PTZ service logging for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Update PTZ service logging following requirement 2.2, implementing contextual logging for all PTZ operations including movement, presets, and continuous control | Restrictions: Must not affect PTZ response times, maintain movement precision, preserve PTZ operation timing | Success: PTZ service uses contextual logging, movement operations properly traced, response times unchanged, PTZ functionality preserved_

- [ ] 11. Migrate ONVIF Imaging Service logging in src/services/imaging/onvif_imaging.c
  - File: src/services/imaging/onvif_imaging.c
  - Replace service logging with contextual unified logging
  - Add imaging service context for parameter adjustments
  - Update brightness/contrast/saturation logging with operation tracking
  - Purpose: Complete imaging service migration
  - _Leverage: src/utils/logging/logging.h, imaging service patterns_
  - _Requirements: 2.2_
  - _Prompt: Role: Image Processing Developer with expertise in camera imaging controls and ONVIF imaging services | Task: Migrate imaging service logging for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Update imaging service logging following requirement 2.2, implementing contextual logging for all imaging parameter adjustments and image quality controls | Restrictions: Must preserve image quality settings, maintain parameter validation, ensure imaging operation consistency | Success: Imaging service uses contextual logging, parameter adjustments properly traced, image quality unchanged, imaging functionality preserved_

- [ ] 12. Migrate ONVIF Snapshot Service logging in src/services/snapshot/onvif_snapshot.c
  - File: src/services/snapshot/onvif_snapshot.c
  - Replace service logging with contextual unified logging
  - Add snapshot service context for image capture operations
  - Update snapshot generation logging with session tracking
  - Purpose: Complete snapshot service migration
  - _Leverage: src/utils/logging/logging.h, snapshot service patterns_
  - _Requirements: 2.2_
  - _Prompt: Role: Image Capture Developer with expertise in snapshot generation and ONVIF snapshot services | Task: Migrate snapshot service logging for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Update snapshot service logging following requirement 2.2, implementing contextual logging for all snapshot operations and image capture processes | Restrictions: Must maintain snapshot quality, preserve capture timing, ensure snapshot generation reliability | Success: Snapshot service uses contextual logging, capture operations properly traced, snapshot quality unchanged, capture functionality preserved_

- [ ] 13. Migrate HTTP server logging in src/networking/http/http_server.c
  - File: src/networking/http/http_server.c
  - Replace logging calls with contextual unified logging
  - Add HTTP server context for request processing
  - Update connection management logging with client identification
  - Purpose: Begin network layer migration
  - _Leverage: src/utils/logging/logging.h, HTTP server patterns_
  - _Requirements: 2.2_
  - _Prompt: Role: Network Programming Developer with expertise in HTTP servers and connection management | Task: Migrate HTTP server logging for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Update HTTP server logging following requirement 2.2, implementing contextual logging for request processing and connection management with client identification | Restrictions: Must not affect HTTP performance, maintain request handling speed, preserve connection reliability | Success: HTTP server uses contextual logging, requests properly traced with client context, performance unchanged, HTTP functionality preserved_

- [ ] 14. Migrate HTTP parser logging in src/networking/http/http_parser.c
  - File: src/networking/http/http_parser.c
  - Replace logging calls with unified logging
  - Add parser context for HTTP message processing
  - Update parsing error logging with request details
  - Purpose: Complete HTTP layer migration
  - _Leverage: src/utils/logging/logging.h, HTTP parser patterns_
  - _Requirements: 2.2_
  - _Prompt: Role: Protocol Parser Developer with expertise in HTTP message parsing and error handling | Task: Migrate HTTP parser logging for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Update HTTP parser logging following requirement 2.2, implementing unified logging for all parsing operations and error conditions | Restrictions: Must maintain parsing accuracy, preserve error detection, ensure parsing performance | Success: HTTP parser uses unified logging, parsing operations properly logged, error handling improved, parsing functionality unchanged_

- [ ] 15. Migrate RTSP multistream logging in src/networking/rtsp/rtsp_multistream.c
  - File: src/networking/rtsp/rtsp_multistream.c
  - Replace logging calls with contextual unified logging
  - Add RTSP context for streaming operations
  - Update stream management logging with session tracking
  - Purpose: Complete RTSP layer migration
  - _Leverage: src/utils/logging/logging.h, RTSP streaming patterns_
  - _Requirements: 2.2_
  - _Prompt: Role: RTSP Streaming Developer with expertise in real-time media streaming and session management | Task: Migrate RTSP multistream logging for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Update RTSP streaming logging following requirement 2.2, implementing contextual logging for all streaming operations and session management | Restrictions: Must not affect streaming performance, maintain real-time characteristics, preserve streaming quality | Success: RTSP streaming uses contextual logging, streaming operations properly traced, performance unchanged, streaming functionality preserved_

- [ ] 16. Migrate WS-Discovery logging in src/networking/discovery/ws_discovery.c
  - File: src/networking/discovery/ws_discovery.c
  - Replace logging calls with unified logging
  - Add discovery context for network operations
  - Update device discovery logging with network details
  - Purpose: Complete discovery layer migration
  - _Leverage: src/utils/logging/logging.h, discovery patterns_
  - _Requirements: 2.2_
  - _Prompt: Role: Network Discovery Developer with expertise in WS-Discovery protocol and multicast networking | Task: Migrate WS-Discovery logging for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Update discovery service logging following requirement 2.2, implementing unified logging for all discovery operations and network events | Restrictions: Must maintain discovery timing, preserve multicast reliability, ensure network compatibility | Success: Discovery service uses unified logging, discovery operations properly logged, network functionality unchanged, discovery reliability preserved_

- [ ] 17. Migrate connection management logging
  - Files: src/networking/common/connection_manager.c, src/networking/common/thread_pool.c, src/networking/common/epoll_server.c, src/networking/common/buffer_pool.c
  - Replace logging calls with unified logging
  - Add connection context for client tracking
  - Update performance logging with operation metrics
  - Purpose: Complete network infrastructure migration
  - _Leverage: src/utils/logging/logging.h, connection management patterns_
  - _Requirements: 2.2_
  - _Prompt: Role: Network Infrastructure Developer with expertise in connection pooling and high-performance networking | Task: Migrate connection management logging for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Update all network infrastructure logging following requirement 2.2, implementing unified logging for connection management, thread pools, and buffer management | Restrictions: Must maintain network performance, preserve connection handling efficiency, ensure resource management reliability | Success: Network infrastructure uses unified logging, connection operations properly traced, performance metrics maintained, network functionality unchanged_

- [ ] 18. Migrate ONVIF service handler logging in src/protocol/response/onvif_service_handler.c
  - File: src/protocol/response/onvif_service_handler.c
  - Replace logging calls with contextual unified logging
  - Add protocol handler context for SOAP processing
  - Update response generation logging with timing information
  - Purpose: Complete protocol layer migration
  - _Leverage: src/utils/logging/logging.h, ONVIF protocol patterns_
  - _Requirements: 2.2_
  - _Prompt: Role: ONVIF Protocol Developer with expertise in SOAP message processing and response generation | Task: Migrate protocol handler logging for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Update service handler logging following requirement 2.2, implementing contextual logging for all SOAP processing and response generation with timing metrics | Restrictions: Must maintain ONVIF compliance, preserve protocol timing, ensure response accuracy | Success: Protocol handler uses contextual logging, SOAP operations properly traced, compliance maintained, protocol functionality preserved_

- [ ] 19. Migrate error handling logging in src/utils/error/error_handling.c
  - File: src/utils/error/error_handling.c
  - Replace logging calls with unified logging
  - Update error reporting to use contextual information
  - Add error context for better debugging
  - Purpose: Complete error handling integration
  - _Leverage: src/utils/logging/logging.h, error handling patterns_
  - _Requirements: 2.2_
  - _Prompt: Role: Error Handling Specialist with expertise in debugging and error reporting systems | Task: Migrate error handling logging for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Update error handling logging following requirement 2.2, implementing unified logging for all error conditions with enhanced context for debugging | Restrictions: Must preserve error detection, maintain error propagation, ensure error handling reliability | Success: Error handling uses unified logging, error conditions properly traced with context, debugging capability improved, error handling functionality preserved_

- [ ] 20. Migrate memory management logging in src/utils/memory/memory_manager.c
  - File: src/utils/memory/memory_manager.c
  - Replace logging calls with unified logging
  - Add memory context for allocation tracking
  - Update memory usage logging with performance metrics
  - Purpose: Complete memory management integration
  - _Leverage: src/utils/logging/logging.h, memory management patterns_
  - _Requirements: 2.2_
  - _Prompt: Role: Memory Management Developer with expertise in embedded systems and performance optimization | Task: Migrate memory management logging for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Update memory management logging following requirement 2.2, implementing unified logging for all memory operations with performance tracking | Restrictions: Must not affect memory performance, maintain allocation efficiency, preserve memory safety | Success: Memory management uses unified logging, memory operations properly traced, performance impact minimal, memory functionality unchanged_

- [ ] 21. Migrate security hardening logging in src/utils/security/security_hardening.c
  - File: src/utils/security/security_hardening.c
  - Replace logging calls with unified logging
  - Add security context for audit trail
  - Update security event logging with detailed information
  - Purpose: Complete security integration
  - _Leverage: src/utils/logging/logging.h, security patterns_
  - _Requirements: 2.2_
  - _Prompt: Role: Security Developer with expertise in security hardening and audit logging | Task: Migrate security logging for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Update security hardening logging following requirement 2.2, implementing unified logging for all security events with comprehensive audit information | Restrictions: Must maintain security effectiveness, preserve security event detection, ensure audit trail completeness | Success: Security hardening uses unified logging, security events properly traced with audit context, security functionality preserved, audit trail enhanced_

- [ ] 22. Remove platform logging system files
  - Files: src/utils/logging/platform_logging.h, src/utils/logging/platform_logging.c
  - Delete legacy platform logging implementation
  - Remove from build system (Makefile)
  - Clean up any remaining references
  - Purpose: Clean break from legacy logging systems
  - _Leverage: build system patterns, dependency analysis_
  - _Requirements: 2.1_
  - _Prompt: Role: Build System Maintainer with expertise in dependency management and clean refactoring | Task: Remove platform logging system for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Delete legacy platform logging files following requirement 2.1, ensuring complete removal from build system and all references | Restrictions: Must verify no remaining dependencies, ensure clean compilation, maintain build system integrity | Success: Platform logging files completely removed, build system updated correctly, no compilation errors, no remaining references_

- [ ] 23. Remove service logging system files
  - Files: src/utils/logging/service_logging.h, src/utils/logging/service_logging.c
  - Delete legacy service logging implementation
  - Remove from build system
  - Verify no remaining service logging references
  - Purpose: Continue legacy system cleanup
  - _Leverage: build system patterns, dependency analysis_
  - _Requirements: 2.1_
  - _Prompt: Role: Build System Maintainer with expertise in legacy code removal and dependency cleanup | Task: Remove service logging system for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Delete legacy service logging files following requirement 2.1, ensuring complete removal and verification of no remaining dependencies | Restrictions: Must verify complete migration before removal, ensure no broken references, maintain service functionality | Success: Service logging files completely removed, build system clean, services continue to function correctly, no compilation issues_

- [ ] 24. Remove generic logging utils files
  - Files: src/utils/logging/logging_utils.h, src/utils/logging/logging_utils.c
  - Delete legacy utility logging implementation
  - Remove from build system
  - Clean up utility module references
  - Purpose: Continue legacy cleanup
  - _Leverage: build system patterns, utility analysis_
  - _Requirements: 2.1_
  - _Prompt: Role: Build System Maintainer with expertise in utility module cleanup and dependency management | Task: Remove utility logging system for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Delete legacy utility logging files following requirement 2.1, ensuring complete cleanup of utility logging references | Restrictions: Must verify utility migration complete, ensure no broken dependencies, maintain utility functionality | Success: Utility logging files completely removed, utility modules function correctly, build system clean, no remaining legacy references_

- [ ] 25. Clean up direct stdio usage
  - Files: Multiple files with printf(), fprintf(), perror() calls
  - Search and replace remaining direct stdio logging
  - Replace with appropriate unified logging calls
  - Verify no direct output remains for logging
  - Purpose: Complete elimination of fragmented logging
  - _Leverage: search tools, unified logging API_
  - _Requirements: 2.1_
  - _Prompt: Role: Code Cleanup Specialist with expertise in systematic code refactoring and pattern replacement | Task: Clean up direct stdio usage for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Replace all remaining direct stdio logging calls following requirement 2.1, ensuring complete elimination of printf/fprintf/perror usage for logging purposes | Restrictions: Must preserve non-logging stdio usage, maintain output behavior, ensure proper error handling | Success: No direct stdio logging remains, all logging uses unified API, output behavior unchanged, error handling preserved_

- [ ] 26. Update build system configuration
  - File: cross-compile/onvif/Makefile
  - Update to include only logging.c in build
  - Remove legacy logging object files
  - Update dependencies and compile commands
  - Purpose: Finalize build system for unified logging
  - _Leverage: existing Makefile patterns, dependency management_
  - _Requirements: 2.1_
  - _Prompt: Role: Build Engineer with expertise in Makefile configuration and dependency management | Task: Update build system for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Configure build system following requirement 2.1, updating Makefile to include only unified logging implementation and removing all legacy logging dependencies | Restrictions: Must maintain build functionality, preserve compilation settings, ensure clean builds | Success: Build system includes only unified logging, clean compilation achieved, all dependencies correctly configured, build performance maintained_

- [ ] 27. Update documentation and generate docs
  - Files: Doxygen configuration, documentation updates
  - Update Doxygen to include new logging module
  - Generate updated documentation with make docs
  - Update any developer documentation references
  - Purpose: Complete documentation for unified logging
  - _Leverage: existing documentation patterns, Doxygen configuration_
  - _Requirements: All requirements_
  - _Prompt: Role: Technical Documentation Specialist with expertise in Doxygen and API documentation | Task: Update documentation for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Generate comprehensive documentation covering all requirements, updating Doxygen configuration and creating developer guides for the unified logging system | Restrictions: Must maintain documentation standards, ensure API documentation completeness, preserve existing documentation structure | Success: Documentation generated successfully, API fully documented, developer guides updated, documentation follows project standards_

- [ ] 28. Create comprehensive unit tests
  - File: cross-compile/onvif/tests/test_logging.c
  - Create unit tests for all logging functions
  - Test log level filtering and contextual logging
  - Test thread safety and error handling
  - Purpose: Ensure logging system reliability
  - _Leverage: existing test framework, test patterns_
  - _Requirements: All requirements_
  - _Prompt: Role: Test Engineer with expertise in C unit testing and embedded systems validation | Task: Create unit tests for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Develop comprehensive unit tests covering all requirements, testing core functionality, thread safety, error handling, and performance characteristics | Restrictions: Must test all API functions, ensure test isolation, validate thread safety, test error conditions | Success: All logging functions thoroughly tested, thread safety verified, error handling validated, test coverage complete_

- [ ] 29. Perform integration testing
  - Test complete ONVIF functionality with unified logging
  - Verify log format compliance across all services
  - Test performance impact on video streaming
  - Validate configuration loading and error handling
  - Purpose: Ensure system-wide logging integration
  - _Leverage: existing integration test patterns, ONVIF test suite_
  - _Requirements: All requirements_
  - _Prompt: Role: Integration Test Engineer with expertise in ONVIF testing and system validation | Task: Perform integration testing for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Execute comprehensive integration tests covering all requirements, validating ONVIF functionality, log format compliance, and performance characteristics | Restrictions: Must test complete system functionality, validate ONVIF compliance, ensure performance requirements met | Success: All ONVIF services function correctly, log format compliance verified, performance impact within limits, integration complete_

- [ ] 30. Validate production readiness
  - Build with cross-compilation toolchain
  - Deploy to SD card for device testing
  - Test on actual Anyka hardware
  - Verify performance and stability
  - Purpose: Final validation for production deployment
  - _Leverage: SD card deployment process, hardware testing_
  - _Requirements: All requirements_
  - _Prompt: Role: Production Validation Engineer with expertise in embedded systems deployment and hardware testing | Task: Validate production readiness for spec logging-unification, first run spec-workflow-guide to get the workflow guide then implement the task: Complete final validation covering all requirements, testing on actual hardware and validating production deployment readiness | Restrictions: Must test on target hardware, validate performance requirements, ensure stability under load | Success: System functions correctly on target hardware, performance requirements met, production deployment validated, stability confirmed_
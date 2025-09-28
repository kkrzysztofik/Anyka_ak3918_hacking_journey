# Tasks Document

- [x] 1. Establish baseline and audit current implementation
  - File: cross-compile/onvif/src/services/device/onvif_device.c
  - Analyze current memory allocation patterns and identify optimization targets
  - Document existing smart response builder infrastructure status
  - Purpose: Establish current state and optimization targets for memory refactoring
  - _Leverage: existing smart response builders, buffer pool infrastructure_
  - _Requirements: 1_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Memory Analysis Engineer with C allocation tracking expertise | Task: Analyze current memory allocation patterns in device service and document baseline state | Restrictions: Analysis only, no code modifications, document exact allocation patterns | Success: Complete baseline documented with allocation patterns and smart builder status | Instructions: Mark [-] when starting, analyze allocation patterns, document findings, mark [x] when baseline established_

- [x] 2. Remove HTTP-ONVIF adapter layer
  - File: cross-compile/onvif/src/networking/http/http_onvif_adapter.c
  - Remove obsolete adapter layer between HTTP server and ONVIF services
  - Update device service to accept http_request_t directly
  - Purpose: Eliminate unnecessary adapter layer and simplify HTTP processing
  - _Leverage: existing HTTP request structure, device service handlers_
  - _Requirements: 5_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: HTTP Architecture Engineer with adapter removal expertise | Task: Remove HTTP-ONVIF adapter layer and update device service for direct HTTP processing | Restrictions: Must maintain HTTP functionality, preserve ONVIF compliance, follow existing patterns | Success: Adapter layer removed, device service accepts http_request_t, HTTP functionality preserved | Instructions: Mark [-] when starting, remove adapter files, update service handlers, test HTTP processing, mark [x] when adapter removal complete_

- [x] 3. Complete unused service utilities cleanup
  - File: cross-compile/onvif/src/services/common/
  - Remove unused utility files and dead code from service layer
  - Clean up service directory structure and dependencies
  - Purpose: Eliminate dead code and unused utilities from service layer
  - _Leverage: existing service patterns, dependency analysis_
  - _Requirements: 3_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Code Cleanup Engineer with dead code elimination expertise | Task: Remove unused service utilities and clean up service layer dependencies | Restrictions: Must preserve all active functionality, verify no dependencies broken, maintain service patterns | Success: All unused utilities removed, service layer cleaned, no functionality broken | Instructions: Mark [-] when starting, identify unused utilities, remove dead code, verify dependencies, mark [x] when cleanup complete_

- [x] 4. Eliminate remaining large allocations in device service
  - File: cross-compile/onvif/src/services/device/onvif_device.c
  - Replace final 2×ONVIF_RESPONSE_BUFFER_SIZE (128KB) allocations with dynamic allocation
  - Use existing smart response builders for device info and capabilities handlers
  - Purpose: Complete elimination of large static allocations in device service
  - _Leverage: existing smart response builders, build_response_with_dynamic_buffer function_
  - _Requirements: 1_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Memory Optimization Specialist with C expertise | Task: Locate and replace the final 2×ONVIF_RESPONSE_BUFFER_SIZE (128KB) allocations in device service handlers with dynamic allocation using existing smart response builders | Restrictions: Must use existing build_response_with_dynamic_buffer function, maintain exact ONVIF response format, follow AGENTS.md coding standards | Success: Zero references to ONVIF_RESPONSE_BUFFER_SIZE in device service, all handlers use dynamic allocation, ONVIF functionality preserved | Instructions: Mark [-] when starting, grep for ONVIF_RESPONSE_BUFFER_SIZE, replace each with smart response builder call, test functionality, mark [x] when eliminated_

- [x] 5. Analyze buffer pool utilization patterns
  - File: cross-compile/onvif/src/networking/common/buffer_pool.c
  - Add debug logging to buffer_pool_acquire() and buffer_pool_release() functions
  - Identify why buffer pool utilization is currently 0% despite infrastructure availability
  - Purpose: Understand buffer pool usage patterns for optimization
  - _Leverage: existing buffer pool infrastructure, platform_logging.h_
  - _Requirements: 1.2_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Memory Analysis Engineer with debugging expertise | Task: Add detailed logging to buffer pool operations to track acquisition patterns and identify why utilization is 0% | Restrictions: Use platform_logging.h only, do not modify pool logic, add logging without performance impact | Success: Comprehensive logging shows pool acquisition patterns, clear identification of why pools unused | Instructions: Mark [-] when starting, add LOG_DEBUG calls to pool functions, run device operations, analyze log output, mark [x] when usage patterns documented_

- [x] 6. Implement buffer pool usage for device service XML processing
  - File: cross-compile/onvif/src/services/device/onvif_device.c
  - Replace malloc() calls in device handlers with buffer_pool_acquire() for temporary XML buffers
  - Use g_device_response_buffer_pool for temporary operations
  - Purpose: Increase buffer pool utilization by using pools for temporary operations
  - _Leverage: g_device_response_buffer_pool, buffer_pool_acquire/release functions_
  - _Requirements: 1.2_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: XML Memory Developer with buffer pool expertise | Task: Replace malloc() calls in device service XML processing with buffer_pool_acquire() to utilize existing pool infrastructure | Restrictions: Must maintain XML functionality, use g_device_response_buffer_pool specifically, add buffer_pool_release() for cleanup | Success: Device service uses buffer pool for XML operations, utilization increases to >25%, temporary allocations eliminated | Instructions: Mark [-] when starting, find malloc calls in device handlers, replace with buffer_pool_acquire(), add corresponding release calls, test XML generation, mark [x] when pool usage implemented_

- [x] 7. Add buffer pool utilization monitoring
  - File: cross-compile/onvif/src/networking/common/buffer_pool.c
  - Add global counters for pool hits, misses, and current utilization percentage
  - Implement get_buffer_pool_stats() function to return utilization metrics
  - Purpose: Enable real-time monitoring of buffer pool efficiency
  - _Leverage: existing buffer pool structure, stdatomic APIs, platform logging_
  - _Requirements: 1.2, 6_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Metrics Collection Developer with thread-safe programming expertise | Task: Implement buffer pool statistics collection with atomic counters, real-time utilisation reporting, and >80% warning logging | Restrictions: Must use stdatomic operations for thread safety, avoid lock contention, log WARN when utilisation exceeds 80%, ensure stats available without blocking hot paths | Success: get_buffer_pool_stats() returns accurate utilisation with atomic counters, WARN logs trigger at 80%, metrics fetched without blocking HTTP processing | Instructions: Mark [-] when starting, replace raw counters with atomic types, update acquire/release paths, add threshold logging, mark [x] when monitoring functional_

- [x] 8. Implement size-based allocation strategy
  - File: cross-compile/onvif/src/services/device/onvif_device.c
  - Implement allocation strategy: use pool for buffers ≤32KB, direct allocation for larger
  - Modify device handlers to choose allocation method based on response size estimation
  - Purpose: Optimize allocation decisions to maximize pool utilization
  - _Leverage: existing response size estimation logic, buffer pool constants_
  - _Requirements: 1.2_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Memory Strategy Engineer with size optimization expertise | Task: Implement size-based allocation strategy where device handlers use buffer pool for responses ≤32KB and direct allocation for larger responses | Restrictions: Must estimate response size accurately, maintain fallback to direct allocation, preserve all functionality | Success: Allocation strategy implemented, buffer pool utilization reaches >50%, larger responses use direct allocation | Instructions: Mark [-] when starting, add response size estimation, implement allocation decision logic, test with various device operations, mark [x] when strategy operational_

- [x] 9. Locate HTTP server stack buffer allocations
  - File: cross-compile/onvif/src/networking/http/http_server.c
  - Search for stack buffer declarations using patterns: char buffer[32*1024], char buffer[BUFFER_SIZE]
  - Document exact line numbers and function names where 32KB stack buffers are allocated
  - Purpose: Create precise inventory of stack buffer locations for replacement
  - _Leverage: existing HTTP server code structure_
  - _Requirements: 1.1_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Code Analysis Engineer with C buffer analysis expertise | Task: Locate all 32KB stack buffer allocations in HTTP server by searching for specific buffer declaration patterns | Restrictions: Analysis only, no code modifications, document exact locations with line numbers | Success: Complete list of stack buffer locations with file paths and line numbers, function context documented | Instructions: Mark [-] when starting, grep for buffer declarations, check HTTP request handlers, document each finding with location details, mark [x] when all locations catalogued_

- [x] 10. Add buffer field to connection_t structure
  - File: cross-compile/onvif/src/networking/common/connection_manager.c
  - Add char *request_buffer field to connection_t structure
  - Allocate 32KB buffer during connection initialization in create_connection()
  - Purpose: Provide persistent buffer per connection instead of per-request stack allocation
  - _Leverage: existing connection_t structure, connection lifecycle functions_
  - _Requirements: 1.1_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Connection Structure Developer with C memory management expertise | Task: Add request_buffer field to connection_t and implement allocation/deallocation in connection lifecycle | Restrictions: Must use existing connection lifecycle functions, add proper cleanup, maintain thread safety | Success: connection_t has request_buffer field, buffer allocated in create_connection, freed in cleanup_connection | Instructions: Mark [-] when starting, modify connection_t in header, add allocation in create_connection, add free in cleanup_connection, mark [x] when structure updated_

- [x] 11. Replace stack buffers with connection buffers in HTTP handlers
  - File: cross-compile/onvif/src/networking/http/http_server.c
  - Replace stack buffer declarations with connection->request_buffer usage
  - Update all buffer references to use connection buffer instead of local stack buffer
  - Purpose: Eliminate stack buffer allocations by using persistent connection buffers
  - _Leverage: connection->request_buffer from updated connection_t structure_
  - _Requirements: 1.1_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: HTTP Handler Developer with buffer replacement expertise | Task: Replace stack buffer usage in HTTP handlers with connection->request_buffer to eliminate per-request allocations | Restrictions: Must maintain exact HTTP processing logic, update all buffer references consistently, ensure buffer bounds checking | Success: All stack buffer declarations removed, HTTP handlers use connection->request_buffer, HTTP functionality preserved | Instructions: Mark [-] when starting, replace buffer declarations with connection->request_buffer, update all buffer references, test HTTP request processing, mark [x] when stack buffers eliminated_

- [x] 12. Implement zero-copy HTTP parsing
  - File: cross-compile/onvif/src/networking/http/http_parser.c
  - Modify HTTP parsing functions to work directly with connection buffer without copying
  - Use pointer arithmetic and in-place parsing instead of copying to temporary buffers
  - Purpose: Eliminate memory copies during HTTP request parsing
  - _Leverage: existing HTTP parsing logic, connection buffer infrastructure_
  - _Requirements: 1.1_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Zero-Copy Parsing Developer with C pointer manipulation expertise | Task: Modify HTTP parsing to work directly with connection buffer using pointer arithmetic instead of memory copies | Restrictions: Must maintain parsing accuracy, preserve HTTP protocol compliance, use in-place parsing techniques | Success: HTTP parsing works directly on connection buffer, no temporary copies during parsing, protocol compliance maintained | Instructions: Mark [-] when starting, modify parsing functions to use pointers, eliminate strcpy/memcpy calls in parsing, test with various HTTP requests, mark [x] when zero-copy parsing implemented_

- [x] 13. Test HTTP server memory optimization
  - File: tests/networking/http_memory_tests.c
  - Create tests to verify stack buffer elimination and connection buffer usage
  - Test memory usage before and after optimization with realistic HTTP loads
  - Purpose: Validate HTTP server memory optimization achievements
  - _Leverage: existing test infrastructure, memory measurement tools_
  - _Requirements: 1.1_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: HTTP Memory Test Engineer with load testing expertise | Task: Create comprehensive tests to validate HTTP server memory optimization and measure improvement | Restrictions: Must test realistic HTTP loads, measure memory accurately, validate functionality preservation | Success: Tests confirm stack buffer elimination, memory usage reduction measured, HTTP functionality verified | Instructions: Mark [-] when starting, create memory measurement tests, run before/after comparisons, validate HTTP operations, mark [x] when optimization validated_

- [x] 14. Implement chunked streaming for large HTTP responses
  - File: cross-compile/onvif/src/networking/http/http_server.c
  - Add HTTP/1.1 chunked transfer support for responses >32KB to enable streaming without full-buffer allocation
  - Integrate chunked writer path with smart response builders and connection buffers
  - Purpose: Fulfill zero-copy streaming requirement for oversized responses
  - _Leverage: existing http_response_t structures, smart response builders, connection_t buffers_
  - _Requirements: 7_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: HTTP Streaming Engineer with chunked transfer expertise | Task: Implement chunked transfer encoding for large ONVIF responses using connection buffers and smart builders | Restrictions: Maintain HTTP compliance, preserve zero-copy behaviour, ensure fallbacks for non-chunked clients, follow coding standards | Success: Responses >32KB stream via chunked encoding without allocating full payload, compatibility validated with ONVIF clients | Instructions: Mark [-] when starting, design chunk writer integrated with connection buffer, add unit/integration coverage, mark [x] when streaming operational_

- [x] 15. Fix authentication bypass at line 805
  - File: cross-compile/onvif/src/networking/http/http_server.c
  - Replace if (0) authentication bypass with proper credential validation
  - Implement HTTP Basic Auth validation using existing security infrastructure
  - Purpose: Enable proper HTTP authentication instead of bypassing it
  - _Leverage: existing security_context_t structure, security_hardening.c_
  - _Requirements: 2_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: HTTP Security Engineer with authentication expertise | Task: Replace bypassed authentication at line 805 with proper credential validation using existing security infrastructure | Restrictions: Must use existing security_context_t, maintain HTTP performance, implement proper auth flow | Success: Authentication enabled, credentials validated, security bypass eliminated | Instructions: Mark [-] when starting, replace if(0) with auth validation, test authentication flow, mark [x] when auth enabled_

- [x] 16. Create HTTP authentication infrastructure
  - File: cross-compile/onvif/src/networking/http/http_auth.h
  - Create HTTP auth module based on existing RTSP auth patterns
  - Implement http_auth_config_t structure and basic management functions
  - Purpose: Establish foundation for HTTP Basic Authentication system
  - _Leverage: existing rtsp_auth.h patterns, rtsp_auth_config structure design_
  - _Requirements: 2.1_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: HTTP Authentication Infrastructure Engineer with C module design expertise | Task: Create HTTP authentication module files based on existing RTSP auth patterns, implementing http_auth_config_t structure and basic init/cleanup functions | Restrictions: Must follow existing RTSP auth patterns exactly, use same naming conventions, maintain compatibility with security_context_t | Success: HTTP auth module created, config structure implemented, init/cleanup functions working, module follows RTSP patterns | Instructions: Mark [-] when starting in tasks.md, analyze rtsp_auth patterns, create http_auth module, implement config structure, test module initialization, mark [x] when infrastructure complete_

- [x] 17. Implement Base64 credential decoding for Basic Auth
  - File: cross-compile/onvif/src/networking/http/http_auth.c
  - Add Base64 decoding function and Authorization header parsing
  - Implement credential extraction from "Authorization: Basic <encoded>" headers
  - Purpose: Enable parsing of HTTP Basic Auth credentials from request headers
  - _Leverage: existing string_shims.h utilities, safe string functions_
  - _Requirements: 2.1_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: HTTP Credential Processing Engineer with Base64 and header parsing expertise | Task: Implement Base64 decoding and Authorization header parsing for HTTP Basic Auth credentials | Restrictions: Must use existing string_shims utilities, add safe_base64_decode function, validate all inputs, prevent buffer overflows | Success: Base64 decoding implemented, Authorization header parsing working, credentials extracted safely, input validation complete | Instructions: Mark [-] when starting in tasks.md, implement safe Base64 decoder, add header parsing, test with various inputs, mark [x] when credential parsing complete_

- [x] 18. Implement HTTP Basic Auth validation logic
  - File: cross-compile/onvif/src/networking/http/http_auth.c
  - Add user credential validation and authentication flow
  - Integrate with existing security_context_t and user management
  - Purpose: Enable actual HTTP authentication with proper credential validation
  - _Leverage: existing RTSP user validation patterns, security_context_t integration_
  - _Requirements: 2.1_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: HTTP Authentication Validation Engineer with credential security expertise | Task: Implement HTTP Basic Auth validation logic using existing RTSP patterns and security infrastructure | Restrictions: Must follow RTSP auth validation patterns, integrate with security_context_t, maintain existing security flow, use safe string comparison | Success: Authentication validation implemented, proper auth flow active, security context integration working | Instructions: Mark [-] when starting in tasks.md, implement user lookup and validation, integrate with security context, test authentication flow, mark [x] when validation active_

- [x] 19. Implement WWW-Authenticate challenge responses
  - File: cross-compile/onvif/src/networking/http/http_auth.c
  - Add 401 Unauthorized response generation with WWW-Authenticate header
  - Implement ONVIF-compliant authentication challenges
  - Purpose: Provide proper HTTP auth challenges when credentials are missing or invalid
  - _Leverage: existing HTTP response generation, ONVIF specification requirements_
  - _Requirements: 2.1_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: HTTP Challenge Response Engineer with ONVIF compliance expertise | Task: Implement WWW-Authenticate challenge responses for HTTP Basic Auth following ONVIF standards | Restrictions: Must follow ONVIF authentication specification, use existing HTTP response infrastructure, generate proper Basic realm challenges | Success: 401 responses implemented, WWW-Authenticate headers correct, ONVIF compliance verified, challenge flow working | Instructions: Mark [-] when starting in tasks.md, implement 401 response generation, add WWW-Authenticate headers, test challenge flow, mark [x] when challenges complete_

- [x] 20. Integrate HTTP auth with security hardening system
  - File: cross-compile/onvif/src/networking/http/http_auth.c
  - Add rate limiting and attack detection for authentication attempts
  - Integrate with existing security_context_t and rate limiting infrastructure
  - Purpose: Prevent brute force attacks and integrate auth with security system
  - _Leverage: existing security_hardening.c rate limiting, attack detection functions_
  - _Requirements: 2.1_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Authentication Security Integration Engineer with rate limiting and attack detection expertise | Task: Integrate HTTP authentication with existing security hardening system for rate limiting and attack detection | Restrictions: Must use existing security_hardening functions only, integrate with security_context_t, maintain existing rate limiting patterns | Success: Rate limiting integrated, attack detection active, security context updated, brute force protection working | Instructions: Mark [-] when starting in tasks.md, integrate with security_hardening, add rate limiting, test attack scenarios, mark [x] when security integration complete_

- [x] 21. Implement comprehensive input validation for auth data
  - File: cross-compile/onvif/src/networking/http/http_auth.c
  - Add input validation for usernames, passwords, and header values
  - Implement sanitization and bounds checking for all auth-related inputs
  - Purpose: Prevent injection attacks and malformed credential handling
  - _Leverage: existing input_validation.c utilities, validation patterns_
  - _Requirements: 2.1_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Authentication Input Validation Specialist with injection prevention expertise | Task: Implement comprehensive input validation for all HTTP authentication data using existing validation utilities | Restrictions: Must use existing input_validation utilities only, validate usernames/passwords/headers, prevent injection attacks, maintain auth performance | Success: All auth inputs validated, injection prevention implemented, validation integrated, malformed input handled | Instructions: Mark [-] when starting in tasks.md, add validation to all auth inputs, test with malicious data, mark [x] when validation complete_

- [x] 22. Create comprehensive HTTP authentication tests
  - File: tests/networking/http_auth_tests.c
  - Implement complete test coverage for HTTP authentication system
  - Test valid/invalid credentials, attack scenarios, and ONVIF compliance
  - Purpose: Validate HTTP authentication implementation and security
  - _Leverage: existing test infrastructure, ONVIF test patterns_
  - _Requirements: 2.1_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: HTTP Authentication Test Engineer with security testing expertise | Task: Create comprehensive test suite for HTTP authentication system covering functionality and security scenarios | Restrictions: Must test all auth scenarios, validate ONVIF compliance, test attack prevention, use existing test infrastructure | Success: Complete auth test suite implemented, all scenarios tested, security validated, ONVIF compliance verified | Instructions: Mark [-] when starting in tasks.md, implement auth tests, test security scenarios, validate ONVIF compliance, mark [x] when testing complete_

- [x] 23. Integrate validation utilities and safe string operations in HTTP parsing
  - File: cross-compile/onvif/src/networking/http/http_server.c, cross-compile/onvif/src/networking/http/http_parser.c
  - Add comprehensive HTTP request validation using common_validation.h and replace raw string copies with string_shims safe APIs
  - Implement validation for headers, query parameters, and request body while removing strncpy/strcpy usage in HTTP modules
  - Purpose: Use existing validation utilities and safe string helpers to satisfy security requirements
  - _Leverage: common_validation.h utilities, string_shims.h safe_* functions, existing validation patterns_
  - _Requirements: 2.1_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: HTTP Validation Developer with input sanitization expertise | Task: Integrate common_validation utilities across HTTP parsing and migrate all HTTP string handling to safe_* helpers | Restrictions: Must use existing utilities only, maintain HTTP performance, ensure no direct strncpy/strcpy remain in HTTP modules | Success: HTTP requests validated using shared utilities, all string copies performed via safe_* helpers, security coverage confirmed by tests | Instructions: Mark [-] when starting, wire validation into parser/server, replace unsafe string usage, extend tests, mark [x] when validation integrated_

- [x] 24. Integrate service_logging.h for structured HTTP logging
  - File: cross-compile/onvif/src/networking/http/http_server.c
  - Add structured logging for HTTP requests and responses
  - Implement consistent logging format across HTTP operations
  - Purpose: Use existing logging utilities for HTTP layer logging
  - _Leverage: service_logging.h utilities, existing logging patterns_
  - _Requirements: 2.1_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: HTTP Logging Developer with structured logging expertise | Task: Integrate service_logging.h utilities for structured HTTP logging | Restrictions: Must use existing utilities only, maintain logging performance, follow existing patterns | Success: Structured HTTP logging implemented, consistent format across operations | Instructions: Mark [-] when starting, integrate logging utilities, test logging output, mark [x] when logging integrated_

- [-] 25. Implement HTTP error handling with utilities
  - File: cross-compile/onvif/src/networking/http/http_server.c
  - Use existing error handling utilities for HTTP error responses
  - Implement consistent error reporting across HTTP operations
  - Purpose: Standardize HTTP error handling using existing utilities
  - _Leverage: error_handling.c utilities, existing error patterns_
  - _Requirements: 2.1_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task: Role: HTTP Error Handling Developer with error response expertise | Task: Implement consistent HTTP error handling using existing error utilities | Restrictions: Must use existing utilities only, maintain error response accuracy, follow existing patterns | Success: Consistent error handling implemented, error utilities integrated | Instructions: Mark [-] when starting, integrate error utilities, test error responses, mark [x] when error handling complete_

- [x] 26. Define standardized ONVIF service callback interface
  - File: cross-compile/onvif/src/services/common/service_dispatcher.h, cross-compile/onvif/src/services/common/service_dispatcher.c
  - Introduce shared callback typedefs and registration helpers reflecting device service pattern
  - Update service dispatcher documentation to describe required handler signature and lifecycle
  - Purpose: Provide reusable callback interface so every service adopts the device service pattern
  - _Leverage: device service registration code, existing service dispatcher infrastructure_
  - _Requirements: 3, 9_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: ONVIF Service Architect with dispatcher expertise | Task: Define shared callback typedefs and helpers in service_dispatcher to codify the device service handler pattern | Restrictions: Maintain backward compatibility for existing device service usage, document new interfaces with Doxygen, avoid introducing unused abstractions | Success: Shared callback interface published, dispatcher enforces signature, documentation updated | Instructions: Mark [-] when starting, add typedefs/helpers, update docs, run unit tests, mark [x] when interface defined_

- [ ] 27. Align device service registration with standardized callback interface
  - File: cross-compile/onvif/src/services/device/onvif_device.c
  - Refactor device service registration to use new callback typedefs and helpers
  - Remove legacy registration code paths that bypass standardized dispatcher APIs
  - Purpose: Ensure device service remains canonical implementation of the callback pattern
  - _Leverage: standardized dispatcher interface, existing device service handlers_
  - _Requirements: 3, 9_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Device Service Maintainer with callback refactor expertise | Task: Update device service registration to consume the new shared callback helpers and eliminate bespoke code paths | Restrictions: Preserve all device service functionality, keep memory optimizations intact, follow AGENTS.md coding standards | Success: Device service registers exclusively through shared helpers, code compiles cleanly, unit/integration tests pass | Instructions: Mark [-] when starting, refactor registration, run relevant tests, mark [x] when alignment complete_

- [ ] 28. Add device service callback unit tests
  - File: cross-compile/onvif/tests/unit/services/device/test_onvif_device_callbacks.c
  - Create CMocka tests covering device service callback registration and dispatch flow
  - Validate error handling for null callbacks and ensure dispatcher forwards requests correctly
  - Purpose: Provide regression coverage for standardized callback pattern on device service
  - _Leverage: CMocka test framework, existing device service mocks_
  - _Requirements: 3, 4_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Device Service Test Engineer with CMocka expertise | Task: Build unit tests asserting device callbacks register through shared helpers and handle error scenarios | Restrictions: Use existing mocking utilities, keep tests deterministic, ensure memory tracking macros used | Success: New tests compile and pass under make test, coverage includes success and failure paths | Instructions: Mark [-] when starting, add tests, run make test, mark [x] when tests pass_

- [ ] 29. Refactor media service to adopt standardized callback pattern
  - File: cross-compile/onvif/src/services/media/onvif_media.c
  - Update media service registration and handlers to use shared callback typedefs and helpers
  - Remove custom dispatch code and align handler signatures with device service pattern
  - Purpose: Ensure media service follows the unified callback approach
  - _Leverage: standardized dispatcher interface, device service reference implementation_
  - _Requirements: 3, 9_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Media Service Refactoring Engineer with callback expertise | Task: Replace media service registration/dispatch logic with the standardized helper-driven approach mirroring device service | Restrictions: Maintain media functionality, reuse shared helpers, update Doxygen comments | Success: Media service builds with new callbacks, no bespoke dispatcher logic remains, unit/integration tests pass | Instructions: Mark [-] when starting, refactor callbacks, run media tests, mark [x] when refactor complete_

- [ ] 30. Add media service callback unit tests
  - File: cross-compile/onvif/tests/unit/services/media/test_onvif_media_callbacks.c
  - Build CMocka tests verifying media service registration with shared helpers and dispatch routing
  - Cover error cases (null handlers, duplicate registration) and success path
  - Purpose: Deliver unit-level assurance that media service follows standardized callback pattern
  - _Leverage: CMocka framework, new standardized dispatcher helpers_
  - _Requirements: 3, 4_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Media Service Test Engineer with dispatcher validation expertise | Task: Add unit tests confirming media callbacks register via shared helpers and behave correctly | Restrictions: Use shared mock utilities, ensure tests enforce error handling, follow test naming conventions | Success: Tests compile, run via make test, and validate both success and failure flows | Instructions: Mark [-] when starting, implement tests, execute make test, mark [x] when passing_

- [ ] 31. Refactor PTZ service to adopt standardized callback pattern
  - File: cross-compile/onvif/src/services/ptz/onvif_ptz.c
  - Update PTZ service registration and handlers to use shared callback typedefs and helpers
  - Remove bespoke dispatcher glue in PTZ module and align with device service flow
  - Purpose: Extend unified callback approach to PTZ service
  - _Leverage: standardized dispatcher interface, device/media callback refactors_
  - _Requirements: 3, 9_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: PTZ Service Refactoring Engineer with callback expertise | Task: Refactor PTZ service to register handlers through shared helpers and align signatures | Restrictions: Maintain PTZ command behavior, ensure thread safety remains, update documentation | Success: PTZ service builds using standardized callbacks, bespoke logic removed, tests pass | Instructions: Mark [-] when starting, refactor PTZ callbacks, run PTZ tests, mark [x] when complete_

- [ ] 32. Add PTZ service callback unit tests
  - File: cross-compile/onvif/tests/unit/services/ptz/test_onvif_ptz_callbacks.c
  - Implement CMocka tests covering PTZ service registration, dispatch, and error handling with shared helpers
  - Validate callback invocation order and ensure logging occurs for failure paths
  - Purpose: Guarantee PTZ service adherence to standardized callback pattern
  - _Leverage: CMocka framework, standardized dispatcher mocks_
  - _Requirements: 3, 4_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: PTZ Service Test Engineer with dispatcher validation expertise | Task: Add unit tests verifying PTZ service uses shared callbacks and handles errors correctly | Restrictions: Use consistent mocking strategy, assert logging outputs where applicable, keep tests isolated | Success: PTZ callback tests pass under make test, covering success and failure scenarios | Instructions: Mark [-] when starting, author tests, execute make test, mark [x] when passing_

- [ ] 33. Refactor imaging service to adopt standardized callback pattern
  - File: cross-compile/onvif/src/services/imaging/onvif_imaging.c
  - Update imaging service registration and handlers to use shared callback typedefs and helpers
  - Remove imaging-specific dispatch glue and synchronize signatures with device service pattern
  - Purpose: Apply unified callback approach across imaging service
  - _Leverage: standardized dispatcher interface, prior service refactors_
  - _Requirements: 3, 9_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Imaging Service Refactoring Engineer with callback expertise | Task: Refactor imaging service to register through shared helpers and align handler signatures | Restrictions: Maintain imaging parameter functionality, ensure memory optimizations unaffected, update Doxygen documentation | Success: Imaging service builds with standardized callbacks, bespoke logic removed, tests pass | Instructions: Mark [-] when starting, refactor imaging callbacks, run imaging tests, mark [x] when complete_

- [ ] 34. Add imaging service callback unit tests
  - File: cross-compile/onvif/tests/unit/services/imaging/test_onvif_imaging_callbacks.c
  - Develop CMocka tests ensuring imaging service registers through shared helpers and dispatches correctly
  - Include tests for invalid inputs, null handlers, and normal execution paths
  - Purpose: Provide unit-level validation that imaging service follows standardized callback pattern
  - _Leverage: CMocka framework, shared dispatcher mocks_
  - _Requirements: 3, 4_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Imaging Service Test Engineer with dispatcher validation expertise | Task: Create unit tests verifying imaging callbacks follow shared helpers and handle errors | Restrictions: Keep tests deterministic, reuse shared test utilities, follow naming conventions | Success: Imaging callback tests pass under make test, exercising success and failure flows | Instructions: Mark [-] when starting, implement tests, run make test, mark [x] when tests pass_

- [ ] 35. Search for large allocations in media service handlers
  - File: cross-compile/onvif/src/services/media/onvif_media.c
  - Search for ONVIF_RESPONSE_BUFFER_SIZE and malloc() calls in media handlers
  - Document each allocation with function name, line number, and size
  - Purpose: Identify specific allocation targets for optimization in media service
  - _Leverage: grep patterns, existing media service handler structure_
  - _Requirements: 9_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Media Memory Analysis Engineer with C allocation tracking expertise | Task: Search media service for large allocations using grep patterns for ONVIF_RESPONSE_BUFFER_SIZE and malloc | Restrictions: Analysis only, document exact locations with line numbers and allocation sizes | Success: Complete list of large allocations in media service with precise locations documented | Instructions: Mark [-] when starting, grep for allocation patterns, document each finding with location, mark [x] when all allocations catalogued_

- [ ] 36. Implement smart response builders for media service
  - File: cross-compile/onvif/src/services/media/onvif_media.c
  - Replace large allocations with smart response builder patterns
  - Implement dynamic buffer allocation for media responses
  - Purpose: Apply device service optimization patterns to media service
  - _Leverage: smart response builders, device service patterns_
  - _Requirements: 9_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Media Service Memory Developer with response builder expertise | Task: Implement smart response builders for media service following device service patterns | Restrictions: Must maintain media functionality, follow established patterns exactly, preserve ONVIF compliance | Success: Smart response builders implemented, large allocations replaced | Instructions: Mark [-] when starting, implement response builders, test media operations, mark [x] when builders implemented_

- [ ] 37. Optimize media profile management
  - File: cross-compile/onvif/src/services/media/onvif_media.c
  - Optimize media profile creation and management operations
  - Implement efficient profile storage and retrieval patterns
  - Purpose: Optimize media profile operations for memory efficiency
  - _Leverage: existing profile management, smart response builders_
  - _Requirements: 9_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Media Profile Developer with profile management expertise | Task: Optimize media profile management operations for memory efficiency | Restrictions: Must maintain ONVIF compliance, preserve profile functionality, use existing patterns | Success: Profile management optimized, memory usage reduced | Instructions: Mark [-] when starting, optimize profile operations, test profile functionality, mark [x] when profiles optimized_

- [ ] 38. Optimize media stream URI generation
  - File: cross-compile/onvif/src/services/media/onvif_media.c
  - Optimize stream URI generation and response building
  - Implement efficient URI construction patterns
  - Purpose: Optimize stream URI operations for memory efficiency
  - _Leverage: existing URI generation, smart response builders_
  - _Requirements: 9_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Media Stream Developer with URI generation expertise | Task: Optimize media stream URI generation for memory efficiency | Restrictions: Must maintain stream functionality, preserve URI accuracy, use existing patterns | Success: Stream URI generation optimized, memory usage reduced | Instructions: Mark [-] when starting, optimize URI generation, test stream operations, mark [x] when streams optimized_

- [ ] 39. Test media service optimization
  - File: tests/integration/media_service_tests.c
  - Create comprehensive tests for optimized media service
  - Verify memory usage improvements and functionality preservation
  - Purpose: Validate media service optimization results
  - _Leverage: existing test infrastructure, media service patterns_
  - _Requirements: 9_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Media Service Test Engineer with optimization validation expertise | Task: Create comprehensive tests for optimized media service to verify improvements | Restrictions: Must test all media operations, validate memory improvements, ensure functionality preservation | Success: All media tests pass, memory improvements validated, functionality preserved | Instructions: Mark [-] when starting, create media tests, run comprehensive testing, mark [x] when testing complete_

- [ ] 40. Analyze PTZ service memory allocation patterns
  - File: cross-compile/onvif/src/services/ptz/onvif_ptz.c
  - Identify all memory allocations in PTZ service handlers
  - Document PTZ-specific allocation patterns and optimization opportunities
  - Purpose: Understand PTZ service memory usage before optimization
  - _Leverage: existing PTZ service code, device service patterns_
  - _Requirements: 9_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: PTZ Service Analysis Specialist with memory pattern expertise | Task: Analyze PTZ service code to identify allocations and optimization opportunities | Restrictions: Analysis only, no code changes, document all patterns | Success: Complete inventory of PTZ allocations, optimization targets identified | Instructions: Mark [-] when starting, analyze PTZ service code, document findings, mark [x] when analysis complete_

- [ ] 41. Implement smart response builders for PTZ service
  - File: cross-compile/onvif/src/services/ptz/onvif_ptz.c
  - Replace allocations with smart response builder patterns for PTZ operations
  - Implement dynamic buffer allocation for PTZ responses
  - Purpose: Apply device service optimization patterns to PTZ service
  - _Leverage: smart response builders, device service patterns_
  - _Requirements: 9_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: PTZ Memory Developer with response builder expertise | Task: Implement smart response builders for PTZ service following device service patterns | Restrictions: Must maintain PTZ functionality, follow established patterns exactly, preserve ONVIF compliance | Success: Smart response builders implemented, allocations optimized | Instructions: Mark [-] when starting, implement response builders, test PTZ operations, mark [x] when builders implemented_

- [ ] 42. Optimize PTZ movement operations
  - File: cross-compile/onvif/src/services/ptz/onvif_ptz.c
  - Optimize continuous move, absolute move, and relative move operations
  - Implement efficient PTZ command processing patterns
  - Purpose: Optimize PTZ movement operations for memory efficiency
  - _Leverage: existing PTZ movement code, smart response builders_
  - _Requirements: 9_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: PTZ Movement Developer with command optimization expertise | Task: Optimize PTZ movement operations for memory efficiency | Restrictions: Must maintain movement accuracy, preserve PTZ functionality, use existing patterns | Success: Movement operations optimized, memory usage reduced | Instructions: Mark [-] when starting, optimize movement operations, test PTZ movements, mark [x] when movements optimized_

- [ ] 43. Optimize PTZ preset management
  - File: cross-compile/onvif/src/services/ptz/onvif_ptz.c
  - Optimize preset creation, recall, and management operations
  - Implement efficient preset storage and retrieval patterns
  - Purpose: Optimize PTZ preset operations for memory efficiency
  - _Leverage: existing preset management, smart response builders_
  - _Requirements: 9_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: PTZ Preset Developer with preset optimization expertise | Task: Optimize PTZ preset management operations for memory efficiency | Restrictions: Must maintain preset functionality, preserve preset accuracy, use existing patterns | Success: Preset operations optimized, memory usage reduced | Instructions: Mark [-] when starting, optimize preset operations, test preset functionality, mark [x] when presets optimized_

- [ ] 44. Test PTZ service optimization
  - File: tests/integration/ptz_service_tests.c
  - Create comprehensive tests for optimized PTZ service
  - Verify memory usage improvements and PTZ functionality preservation
  - Purpose: Validate PTZ service optimization results
  - _Leverage: existing test infrastructure, PTZ service patterns_
  - _Requirements: 9_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: PTZ Test Engineer with optimization validation expertise | Task: Create comprehensive tests for optimized PTZ service to verify improvements | Restrictions: Must test all PTZ operations, validate memory improvements, ensure functionality preservation | Success: All PTZ tests pass, memory improvements validated, PTZ functionality preserved | Instructions: Mark [-] when starting, create PTZ tests, run comprehensive testing, mark [x] when testing complete_

- [ ] 45. Analyze imaging service memory allocation patterns
  - File: cross-compile/onvif/src/services/imaging/onvif_imaging.c
  - Identify all memory allocations in imaging service handlers
  - Document imaging-specific allocation patterns and optimization opportunities
  - Purpose: Understand imaging service memory usage before optimization
  - _Leverage: existing imaging service code, established service patterns_
  - _Requirements: 9_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Imaging Service Analysis Specialist with memory pattern expertise | Task: Analyze imaging service code to identify allocations and optimization opportunities | Restrictions: Analysis only, no code changes, document all patterns | Success: Complete inventory of imaging allocations, optimization targets identified | Instructions: Mark [-] when starting, analyze imaging service code, document findings, mark [x] when analysis complete_

- [ ] 46. Implement smart response builders for imaging service
  - File: cross-compile/onvif/src/services/imaging/onvif_imaging.c
  - Replace allocations with smart response builder patterns for imaging operations
  - Implement dynamic buffer allocation for imaging responses
  - Purpose: Apply established optimization patterns to imaging service
  - _Leverage: smart response builders, established service patterns_
  - _Requirements: 9_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Imaging Memory Developer with response builder expertise | Task: Implement smart response builders for imaging service following established patterns | Restrictions: Must maintain imaging functionality, follow established patterns exactly, preserve ONVIF compliance | Success: Smart response builders implemented, allocations optimized | Instructions: Mark [-] when starting, implement response builders, test imaging operations, mark [x] when builders implemented_

- [ ] 47. Optimize imaging parameter management
  - File: cross-compile/onvif/src/services/imaging/onvif_imaging.c
  - Optimize brightness, contrast, saturation, and other imaging parameter operations
  - Implement efficient parameter storage and retrieval patterns
  - Purpose: Optimize imaging parameter operations for memory efficiency
  - _Leverage: existing parameter management, smart response builders_
  - _Requirements: 9_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Imaging Parameter Developer with parameter optimization expertise | Task: Optimize imaging parameter management operations for memory efficiency | Restrictions: Must maintain parameter functionality, preserve image quality settings, use existing patterns | Success: Parameter operations optimized, memory usage reduced | Instructions: Mark [-] when starting, optimize parameter operations, test imaging parameters, mark [x] when parameters optimized_

- [ ] 48. Optimize imaging settings operations
  - File: cross-compile/onvif/src/services/imaging/onvif_imaging.c
  - Optimize imaging settings get/set operations
  - Implement efficient settings processing patterns
  - Purpose: Optimize imaging settings operations for memory efficiency
  - _Leverage: existing settings management, smart response builders_
  - _Requirements: 9_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Imaging Settings Developer with settings optimization expertise | Task: Optimize imaging settings operations for memory efficiency | Restrictions: Must maintain settings functionality, preserve settings accuracy, use existing patterns | Success: Settings operations optimized, memory usage reduced | Instructions: Mark [-] when starting, optimize settings operations, test imaging settings, mark [x] when settings optimized_

- [ ] 49. Test imaging service optimization
  - File: tests/integration/imaging_service_tests.c
  - Create comprehensive tests for optimized imaging service
  - Verify memory usage improvements and imaging functionality preservation
  - Purpose: Validate imaging service optimization results
  - _Leverage: existing test infrastructure, imaging service patterns_
  - _Requirements: 9_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Imaging Test Engineer with optimization validation expertise | Task: Create comprehensive tests for optimized imaging service to verify improvements | Restrictions: Must test all imaging operations, validate memory improvements, ensure functionality preservation | Success: All imaging tests pass, memory improvements validated, imaging functionality preserved | Instructions: Mark [-] when starting, create imaging tests, run comprehensive testing, mark [x] when testing complete_

- [ ] 50. Create baseline memory measurement tests
  - File: tests/integration/onvif_memory_tests.c
  - Implement memory measurement using /proc/self/status VmRSS before optimization
  - Run device, media, PTZ, and imaging operations to measure baseline memory
  - Purpose: Establish baseline memory usage for comparison after optimization
  - _Leverage: /proc filesystem memory measurement, existing ONVIF service operations_
  - _Requirements: 10_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Memory Measurement Engineer with Linux /proc expertise | Task: Create baseline memory measurement tests using /proc/self/status VmRSS to measure memory usage before optimization | Restrictions: Must measure actual RSS memory, run realistic ONVIF operations, document baseline accurately | Success: Baseline memory measurements for all services documented, measurement infrastructure working | Instructions: Mark [-] when starting, implement /proc/self/status reading, run ONVIF operations, document baseline memory, mark [x] when baseline established_

- [ ] 51. Create security functionality integration tests
  - File: tests/integration/onvif_security_tests.c
  - Implement comprehensive security testing for HTTP layer and authentication
  - Test buffer overflow prevention, input validation, and authentication
  - Purpose: Validate security improvements and vulnerability fixes
  - _Leverage: existing test infrastructure, security testing patterns_
  - _Requirements: 10_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Security Testing Engineer with vulnerability validation expertise | Task: Create comprehensive security tests to validate vulnerability fixes and security functionality | Restrictions: Must test all security aspects, validate authentication, test input validation | Success: Security tests validate vulnerability fixes, authentication working, input validation effective | Instructions: Mark [-] when starting, implement security tests, validate fixes, mark [x] when security testing complete_

- [ ] 52. Create service functionality integration tests
  - File: tests/integration/onvif_service_tests.c
  - Implement comprehensive ONVIF service functionality testing
  - Test device, media, PTZ, and imaging service operations
  - Purpose: Validate that all ONVIF functionality is preserved after optimization
  - _Leverage: existing ONVIF test patterns, service testing infrastructure_
  - _Requirements: 10_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: ONVIF Service Testing Engineer with compliance validation expertise | Task: Create comprehensive service functionality tests to validate all ONVIF operations work correctly | Restrictions: Must test all ONVIF operations, validate service compliance, ensure no regressions | Success: All ONVIF services tested and functional, compliance validated, no regressions found | Instructions: Mark [-] when starting, implement service tests, validate functionality, mark [x] when service testing complete_

- [ ] 53. Create performance regression tests
  - File: tests/integration/onvif_performance_tests.c
  - Implement performance testing to ensure optimizations don't degrade performance
  - Test response times, throughput, and resource usage
  - Purpose: Validate that optimizations maintain or improve performance
  - _Leverage: existing performance testing tools, benchmarking infrastructure_
  - _Requirements: 10_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Performance Testing Engineer with benchmark validation expertise | Task: Create performance regression tests to ensure optimizations maintain or improve performance | Restrictions: Must test performance metrics, validate no degradation, measure improvements | Success: Performance tests validate maintained or improved performance, no regressions found | Instructions: Mark [-] when starting, implement performance tests, measure metrics, mark [x] when performance testing complete_

- [ ] 54. Create end-to-end integration test suite
  - File: tests/integration/onvif_e2e_tests.c
  - Implement complete end-to-end ONVIF client-server testing
  - Test full ONVIF workflows with real client interactions
  - Purpose: Validate complete ONVIF functionality in realistic scenarios
  - _Leverage: existing ONVIF client libraries, test automation infrastructure_
  - _Requirements: 10_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: E2E Testing Engineer with client integration expertise | Task: Create end-to-end integration tests with real ONVIF client interactions | Restrictions: Must test complete workflows, validate real client compatibility, ensure full functionality | Success: E2E tests validate complete functionality, real client compatibility confirmed | Instructions: Mark [-] when starting, implement E2E tests, validate with real clients, mark [x] when E2E testing complete_

- [ ] 55. Collect baseline and optimized performance metrics
  - File: docs/refactoring/performance_report.md
  - Measure memory usage, response times, and resource consumption
  - Document performance metrics for baseline and optimized implementations
  - Purpose: Gather objective data for performance comparison
  - _Leverage: performance measurement tools, baseline audit data_
  - _Requirements: 4, 10_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Performance Measurement Specialist with metrics collection expertise | Task: Collect comprehensive performance metrics for baseline and optimized implementations | Restrictions: Must use accurate measurement tools, document all metrics | Success: Complete performance data collected for both implementations | Instructions: Mark [-] when starting, run performance measurements, document metrics, mark [x] when metrics collected_

- [ ] 56. Analyze memory optimization achievements
  - File: docs/refactoring/performance_report.md
  - Calculate memory reduction percentages and buffer pool utilization
  - Document specific optimization achievements and improvements
  - Purpose: Quantify memory optimization results
  - _Leverage: collected performance data, memory measurement tools_
  - _Requirements: 1.1_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Memory Analysis Engineer with optimization quantification expertise | Task: Analyze memory optimization achievements and document specific improvements | Restrictions: Must provide accurate calculations, document all improvements | Success: Memory optimization achievements quantified and documented | Instructions: Mark [-] when starting, analyze memory data, calculate improvements, mark [x] when analysis complete_

- [ ] 57. Document security improvements and vulnerability fixes
  - File: docs/refactoring/performance_report.md
  - Document all security vulnerabilities that were fixed
  - Document security improvements and authentication enhancements
  - Purpose: Document security achievements and improvements
  - _Leverage: security test results, vulnerability analysis data_
  - _Requirements: 2.1_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Security Documentation Specialist with vulnerability reporting expertise | Task: Document all security improvements and vulnerability fixes in the performance report | Restrictions: Must document all security aspects, provide clear explanations | Success: Security improvements comprehensively documented | Instructions: Mark [-] when starting, document security fixes, explain improvements, mark [x] when security documented_

- [ ] 58. Create comprehensive performance comparison report
  - File: docs/refactoring/performance_report.md
  - Create detailed comparison between baseline and optimized implementations
  - Document all achievements against requirements and objectives
  - Purpose: Create final validation report demonstrating all achievements
  - _Leverage: all collected data, performance metrics, security analysis_
  - _Requirements: 4, 10_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Technical Report Writer with comprehensive analysis expertise | Task: Create comprehensive performance comparison report documenting all achievements | Restrictions: Must be comprehensive, accurate, and well-structured | Success: Complete performance report demonstrates all achievements and improvements | Instructions: Mark [-] when starting, create comprehensive report, validate all data, mark [x] when report complete_

- [ ] 59. Validate achievement of all requirements
  - File: docs/refactoring/performance_report.md
  - Verify that all requirements have been met and documented
  - Create final validation summary with achievement confirmation
  - Purpose: Confirm that all refactoring objectives have been achieved
  - _Leverage: requirements document, performance data, test results_
  - _Requirements: 4, 10_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Requirements Validation Engineer with compliance verification expertise | Task: Validate that all requirements have been met and create final achievement summary | Restrictions: Must verify all requirements, provide clear validation | Success: All requirements validated and documented, final summary complete | Instructions: Mark [-] when starting, validate all requirements, create summary, mark [x] when validation complete_

- [ ] 60. Implement real-time HTTP performance metrics
  - File: cross-compile/onvif/src/networking/http/http_server.c
  - Capture per-request latency, response size, and error rate metrics without exceeding 5% CPU overhead
  - Record metrics in thread-safe rolling windows and expose them via monitoring API
  - Purpose: Fulfill Requirement 6 real-time monitoring for HTTP transport layer
  - _Leverage: connection_t timestamps, platform_get_time_ms, existing logging infrastructure_
  - _Requirements: 6_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Embedded Monitoring Engineer with low-overhead telemetry expertise | Task: Implement real-time HTTP performance metrics (latency, throughput, error rates, memory usage deltas) with <5% CPU overhead and expose retrieval API | Restrictions: Must avoid blocking request processing, use atomic counters for shared data, integrate with existing logging conventions | Success: Metrics collected per acceptance criteria, retrieval API returns current statistics, verified overhead below threshold | Instructions: Mark [-] when starting, instrument request lifecycle, add metrics aggregation, expose retrieval function, validate overhead, mark [x] when monitoring complete_

- [ ] 61. Add HTTP performance metrics tests
  - File: tests/networking/http_metrics_tests.c
  - Create unit and integration tests validating metric accuracy and performance thresholds
  - Simulate concurrent requests to ensure metrics remain thread-safe and non-blocking
  - Purpose: Verify new monitoring capabilities meet Requirement 6 criteria
  - _Leverage: existing test harness, mock connection lifecycle helpers_
  - _Requirements: 6_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Telemetry Test Engineer with concurrency testing expertise | Task: Build automated tests that validate HTTP metrics correctness under concurrent load and confirm <5% CPU overhead | Restrictions: Must simulate concurrent requests, use existing test infrastructure, capture before/after metrics | Success: Tests cover latency, error rate, utilisation accuracy, overhead assertions pass | Instructions: Mark [-] when starting, implement metrics tests, run under load scenarios, mark [x] when tests validate monitoring_

- [ ] 62. Harden connection lifecycle concurrency
  - File: cross-compile/onvif/src/networking/common/connection_manager.c
  - Introduce atomic counters and deterministic cleanup sequencing for connection state transitions
  - Ensure buffer pool returns and socket closures are race-free under concurrent access
  - Purpose: Satisfy Requirement 8 thread-safe connection management
  - _Leverage: stdatomic APIs, existing mutex infrastructure, platform logging_
  - _Requirements: 8_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Concurrency Engineer with embedded networking expertise | Task: Harden connection lifecycle by adding atomic state tracking, deterministic cleanup ordering, and race-safe buffer returns | Restrictions: Maintain existing mutex usage, ensure no double-free or leaks, follow logging standards | Success: Connection teardown is deterministic under concurrent load, race detection tools show no data races, buffer pool interactions remain safe | Instructions: Mark [-] when starting, add atomic state tracking, restructure cleanup, run concurrency validation, mark [x] when hardened_

- [ ] 63. Validate connection concurrency with stress tests
  - File: tests/networking/connection_concurrency_tests.c
  - Implement stress tests simulating rapid connect/disconnect cycles with concurrent threads
  - Verify absence of race conditions, deadlocks, and resource leaks using sanitizers
  - Purpose: Provide automated coverage for Requirement 8 concurrency guarantees
  - _Leverage: pthread-based test harness, existing connection manager APIs_
  - _Requirements: 8_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Concurrency Test Engineer with stress testing expertise | Task: Create stress tests for connection manager ensuring deterministic cleanup, atomic counters, and buffer safety hold under concurrent load | Restrictions: Use existing test infrastructure, include sanitizer instrumentation, collect logs for debugging | Success: Tests pass under high concurrency with sanitizers clean, logs show deterministic cleanup, no leaks detected | Instructions: Mark [-] when starting, implement stress scenarios, execute with sanitizers, mark [x] when concurrency validated_

- [ ] 64. Secure HTTP header response building
  - File: cross-compile/onvif/src/networking/http/http_parser.c
  - Replace strcpy usage in http_response_add_header with safe string utilities and size bounds
  - Add unit tests covering header copy edge cases and malicious inputs
  - Purpose: Close remaining Requirement 2.1 gaps in HTTP response handling
  - _Leverage: string_shims.h safe_strncpy, existing HTTP response structures_
  - _Requirements: 2.1_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: HTTP Security Engineer with memory safety expertise | Task: Refactor http_response_add_header to use safe string helpers and add tests covering overflow and truncation scenarios | Restrictions: Must use string_shims safe APIs, maintain allocation strategy, ensure tests cover boundary cases | Success: No raw strcpy usage remains, tests verify safe handling, static analysis clean | Instructions: Mark [-] when starting, refactor function, add tests, run security-focused test suite, mark [x] when secure_

- [ ] 65. Delegate system utilization requests
  - File: cross-compile/onvif/src/networking/http/http_server.c
  - Route `/system/utilization` and related endpoints to dedicated system service handlers
  - Remove business logic from HTTP transport layer in favor of service modules
  - Purpose: Satisfy Requirement 5 separation-of-concerns for system utilization
  - _Leverage: services/system/onvif_system.h, existing service dispatch patterns_
  - _Requirements: 5_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: HTTP Architecture Engineer with service delegation expertise | Task: Refactor HTTP server to delegate system utilization requests to system service module, eliminating transport-layer business logic | Restrictions: Preserve existing endpoint behavior, update routing tables only, ensure unit tests cover delegation | Success: HTTP server performs delegation only, system service handles logic, tests confirm routing | Instructions: Mark [-] when starting, refactor routing paths, add delegation tests, mark [x] when delegation complete_

- [ ] 66. Separate HTTP request and response buffers
  - File: cross-compile/onvif/src/networking/http/http_server.c, cross-compile/onvif/src/networking/common/connection_manager.c
  - Introduce distinct buffer structures for request parsing and response generation per connection
  - Ensure buffer reuse patterns avoid memory conflicts and align with zero-copy goals
  - Purpose: Meet Requirement 5 buffer separation criterion
  - _Leverage: connection_t lifecycle, buffer pool utilities, smart response builders_
  - _Requirements: 5_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Embedded Memory Engineer with networking expertise | Task: Provide separate request/response buffers per connection using buffer pools or dynamic allocation while preserving zero-copy parsing | Restrictions: Maintain existing buffer pool usage, avoid regressions in parsing/response flow, update cleanup paths | Success: Distinct buffers verified in connection manager, tests confirm no conflicts, memory usage tracked | Instructions: Mark [-] when starting, update connection structures, adjust parsing/response paths, extend tests, mark [x] when separation complete_

- [ ] 67. Replace hard-coded HTTP buffer constants
  - File: cross-compile/onvif/src/networking/http/http_server.c, cross-compile/onvif/src/networking/http/http_parser.c
  - Remove remaining BUFFER_SIZE/ONVIF_RESPONSE_BUFFER_SIZE constants from HTTP transport
  - Adopt dynamic allocation and size hints from smart response builders throughout
  - Purpose: Fulfill Requirement 5 dynamic allocation mandate
  - _Leverage: smart response builder APIs, allocation strategy utilities_
  - _Requirements: 5_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Memory Optimization Engineer with HTTP expertise | Task: Replace hard-coded buffer constants in HTTP transport with dynamic allocations guided by smart response builders | Restrictions: Preserve performance targets, ensure allocations tracked via memory_manager, maintain zero-copy parsing | Success: No static buffer constants remain in HTTP transport, dynamic sizing validated by tests | Instructions: Mark [-] when starting, replace constants, run memory optimization tests, mark [x] when dynamic sizing complete_

- [ ] 68. Implement generic snapshot routing
  - File: cross-compile/onvif/src/networking/http/http_server.c
  - Update snapshot handling to use service delegation framework instead of HTTP-specific logic
  - Ensure routing aligns with ONVIF snapshot service patterns
  - Purpose: Complete Requirement 5 snapshot routing compliance
  - _Leverage: services/snapshot/onvif_snapshot.h, service dispatcher utilities_
  - _Requirements: 5_
  - _Prompt: Implement the task for spec onvif-http-refactoring, first run spec-workflow-guide to get the workflow guide then implement the task:  Role: Snapshot Service Engineer with routing expertise | Task: Rework snapshot HTTP endpoint to use generic service delegation leveraging snapshot service handlers | Restrictions: Maintain ONVIF compliance, ensure existing functionality preserved, follow logging conventions | Success: Snapshot requests routed via service dispatcher, HTTP layer contains no business logic, regression tests pass | Instructions: Mark [-] when starting, refactor routing, add regression tests, mark [x] when routing compliant_

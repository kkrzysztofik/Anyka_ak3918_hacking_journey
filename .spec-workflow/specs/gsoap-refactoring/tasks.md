# Tasks Document: gSOAP Refactoring

## Phase 1: Core Infrastructure & Context Improvement

- [x] 1. Update gSOAP context structure in onvif_gsoap.h
  - File: src/protocol/gsoap/onvif_gsoap.h
  - Replace pointer-based context with embedded soap structure
  - Add request_state, response_state, and error_context sub-structures
  - Remove old `struct soap* soap` member, add `struct soap soap` member
  - Purpose: Eliminate dynamic allocation and improve error tracking
  - _Leverage: Existing error_handling.h constants_
  - _Requirements: 9.1, 9.2, 9.3, 9.4_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C Systems Developer specializing in embedded structures and performance optimization | Task: Replace the pointer-based gSOAP context structure with an embedded design following requirements 9.1-9.4. Update onvif_gsoap_context_t to use embedded `struct soap soap` instead of pointer, and add request_state, response_state, and error_context sub-structures as defined in the design document. Ensure proper alignment and packing. | Restrictions: Do not modify gSOAP library code, maintain binary compatibility with struct soap, follow project naming conventions for struct members | Success: Structure compiles without warnings, embedded soap context is properly aligned, new state tracking fields are accessible, context size is within acceptable limits (~350 bytes) | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [x] 2. Implement new initialization function in onvif_gsoap_core.c
  - File: src/protocol/gsoap/onvif_gsoap_core.c (new file)
  - Implement `onvif_gsoap_init()` using `soap_init()` instead of `soap_new()`
  - Initialize all state tracking structures (request_state, response_state, error_context)
  - Remove dynamic allocation code
  - Purpose: Eliminate allocation failure path and improve initialization
  - _Leverage: gSOAP soap_init() API, utils/error/error_handling.h_
  - _Requirements: 9.5, 9.8_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C Developer with expertise in gSOAP library and initialization patterns | Task: Implement onvif_gsoap_init() function following requirements 9.5 and 9.8 using soap_init() on embedded context. Initialize request_state, response_state, and error_context to zero. Configure soap context with SOAP_C_UTFSTRING mode and set namespaces. Return ONVIF_SUCCESS or appropriate error code. | Restrictions: Must not use soap_new() or any dynamic allocation, must initialize all struct members to known state, follow project error code patterns | Success: Function initializes context without allocation, all state structures are zeroed, soap context is properly configured, returns appropriate error codes, no memory leaks | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [x] 3. Implement new cleanup function in onvif_gsoap_core.c
  - File: src/protocol/gsoap/onvif_gsoap_core.c
  - Implement `onvif_gsoap_cleanup()` using `soap_done()` instead of `soap_free()`
  - Clear all state tracking structures
  - Remove deallocation code
  - Purpose: Proper cleanup without deallocation for embedded context
  - _Leverage: gSOAP soap_destroy(), soap_end(), soap_done() API_
  - _Requirements: 9.6_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C Developer with expertise in resource cleanup and gSOAP library | Task: Implement onvif_gsoap_cleanup() function following requirement 9.6 using soap_destroy(), soap_end(), and soap_done() on embedded context. Clear all state tracking structures with memset. Ensure no memory leaks from gSOAP managed memory. | Restrictions: Must not use soap_free(), must call cleanup functions in correct order (destroy, end, done), must handle NULL context gracefully | Success: Function cleans up gSOAP resources properly, all state is cleared, no memory leaks detected with valgrind, handles NULL input safely | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [x] 4. Implement enhanced error reporting functions in onvif_gsoap_core.c
  - File: src/protocol/gsoap/onvif_gsoap_core.c
  - Implement `onvif_gsoap_set_error()` to store detailed error context
  - Implement `onvif_gsoap_get_detailed_error()` to retrieve full error info
  - Update `onvif_gsoap_has_error()` and `onvif_gsoap_get_error()` to use new error_context
  - Purpose: Provide detailed error debugging capabilities
  - _Leverage: __func__ macro for location tracking, utils/error/error_handling.h_
  - _Requirements: 9.7_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C Developer with expertise in error handling and debugging support | Task: Implement error reporting functions following requirement 9.7. Create onvif_gsoap_set_error() to store error_code, location (__func__), message (via snprintf), and soap_error_code. Create onvif_gsoap_get_detailed_error() to return error message and output error details through pointer parameters. Update existing error functions to use new error_context structure. | Restrictions: Must safely handle NULL pointers, message buffer must not overflow (use snprintf), error location must be captured from caller context | Success: Functions store and retrieve complete error context, NULL parameters handled safely, error messages are properly formatted and bounded, existing error API remains functional | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [x] 5. Update onvif_gsoap_init_request_parsing() for state tracking in onvif_gsoap_core.c
  - File: src/protocol/gsoap/onvif_gsoap_core.c
  - Update function to set `request_state.is_initialized = true`
  - Store `request_state.request_size`
  - Record `request_state.parse_start_time`
  - Fix existing initialization logic for embedded context
  - Purpose: Enable request parsing state tracking
  - _Leverage: get_timestamp_us() utility function_
  - _Requirements: 9.8, 9.9_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C Developer with expertise in gSOAP request parsing and state management | Task: Update onvif_gsoap_init_request_parsing() following requirements 9.8 and 9.9 to set request_state.is_initialized, store request_size, and capture parse_start_time using get_timestamp_us(). Configure embedded soap context for parsing (soap_begin(), set soap.is pointer). Handle initialization errors properly. | Restrictions: Must validate context before accessing members, must properly configure soap input stream for embedded context, must handle errors and set error_context | Success: Function properly initializes request parsing state, all state fields are set correctly, soap input stream configured for embedded context, invalid state detection works, errors are tracked | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [x] 6. Create onvif_gsoap_core.h header file
  - File: src/protocol/gsoap/onvif_gsoap_core.h (new file)
  - Declare all core initialization, cleanup, and error functions
  - Include necessary gSOAP and project headers
  - Add Doxygen documentation for all functions
  - Purpose: Provide clean API for core gSOAP functionality
  - _Leverage: Project Doxygen documentation standards from AGENTS.md_
  - _Requirements: 10.5, 10.6_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C Developer with expertise in API design and documentation | Task: Create onvif_gsoap_core.h header following requirements 10.5 and 10.6. Declare all core functions (init, cleanup, reset, error handling, request parsing initialization). Include complete Doxygen documentation with @brief, @param, @return, @note tags following AGENTS.md standards. Use include guards. | Restrictions: Must not expose internal implementation details, follow project include path standards (relative from src/), maintain consistent function naming, use proper Doxygen format | Success: Header compiles cleanly, all functions properly declared, complete Doxygen documentation, follows project standards, can be included by service modules | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [x] 7. Update Makefile to compile new modular structure
  - File: cross-compile/onvif/Makefile
  - Add onvif_gsoap_core.c to SOURCES
  - Add future service module files (media, ptz, device, imaging) to SOURCES
  - Update dependencies for modular compilation
  - Purpose: Enable compilation of new modular file structure
  - _Leverage: Existing Makefile patterns and arm-anykav200-linux-uclibcgnueabi toolchain_
  - _Requirements: 10.8_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: Build Engineer with expertise in Makefile and cross-compilation | Task: Update Makefile following requirement 10.8 to add onvif_gsoap_core.c and future service module files (onvif_gsoap_media.c, onvif_gsoap_ptz.c, onvif_gsoap_device.c, onvif_gsoap_imaging.c). Update SOURCES variable, maintain existing compiler flags and include paths. Ensure proper dependency tracking. | Restrictions: Must maintain cross-compilation compatibility, must not break existing builds, follow existing Makefile patterns and variable naming | Success: Makefile compiles new files successfully, proper dependency tracking works, incremental builds function correctly, cross-compilation produces valid ARM binaries | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

## Phase 2: Media Service Implementation

- [x] 8. Create onvif_gsoap_media.h header with Media service parsing functions
  - File: src/protocol/gsoap/onvif_gsoap_media.h (new file)
  - Declare 6 Media service parsing functions (GetProfiles, GetStreamUri, CreateProfile, DeleteProfile, SetVideoSourceConfig, SetVideoEncoderConfig)
  - Add complete Doxygen documentation
  - Include onvif_gsoap_core.h and gSOAP generated headers
  - Purpose: Define Media service parsing API
  - _Leverage: gSOAP generated _trt__* structures, onvif_gsoap_core.h_
  - _Requirements: 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 10.6_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C API Designer with expertise in ONVIF protocol and gSOAP | Task: Create onvif_gsoap_media.h header following requirements 3.1-3.6 and 10.6. Declare 6 Media service parsing functions with signature: int onvif_gsoap_parse_[operation](onvif_gsoap_context_t* ctx, struct _trt__[Operation]** out). Add complete Doxygen documentation. Include necessary headers. Use include guards. | Restrictions: Follow project include path standards, consistent function naming (lowercase with underscores), proper pointer types for output parameters, include Doxygen file header | Success: Header declares all 6 functions correctly, complete documentation, compiles without errors, proper include structure | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [x] 9. Implement onvif_gsoap_parse_get_profiles() in onvif_gsoap_media.c
  - File: src/protocol/gsoap/onvif_gsoap_media.c (new file)
  - Implement parsing function using soap_read__trt__GetProfiles()
  - Follow parsing function pattern from design document
  - Add state tracking and error reporting
  - Purpose: Parse GetProfiles ONVIF request
  - _Leverage: gSOAP soap_read__trt__GetProfiles(), onvif_gsoap_set_error()_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C Developer with expertise in gSOAP deserialization and ONVIF protocol | Task: Implement onvif_gsoap_parse_get_profiles() following requirement 3.1 and design document parsing pattern. Validate parameters, check request_state.is_initialized, set operation_name and parse_start_time, call soap_new__trt__GetProfiles(), call soap_read__trt__GetProfiles(), set parse_end_time, use onvif_gsoap_set_error() on failures. Return appropriate error codes. | Restrictions: Must follow exact parsing pattern from design, must validate all inputs, must track timing, must use onvif_gsoap_set_error() for all errors | Success: Function parses valid requests successfully, invalid inputs return proper errors, timing is tracked, error context is set on failures, follows design pattern exactly | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [ ] 10. Implement remaining 5 Media service parsing functions in onvif_gsoap_media.c
  - File: src/protocol/gsoap/onvif_gsoap_media.c
  - Implement GetStreamUri, CreateProfile, DeleteProfile, SetVideoSourceConfig, SetVideoEncoderConfig
  - Follow consistent parsing pattern for all functions
  - Add proper error handling and state tracking
  - Purpose: Complete Media service request parsing
  - _Leverage: gSOAP soap_read__trt__* functions, parsing pattern from task 9_
  - _Requirements: 3.2, 3.3, 3.4, 3.5, 3.6_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C Developer with expertise in gSOAP and ONVIF Media service | Task: Implement 5 remaining Media service parsing functions following requirements 3.2-3.6. Use identical pattern from onvif_gsoap_parse_get_profiles(). For each: validate parameters, check initialization, set operation name, allocate with soap_new__trt__[Operation](), deserialize with soap_read__trt__[Operation](), track timing, handle errors with onvif_gsoap_set_error(). | Restrictions: Must use identical pattern for consistency, each function must be self-contained, proper gSOAP structure types for each operation | Success: All 5 functions implemented correctly, consistent error handling, proper timing tracking, follows established pattern, no code duplication beyond pattern structure | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [ ] 11. Add Doxygen file header and function documentation to onvif_gsoap_media.c
  - File: src/protocol/gsoap/onvif_gsoap_media.c
  - Add file header with @file, @brief, @author, @date tags
  - Ensure all functions have complete Doxygen comments
  - Document gSOAP structures being parsed
  - Purpose: Maintain documentation standards
  - _Leverage: Project documentation standards from AGENTS.md_
  - _Requirements: 10.9_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: Technical Writer with C programming knowledge | Task: Add complete Doxygen documentation to onvif_gsoap_media.c following requirement 10.9 and AGENTS.md standards. Add file header with @file, @brief, @author kkrzysztofik, @date 2025. Ensure all 6 functions have @brief, @param, @return, @note tags. Document what each function parses and how output structure is populated. | Restrictions: Must follow project Doxygen style, must not modify function implementations, author must be kkrzysztofik, date must be 2025 | Success: Complete file header, all functions fully documented, documentation is accurate and helpful, generates clean Doxygen output | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

## Phase 3: PTZ Service Implementation

- [ ] 12. Create onvif_gsoap_ptz.h header with PTZ service parsing functions
  - File: src/protocol/gsoap/onvif_gsoap_ptz.h (new file)
  - Declare 6 PTZ service parsing functions (GetNodes, AbsoluteMove, GetPresets, SetPreset, GotoPreset, RemovePreset)
  - Add complete Doxygen documentation
  - Purpose: Define PTZ service parsing API
  - _Leverage: gSOAP generated _tptz__* structures, onvif_gsoap_core.h_
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 10.6_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C API Designer with expertise in ONVIF PTZ protocol and gSOAP | Task: Create onvif_gsoap_ptz.h header following requirements 4.1-4.6 and 10.6. Declare 6 PTZ service parsing functions with signature: int onvif_gsoap_parse_[operation](onvif_gsoap_context_t* ctx, struct _tptz__[Operation]** out). Add complete Doxygen documentation including PTZ-specific notes about Position, Speed, and PresetToken fields. | Restrictions: Follow project standards, consistent naming, proper gSOAP structure types (_tptz__ prefix), include file header | Success: Header declares all 6 PTZ functions correctly, PTZ-specific documentation, compiles cleanly | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [ ] 13. Implement all 6 PTZ service parsing functions in onvif_gsoap_ptz.c
  - File: src/protocol/gsoap/onvif_gsoap_ptz.c (new file)
  - Implement GetNodes, AbsoluteMove, GetPresets, SetPreset, GotoPreset, RemovePreset
  - Follow established parsing pattern from Media service
  - Add comprehensive error handling and state tracking
  - Purpose: Complete PTZ service request parsing
  - _Leverage: gSOAP soap_read__tptz__* functions, Media service parsing pattern_
  - _Requirements: 4.1, 4.2, 4.3, 4.4, 4.5, 4.6_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C Developer with expertise in gSOAP and ONVIF PTZ protocol | Task: Implement all 6 PTZ parsing functions following requirements 4.1-4.6. Use same pattern as Media service: validate, check initialization, set operation name, allocate with soap_new__tptz__[Operation](), deserialize with soap_read__tptz__[Operation](), track timing, handle errors. Include file header and function documentation. | Restrictions: Must use consistent pattern, must handle PTZ-specific structures (PTZVector, PTZSpeed), proper error handling for all cases | Success: All 6 functions implemented correctly, PTZ structures parsed properly, consistent with Media service pattern, complete documentation | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

## Phase 4: Device & Imaging Services Implementation

- [ ] 14. Create onvif_gsoap_device.h header with Device service parsing functions
  - File: src/protocol/gsoap/onvif_gsoap_device.h (new file)
  - Declare 4 Device service parsing functions (GetDeviceInformation, GetCapabilities, GetSystemDateAndTime, SystemReboot)
  - Add complete Doxygen documentation
  - Purpose: Define Device service parsing API
  - _Leverage: gSOAP generated _tds__* structures, onvif_gsoap_core.h_
  - _Requirements: 5.1, 5.2, 5.3, 5.4, 10.6_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C API Designer with expertise in ONVIF Device service protocol | Task: Create onvif_gsoap_device.h header following requirements 5.1-5.4 and 10.6. Declare 4 Device service parsing functions with signature: int onvif_gsoap_parse_[operation](onvif_gsoap_context_t* ctx, struct _tds__[Operation]** out). Add complete Doxygen documentation. | Restrictions: Follow project standards, _tds__ prefix for Device service structures, include file header | Success: Header declares all 4 Device functions, complete documentation, compiles cleanly | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [ ] 15. Implement all 4 Device service parsing functions in onvif_gsoap_device.c
  - File: src/protocol/gsoap/onvif_gsoap_device.c (new file)
  - Implement GetDeviceInformation, GetCapabilities, GetSystemDateAndTime, SystemReboot
  - Follow established parsing pattern
  - Purpose: Complete Device service request parsing
  - _Leverage: gSOAP soap_read__tds__* functions, established parsing pattern_
  - _Requirements: 5.1, 5.2, 5.3, 5.4_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C Developer with expertise in ONVIF Device service | Task: Implement all 4 Device parsing functions following requirements 5.1-5.4 using established pattern. Note that some Device operations (GetDeviceInformation, GetSystemDateAndTime, SystemReboot) have empty request structures - still follow pattern but deserialization is simpler. Include complete documentation. | Restrictions: Must use consistent pattern even for empty requests, handle Capabilities array properly | Success: All 4 functions implemented, empty requests handled correctly, documentation complete | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [ ] 16. Create onvif_gsoap_imaging.h header with Imaging service parsing functions
  - File: src/protocol/gsoap/onvif_gsoap_imaging.h (new file)
  - Declare 2 Imaging service parsing functions (GetImagingSettings, SetImagingSettings)
  - Add complete Doxygen documentation
  - Purpose: Define Imaging service parsing API
  - _Leverage: gSOAP generated _timg__* structures, onvif_gsoap_core.h_
  - _Requirements: 5.5, 5.6, 10.6_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C API Designer with expertise in ONVIF Imaging service protocol | Task: Create onvif_gsoap_imaging.h header following requirements 5.5-5.6 and 10.6. Declare 2 Imaging service parsing functions with signature: int onvif_gsoap_parse_[operation](onvif_gsoap_context_t* ctx, struct _timg__[Operation]** out). Document VideoSourceToken and ImagingSettings fields. | Restrictions: Follow project standards, _timg__ prefix for Imaging service structures | Success: Header declares both functions, complete documentation, compiles cleanly | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [ ] 17. Implement both Imaging service parsing functions in onvif_gsoap_imaging.c
  - File: src/protocol/gsoap/onvif_gsoap_imaging.c (new file)
  - Implement GetImagingSettings and SetImagingSettings
  - Follow established parsing pattern
  - Purpose: Complete Imaging service request parsing
  - _Leverage: gSOAP soap_read__timg__* functions, established parsing pattern_
  - _Requirements: 5.5, 5.6_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C Developer with expertise in ONVIF Imaging service | Task: Implement both Imaging parsing functions following requirements 5.5-5.6 using established pattern. Handle VideoSourceToken extraction and ImagingSettings structure (Brightness, Contrast, Saturation, etc.). Include complete documentation. | Restrictions: Must use consistent pattern, handle ImagingSettings nested structure properly | Success: Both functions implemented correctly, ImagingSettings parsed properly, documentation complete | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

## Phase 5: Remove Legacy Code

- [ ] 18. Delete obsolete manual parsing functions from onvif_gsoap.c
  - File: src/protocol/gsoap/onvif_gsoap.c
  - Delete all `onvif_gsoap_parse_*_token()` functions (PTZ, Media, Snapshot profile tokens)
  - Delete `onvif_gsoap_parse_value()`, `onvif_gsoap_parse_boolean()`, `onvif_gsoap_parse_integer()`
  - Delete `onvif_gsoap_parse_configuration_token()` and `onvif_gsoap_parse_protocol()`
  - Delete helper functions: `parse_integer_value()`, `parse_boolean_value()`, `extract_element_name_from_xpath()`, `find_and_parse_element_value()`
  - Purpose: Remove ~800 lines of obsolete manual parsing code
  - _Leverage: None - pure deletion_
  - _Requirements: 1.1, 1.2, 1.3_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C Developer with code refactoring expertise | Task: Delete all manual parsing functions from onvif_gsoap.c following requirements 1.1-1.3. Remove: onvif_gsoap_parse_ptz_profile_token, onvif_gsoap_parse_media_profile_token, onvif_gsoap_parse_snapshot_profile_token, onvif_gsoap_parse_configuration_token, onvif_gsoap_parse_protocol, onvif_gsoap_parse_value, onvif_gsoap_parse_boolean, onvif_gsoap_parse_integer, and all helper functions. Keep response generation functions intact. | Restrictions: Do not delete response generation functions, do not delete context management functions (those moved to core), verify no other code calls these functions before deletion | Success: ~800 lines of manual parsing code removed, file compiles after deletion, only new API remains, no dangling function calls | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [ ] 19. Remove obsolete function declarations from onvif_gsoap.h
  - File: src/protocol/gsoap/onvif_gsoap.h
  - Delete declarations for all removed manual parsing functions
  - Keep response generation function declarations
  - Purpose: Clean up header to match implementation
  - _Leverage: None - pure deletion_
  - _Requirements: 1.4_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C Developer with header maintenance expertise | Task: Remove obsolete function declarations from onvif_gsoap.h following requirement 1.4. Delete declarations for all manual parsing functions removed in task 18. Keep response generation declarations. Verify header is consistent with implementation. | Restrictions: Do not remove response generation declarations, maintain proper include structure, keep typedef and struct definitions needed by other code | Success: Header only declares functions that exist in implementation, no compilation errors, header is clean and consistent | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [ ] 20. Update main onvif_gsoap.h to include service module headers
  - File: src/protocol/gsoap/onvif_gsoap.h
  - Add includes for onvif_gsoap_core.h, onvif_gsoap_media.h, onvif_gsoap_ptz.h, onvif_gsoap_device.h, onvif_gsoap_imaging.h
  - Maintain as central header for all gSOAP functionality
  - Purpose: Provide unified header interface
  - _Leverage: Project include path standards_
  - _Requirements: 10.7_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C Developer with header organization expertise | Task: Update onvif_gsoap.h to include all service module headers following requirement 10.7. Add includes for onvif_gsoap_core.h, onvif_gsoap_media.h, onvif_gsoap_ptz.h, onvif_gsoap_device.h, onvif_gsoap_imaging.h using relative paths from src/. This provides backward compatibility - code including onvif_gsoap.h gets access to all APIs. | Restrictions: Use relative include paths (protocol/gsoap/onvif_gsoap_*.h), maintain include guards, preserve existing typedef and struct definitions | Success: Main header includes all service headers, provides unified API access, compiles without errors or warnings | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

## Phase 6: Service Layer Updates

- [ ] 21. Update service_dispatcher.c to use new Media parsing functions
  - File: src/services/common/service_dispatcher.c
  - Replace calls to `onvif_gsoap_parse_media_profile_token()` with `onvif_gsoap_parse_get_stream_uri()`, etc.
  - Update Media service handlers to receive parsed request structures
  - Extract data from structures instead of token buffers
  - Purpose: Integrate new Media parsing API with service layer
  - _Leverage: New onvif_gsoap_media.h API, existing service handler patterns_
  - _Requirements: 6.1, 6.2_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C Developer with expertise in service architecture and ONVIF protocol | Task: Update service_dispatcher.c to use new Media parsing functions following requirements 6.1-6.2. Replace manual token parsing with calls to onvif_gsoap_parse_get_profiles(), onvif_gsoap_parse_get_stream_uri(), etc. Update service handlers to accept struct _trt__* parameters. Extract tokens and data from parsed structures (e.g., request->ProfileToken). Handle parsing errors properly. | Restrictions: Must maintain existing service handler logic, only change how requests are parsed and data is extracted, preserve error handling behavior | Success: Media service uses new parsing API, data extracted from structures correctly, service handlers work with parsed requests, integration tested | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [ ] 22. Update service_dispatcher.c to use new PTZ parsing functions
  - File: src/services/common/service_dispatcher.c
  - Replace calls to `onvif_gsoap_parse_ptz_profile_token()` with `onvif_gsoap_parse_absolute_move()`, etc.
  - Update PTZ service handlers to receive parsed request structures
  - Extract ProfileToken, Position, Speed, PresetToken from structures
  - Purpose: Integrate new PTZ parsing API with service layer
  - _Leverage: New onvif_gsoap_ptz.h API, existing PTZ service handler patterns_
  - _Requirements: 6.3_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C Developer with expertise in ONVIF PTZ protocol | Task: Update service_dispatcher.c to use new PTZ parsing functions following requirement 6.3. Replace manual parsing with calls to onvif_gsoap_parse_get_nodes(), onvif_gsoap_parse_absolute_move(), onvif_gsoap_parse_get_presets(), etc. Update PTZ handlers to accept struct _tptz__* parameters. Extract ProfileToken from request->ProfileToken, extract Position from request->Position->PanTilt and request->Position->Zoom, extract PresetToken from request->PresetToken. | Restrictions: Must handle optional fields safely (Speed can be NULL), maintain PTZ control logic, preserve coordinate space handling | Success: PTZ service uses new parsing API, Position and Speed extracted correctly, preset operations work, PTZ movements tested | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [ ] 23. Update service_dispatcher.c to use new Device and Imaging parsing functions
  - File: src/services/common/service_dispatcher.c
  - Integrate Device service parsing (GetDeviceInformation, GetCapabilities, GetSystemDateAndTime, SystemReboot)
  - Integrate Imaging service parsing (GetImagingSettings, SetImagingSettings)
  - Update handlers to use parsed structures
  - Purpose: Complete service layer integration for all ONVIF services
  - _Leverage: New onvif_gsoap_device.h and onvif_gsoap_imaging.h APIs_
  - _Requirements: 6.4_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: C Developer with expertise in ONVIF Device and Imaging services | Task: Update service_dispatcher.c to use new Device and Imaging parsing functions following requirement 6.4. For Device: call parsing functions for GetDeviceInformation, GetCapabilities (extract Category array), GetSystemDateAndTime, SystemReboot. For Imaging: call parsing functions and extract VideoSourceToken and ImagingSettings (Brightness, Contrast, etc.). Update handlers accordingly. | Restrictions: Handle empty request structures properly (some Device operations), handle Capabilities array safely, validate ImagingSettings ranges | Success: All Device and Imaging operations use new parsing API, data extracted correctly, handlers work properly, tested | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

## Phase 7: Test Infrastructure

- [ ] 24. Create SOAP envelope test data header
  - File: tests/data/soap_test_envelopes.h (new file)
  - Define SOAP 1.2 envelope header and footer macros
  - Create valid SOAP request templates for all 18 operations (6 Media, 6 PTZ, 4 Device, 2 Imaging)
  - Create invalid/malformed request templates for error testing
  - Purpose: Provide realistic SOAP test data for unit tests
  - _Leverage: ONVIF 2.5 specification, real ONVIF client requests_
  - _Requirements: 7.1, 7.2, 7.3_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: Test Engineer with expertise in ONVIF protocol and XML | Task: Create soap_test_envelopes.h following requirements 7.1-7.3. Define SOAP_ENVELOPE_HEADER and SOAP_ENVELOPE_FOOTER macros with proper ONVIF namespaces (s, tds, trt, tptz, timg, tt). Create complete SOAP 1.2 envelopes for all 18 operations with realistic parameter values. Create invalid request templates: SOAP_INVALID_XML, SOAP_INVALID_NAMESPACE, SOAP_MISSING_REQUIRED_PARAM. Use string constants, not files. | Restrictions: Must use SOAP 1.2 format, must include all required ONVIF namespaces, requests must be ONVIF 2.5 compliant, use proper XML formatting | Success: All 18 valid requests parse successfully with gSOAP, invalid requests fail as expected, envelopes are realistic and representative of real clients | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [ ] 25. Rewrite test_onvif_gsoap.c with new parsing tests
  - File: tests/src/unit/protocol/test_onvif_gsoap.c
  - Delete obsolete tests for manual parsing functions (6 tests)
  - Implement new tests for all 18 parsing functions
  - Add setup_parsing_test() helper function
  - Add error handling tests (NULL context, NULL output, invalid XML, missing params)
  - Purpose: Comprehensive unit test coverage for new parsing API
  - _Leverage: CMocka framework, soap_test_envelopes.h_
  - _Requirements: 7.4, 7.5, 7.6, 7.7, 8.1, 8.2, 8.3, 8.4, 8.5, 8.6, 8.7_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: Test Engineer with expertise in C unit testing and CMocka | Task: Completely rewrite test_onvif_gsoap.c following requirements 7.4-7.7 and 8.1-8.7. Delete obsolete tests (test_unit_onvif_gsoap_parse_profile_token, etc.). Create setup_parsing_test() helper that initializes context and calls onvif_gsoap_init_request_parsing(). Write test for each of 18 parsing functions: test NULL parameters, test valid request parsing, verify parsed fields match expected values (ProfileToken, Position.x/y, StreamSetup.Protocol, etc.). Add error tests: test_unit_onvif_gsoap_parse_invalid_xml, test_unit_onvif_gsoap_parse_invalid_namespace, test_unit_onvif_gsoap_parse_missing_required_param. Register all tests in main(). | Restrictions: Must use CMocka framework, must clean up context after each test, must test both success and failure paths, each test must be independent | Success: All 18 parsing functions have tests, error cases covered, tests pass, no memory leaks (valgrind clean), 100% coverage of new parsing functions | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [ ] 26. Create integration tests for request-response cycles
  - File: tests/src/integration/test_gsoap_integration.c (new file)
  - Implement integration tests for complete parse → process → response cycles
  - Test Media service: GetStreamUri end-to-end
  - Test PTZ service: AbsoluteMove end-to-end
  - Test response XML validity and content
  - Purpose: Validate complete ONVIF operation flows
  - _Leverage: soap_test_envelopes.h, existing response generation functions, CMocka_
  - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: Integration Test Engineer with expertise in end-to-end testing | Task: Create integration tests following requirements 9.1-9.5. For GetStreamUri: parse request, call service handler, generate response, verify response contains rtsp:// URI. For AbsoluteMove: parse request with Position coordinates, call PTZ handler, generate response, verify success. Add test_integration_onvif_gsoap_full_request_response_cycle() that demonstrates complete flow. Verify response XML structure with basic parsing. | Restrictions: Must test realistic flows, must verify response content not just success codes, should not mock service layer | Success: Integration tests pass, complete flows validated, response XML is valid and contains expected data, tests demonstrate end-to-end functionality | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [ ] 27. Run unit tests and verify all pass
  - File: N/A - verification task
  - Run `make test` to execute all unit tests
  - Verify all tests pass without failures or segfaults
  - Run `make test-valgrind` to check for memory leaks
  - Purpose: Validate implementation correctness
  - _Leverage: CMocka test framework, valgrind_
  - _Requirements: 7.8_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: QA Engineer with expertise in test execution and debugging | Task: Execute unit test suite following requirement 7.8. Run `make test` and verify all tests pass (0 failures). Run `make test-valgrind` and verify no memory leaks detected. If tests fail, analyze failures, debug issues, and work with implementation team to fix. Iterate until all tests pass cleanly. | Restrictions: Must achieve 100% test pass rate, must have zero memory leaks, must not skip or disable failing tests | Success: All unit tests pass (100%), valgrind reports no memory leaks, test execution is reliable and repeatable | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

## Phase 8: Documentation & Validation

- [ ] 28. Generate Doxygen documentation
  - File: N/A - documentation generation task
  - Run `make docs` to generate Doxygen documentation
  - Verify all new functions are documented
  - Check for documentation warnings or errors
  - Review generated HTML output
  - Purpose: Ensure complete API documentation
  - _Leverage: Doxygen configuration in cross-compile/onvif/Doxyfile_
  - _Requirements: 10.9_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: Technical Writer with Doxygen expertise | Task: Generate and review Doxygen documentation following requirement 10.9. Run `make docs` and verify successful generation. Check docs/html/index.html output. Verify all new modules (core, media, ptz, device, imaging) appear in documentation. Verify all 18 parsing functions are documented with complete parameter and return descriptions. Fix any Doxygen warnings. Ensure file headers are present and correctly formatted. | Restrictions: Must fix all Doxygen warnings, documentation must be accurate, must not modify functional code to fix doc issues unless necessary | Success: Doxygen generates without warnings, all modules and functions documented, HTML output is complete and readable, documentation accurately describes implementation | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [ ] 29. Run code formatting and linting
  - File: N/A - code quality validation task
  - Run `./cross-compile/onvif/scripts/format_code.sh --check` to verify formatting
  - Run `./cross-compile/onvif/scripts/lint_code.sh --check` to verify code quality
  - Fix any formatting or linting issues
  - Purpose: Ensure code quality standards compliance
  - _Leverage: Project formatting and linting scripts_
  - _Requirements: Per CLAUDE.md code quality validation is MANDATORY_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: Code Quality Engineer with expertise in C coding standards | Task: Validate code quality per CLAUDE.md requirements. Run `./cross-compile/onvif/scripts/format_code.sh --check` and verify all files pass (exit 0). Run `./cross-compile/onvif/scripts/lint_code.sh --check` and verify no issues (exit 0). If issues found, run without --check flag to auto-fix formatting, manually fix linting issues. Verify function ordering: definitions at top, execution logic at bottom. Verify global variables at top after includes. Verify include ordering: system → third-party → project. | Restrictions: Must fix all issues, must not disable linting rules, code must follow project standards exactly | Success: Formatting check passes (exit 0), linting check passes (exit 0), function ordering correct, global variables placed properly, includes ordered correctly | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [ ] 30. Build and test on target hardware
  - File: N/A - integration testing task
  - Build with `make release` for ARM target
  - Deploy to SD card: copy `out/onvifd` to `SD_card_contents/anyka_hack/usr/bin/`
  - Test on Anyka AK3918 camera with real ONVIF clients
  - Verify all services work correctly (Media, PTZ, Device, Imaging)
  - Purpose: Validate implementation on actual hardware
  - _Leverage: SD card payload deployment system, ONVIF Device Manager for testing_
  - _Requirements: All functional requirements_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: Embedded Systems Engineer with expertise in hardware testing | Task: Build and validate on target hardware covering all requirements. Build with `make release`, verify clean build. Deploy to SD card payload directory. Boot camera with SD card. Test with ONVIF Device Manager or similar client: verify device discovery, get device info, get capabilities, get profiles, get stream URI (verify RTSP URL), test PTZ absolute move, test presets, test imaging settings. Document any issues. | Restrictions: Must test on actual hardware, must use real ONVIF clients, must not modify flash (SD card payload only) | Success: Clean build for ARM target, deploys successfully, all ONVIF operations work on real hardware, tested with real clients, no functional regressions | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [ ] 31. Performance benchmarking
  - File: N/A - performance measurement task
  - Measure request parsing time for all 18 operations
  - Compare with old manual parsing performance (if baseline available)
  - Verify parsing completes in < 1ms per request on target hardware
  - Document performance metrics
  - Purpose: Validate performance improvements
  - _Leverage: request_state timing fields (parse_start_time, parse_end_time)_
  - _Requirements: Per design, parsing should be 50-60% faster_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: Performance Engineer with expertise in embedded systems benchmarking | Task: Measure and validate parsing performance. Create test program that parses each of 18 operations 1000 times, measuring time using request_state.parse_start_time and parse_end_time. Calculate average, min, max, percentiles. Verify all operations parse in < 1ms average on Anyka AK3918. Compare with baseline if available. Document results in performance report. Test memory allocation count (should be 0 for context init, 1 per request for structure). | Restrictions: Must test on actual hardware, must use realistic SOAP envelopes, must run multiple iterations for statistical validity | Success: All operations parse in < 1ms on target, performance improvement documented (ideally 50-60% faster vs baseline), no unexpected performance regressions, memory allocation is minimal | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [ ] 32. ONVIF client compatibility testing
  - File: N/A - compatibility testing task
  - Test with ONVIF Device Test Tool (if available)
  - Test with multiple ONVIF clients (ONVIF Device Manager, Milestone, Blue Iris, etc.)
  - Verify ONVIF 2.5 Profile S compliance
  - Document any compatibility issues
  - Purpose: Ensure broad ONVIF client compatibility
  - _Leverage: Real ONVIF clients, ONVIF conformance tools_
  - _Requirements: Per design, implementation must be ONVIF 2.5 compliant_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: QA Engineer with expertise in ONVIF protocol and client compatibility | Task: Validate ONVIF 2.5 compliance and client compatibility. Test with ONVIF Device Manager: verify discovery, device info, capabilities, media profiles, stream URIs (verify RTSP streams work), PTZ controls, imaging settings. Test with other clients if available (Milestone, Blue Iris, etc.). Test Profile S compliance. Document client compatibility matrix. Report any parsing issues or non-compliant behavior. | Restrictions: Must test with real clients, must verify actual streaming works (not just SOAP), should test both conformant and non-conformant client requests | Success: Works with major ONVIF clients, ONVIF 2.5 Profile S compliant, streams work correctly, PTZ controls functional, no breaking client compatibility issues | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

- [ ] 33. Final code review and cleanup
  - File: All modified/created files
  - Review all code changes for quality, correctness, security
  - Verify all requirements are met
  - Ensure no debug code or TODOs remain
  - Verify all error paths are tested
  - Purpose: Final quality gate before completion
  - _Leverage: Requirements document, design document, AGENTS.md standards_
  - _Requirements: All requirements_
  - _Prompt: Implement the task for spec gsoap-refactoring. First run spec-workflow-guide to get the workflow guide, then implement the task: Role: Senior C Developer with code review expertise | Task: Perform final code review covering all requirements. Review all modified files (onvif_gsoap.h, onvif_gsoap_core.c/.h, onvif_gsoap_media.c/.h, onvif_gsoap_ptz.c/.h, onvif_gsoap_device.c/.h, onvif_gsoap_imaging.c/.h, service_dispatcher.c, test files). Verify: all requirements implemented, no manual parsing remains, all error paths handled, proper error codes used, complete Doxygen docs, code follows AGENTS.md standards, no TODOs or debug code, proper resource cleanup, memory safety. Create review checklist and document findings. | Restrictions: Must check every requirement against implementation, must verify test coverage, must ensure security best practices | Success: All requirements verified as implemented, code quality meets standards, security review passed, no critical issues found, ready for production | Instructions: Mark task as in_progress [-] in tasks.md before starting. When complete, mark as completed [x] in tasks.md._

# Requirements Document: gSOAP Refactoring

## Introduction

This specification defines the complete refactoring of the ONVIF gSOAP protocol layer to eliminate manual XML parsing and fully leverage gSOAP's generated deserialization functions. The current implementation contains approximately 800 lines of manual XML string manipulation code in a monolithic `onvif_gsoap.c` file that attempts to parse ONVIF requests using non-existent or incorrectly used gSOAP APIs. This refactoring will replace all manual parsing with proper usage of gSOAP's generated functions, resulting in a type-safe, ONVIF 2.5 compliant, and maintainable implementation.

The refactoring will modernize the API to parse complete ONVIF request structures instead of extracting individual tokens, enabling structured access to request data throughout the service layer. Additionally, the monolithic gSOAP implementation will be split into service-specific modules (Media, PTZ, Device, Imaging) to improve maintainability and adhere to the single responsibility principle.

## Alignment with Product Vision

This refactoring aligns with the project's goals of creating a robust, maintainable ONVIF 2.5 implementation for Anyka AK3918-based IP cameras by:

- **Eliminating technical debt**: Removing ~800 lines of problematic manual parsing code
- **Improving code organization**: Splitting monolithic implementation into service-specific modules following single responsibility principle
- **Improving reliability**: Using gSOAP's validated deserialization functions ensures ONVIF specification compliance
- **Enhancing maintainability**: Type-safe structured access to request data reduces bugs and improves code clarity
- **Enabling future development**: Clean modular API foundation supports easy addition of new ONVIF operations
- **Ensuring security**: Proper input validation through gSOAP's parsing eliminates manual string manipulation vulnerabilities

## Requirements

### Requirement 1: Remove Legacy Manual Parsing Functions

**User Story:** As a developer, I want all manual XML parsing functions removed from the codebase, so that the implementation uses only validated gSOAP-generated functions and eliminates maintenance burden.

#### Acceptance Criteria

1. WHEN the refactoring is complete THEN all `onvif_gsoap_parse_*_token()` functions SHALL be deleted from `onvif_gsoap.c`
2. WHEN the refactoring is complete THEN all `onvif_gsoap_parse_value()`, `onvif_gsoap_parse_boolean()`, and `onvif_gsoap_parse_integer()` functions SHALL be deleted
3. WHEN the refactoring is complete THEN no manual XML string parsing or token extraction code SHALL remain in the protocol layer
4. WHEN legacy functions are removed THEN corresponding obsolete function declarations SHALL be removed from `onvif_gsoap.h`

### Requirement 2: Implement Proper gSOAP Request Initialization

**User Story:** As a developer, I want the gSOAP context properly configured for request parsing, so that `soap_read_*` functions can successfully deserialize ONVIF requests.

#### Acceptance Criteria

1. WHEN `onvif_gsoap_init_request_parsing()` is called THEN it SHALL properly configure `soap->is`, `soap->buflen`, and `soap->bufidx` for the input buffer
2. WHEN request parsing is initialized THEN manual SOAP envelope/body parsing SHALL be removed
3. WHEN request parsing is initialized THEN gSOAP's `soap_read_*` functions SHALL handle complete SOAP envelope processing
4. IF initialization fails THEN the function SHALL return an appropriate error code from `error_handling.h`

### Requirement 3: Implement Full Request Parsers for Media Service

**User Story:** As a developer, I want complete request parsing functions for all Media service operations, so that I can access structured request data instead of individual tokens.

#### Acceptance Criteria

1. WHEN implementing Media service parsers THEN the system SHALL provide `onvif_gsoap_parse_get_profiles()` returning `struct _trt__GetProfiles**`
2. WHEN implementing Media service parsers THEN the system SHALL provide `onvif_gsoap_parse_get_stream_uri()` returning `struct _trt__GetStreamUri**`
3. WHEN implementing Media service parsers THEN the system SHALL provide `onvif_gsoap_parse_create_profile()` returning `struct _trt__CreateProfile**`
4. WHEN implementing Media service parsers THEN the system SHALL provide `onvif_gsoap_parse_delete_profile()` returning `struct _trt__DeleteProfile**`
5. WHEN implementing Media service parsers THEN the system SHALL provide `onvif_gsoap_parse_set_video_source_config()` returning `struct _trt__SetVideoSourceConfiguration**`
6. WHEN implementing Media service parsers THEN the system SHALL provide `onvif_gsoap_parse_set_video_encoder_config()` returning `struct _trt__SetVideoEncoderConfiguration**`
7. WHEN any Media service parser is called THEN it SHALL use gSOAP's generated `soap_read_*` functions exclusively
8. IF parsing fails THEN the function SHALL return an error code and set the output pointer to NULL

### Requirement 4: Implement Full Request Parsers for PTZ Service

**User Story:** As a developer, I want complete request parsing functions for all PTZ service operations, so that I can access structured PTZ command data including positions, speeds, and preset information.

#### Acceptance Criteria

1. WHEN implementing PTZ service parsers THEN the system SHALL provide `onvif_gsoap_parse_get_nodes()` returning `struct _tptz__GetNodes**`
2. WHEN implementing PTZ service parsers THEN the system SHALL provide `onvif_gsoap_parse_absolute_move()` returning `struct _tptz__AbsoluteMove**`
3. WHEN implementing PTZ service parsers THEN the system SHALL provide `onvif_gsoap_parse_get_presets()` returning `struct _tptz__GetPresets**`
4. WHEN implementing PTZ service parsers THEN the system SHALL provide `onvif_gsoap_parse_set_preset()` returning `struct _tptz__SetPreset**`
5. WHEN implementing PTZ service parsers THEN the system SHALL provide `onvif_gsoap_parse_goto_preset()` returning `struct _tptz__GotoPreset**`
6. WHEN implementing PTZ service parsers THEN the system SHALL provide `onvif_gsoap_parse_remove_preset()` returning `struct _tptz__RemovePreset**`
7. WHEN any PTZ service parser is called THEN it SHALL use gSOAP's generated `soap_read_*` functions exclusively
8. IF parsing fails THEN the function SHALL return an error code and set the output pointer to NULL

### Requirement 5: Implement Full Request Parsers for Device and Imaging Services

**User Story:** As a developer, I want complete request parsing functions for Device and Imaging service operations, so that all ONVIF services use consistent structured parsing.

#### Acceptance Criteria

1. WHEN implementing Device service parsers THEN the system SHALL provide `onvif_gsoap_parse_get_device_information()` returning `struct _tds__GetDeviceInformation**`
2. WHEN implementing Device service parsers THEN the system SHALL provide `onvif_gsoap_parse_get_capabilities()` returning `struct _tds__GetCapabilities**`
3. WHEN implementing Device service parsers THEN the system SHALL provide `onvif_gsoap_parse_get_system_date_time()` returning `struct _tds__GetSystemDateAndTime**`
4. WHEN implementing Device service parsers THEN the system SHALL provide `onvif_gsoap_parse_system_reboot()` returning `struct _tds__SystemReboot**`
5. WHEN implementing Imaging service parsers THEN the system SHALL provide `onvif_gsoap_parse_get_imaging_settings()` returning `struct _timg__GetImagingSettings**`
6. WHEN implementing Imaging service parsers THEN the system SHALL provide `onvif_gsoap_parse_set_imaging_settings()` returning `struct _timg__SetImagingSettings**`
7. WHEN any Device or Imaging service parser is called THEN it SHALL use gSOAP's generated `soap_read_*` functions exclusively
8. IF parsing fails THEN the function SHALL return an error code and set the output pointer to NULL

### Requirement 6: Update Service Layer to Use Structured Request Data

**User Story:** As a developer, I want all service implementation files updated to extract data from parsed request structures, so that the codebase consistently uses type-safe structured access instead of token strings.

#### Acceptance Criteria

1. WHEN updating service dispatchers THEN `service_dispatcher.c` SHALL call new parsing functions and pass structured requests to service handlers
2. WHEN updating Media service THEN `onvif_media.c` SHALL extract profile tokens, stream setup, and configuration data from request structures
3. WHEN updating PTZ service THEN `onvif_ptz.c` SHALL extract profile tokens, positions, speeds, and preset data from request structures
4. WHEN updating Imaging service THEN `onvif_imaging.c` SHALL extract video source tokens and imaging settings from request structures
5. WHEN any service handler accesses request data THEN it SHALL use structured field access (e.g., `request->ProfileToken`) instead of separate token buffers
6. IF a required field is NULL in the parsed structure THEN the service handler SHALL return an appropriate error code

### Requirement 7: Comprehensive Unit Test Coverage with Real SOAP Envelopes

**User Story:** As a developer, I want comprehensive unit tests using complete ONVIF SOAP envelopes, so that I can verify parsing functions work correctly with realistic requests.

#### Acceptance Criteria

1. WHEN creating test data THEN the system SHALL provide complete SOAP 1.2 envelope templates with proper ONVIF namespaces
2. WHEN creating test data THEN valid request templates SHALL be provided for all 18 parsing functions (6 Media, 6 PTZ, 4 Device, 2 Imaging)
3. WHEN creating test data THEN invalid/malformed request templates SHALL be provided for error testing
4. WHEN testing each parsing function THEN tests SHALL verify NULL parameter handling, successful parsing, field validation, and error conditions
5. WHEN testing Media service parsing THEN tests SHALL validate ProfileToken, StreamSetup, and Protocol fields are correctly extracted
6. WHEN testing PTZ service parsing THEN tests SHALL validate ProfileToken, Position coordinates, and PresetToken fields are correctly extracted
7. WHEN testing error cases THEN tests SHALL verify proper handling of invalid XML, unknown namespaces, and missing required parameters
8. WHEN running all tests THEN they SHALL pass without segmentation faults or memory leaks

### Requirement 8: Remove Obsolete Test Functions

**User Story:** As a developer, I want all test functions for removed manual parsing functions deleted, so that the test suite only covers the new API and doesn't maintain obsolete code.

#### Acceptance Criteria

1. WHEN refactoring tests THEN `test_unit_onvif_gsoap_parse_profile_token` SHALL be deleted from `test_onvif_gsoap.c`
2. WHEN refactoring tests THEN `test_unit_onvif_gsoap_parse_configuration_token` SHALL be deleted
3. WHEN refactoring tests THEN `test_unit_onvif_gsoap_parse_protocol` SHALL be deleted
4. WHEN refactoring tests THEN `test_unit_onvif_gsoap_parse_value` SHALL be deleted
5. WHEN refactoring tests THEN `test_unit_onvif_gsoap_parse_boolean` SHALL be deleted
6. WHEN refactoring tests THEN `test_unit_onvif_gsoap_parse_integer` SHALL be deleted
7. WHEN obsolete tests are removed THEN corresponding entries in test registration SHALL be removed from `main()`

### Requirement 9: Improve gSOAP Context Structure

**User Story:** As a developer, I want an improved gSOAP context structure with embedded soap context, detailed error tracking, and state management, so that the implementation is more efficient and provides better debugging capabilities.

#### Acceptance Criteria

1. WHEN refactoring the context structure THEN it SHALL use embedded `struct soap` instead of pointer indirection
2. WHEN refactoring the context structure THEN it SHALL include request parsing state tracking (operation name, timing, initialization status)
3. WHEN refactoring the context structure THEN it SHALL include response generation state tracking (finalization status, timing)
4. WHEN refactoring the context structure THEN it SHALL include enhanced error context (error code, message, location, soap error code)
5. WHEN initializing context THEN `soap_init()` SHALL be used instead of `soap_new()` (no dynamic allocation)
6. WHEN cleaning up context THEN `soap_done()` SHALL be used instead of `soap_free()` (no deallocation)
7. WHEN an error occurs THEN the system SHALL store error code, message, function location, and gSOAP error code
8. WHEN parsing or generating responses THEN timing metrics SHALL be automatically captured
9. IF response data is requested before finalization THEN the system SHALL detect invalid state and return error
10. IF parsing is attempted on uninitialized context THEN the system SHALL detect invalid state and return error

### Requirement 10: Split Monolithic gSOAP File into Service-Specific Modules

**User Story:** As a developer, I want the large `onvif_gsoap.c` file split into service-specific modules, so that the codebase is more maintainable and follows single responsibility principle.

#### Acceptance Criteria

1. WHEN splitting the gSOAP module THEN the system SHALL create `onvif_gsoap_media.c` containing all Media service parsing functions
2. WHEN splitting the gSOAP module THEN the system SHALL create `onvif_gsoap_ptz.c` containing all PTZ service parsing functions
3. WHEN splitting the gSOAP module THEN the system SHALL create `onvif_gsoap_device.c` containing all Device service parsing functions
4. WHEN splitting the gSOAP module THEN the system SHALL create `onvif_gsoap_imaging.c` containing all Imaging service parsing functions
5. WHEN splitting the gSOAP module THEN `onvif_gsoap_core.c` SHALL contain shared initialization, context management, and utility functions
6. WHEN splitting the gSOAP module THEN corresponding header files SHALL be created for each module (`onvif_gsoap_media.h`, `onvif_gsoap_ptz.h`, etc.)
7. WHEN splitting the gSOAP module THEN the main `onvif_gsoap.h` SHALL include all service-specific headers for backward compatibility
8. WHEN splitting modules THEN the Makefile SHALL be updated to compile all new source files
9. WHEN splitting modules THEN each file SHALL contain proper Doxygen file header documentation
10. IF any module depends on shared functionality THEN it SHALL include `onvif_gsoap_core.h` only

### Requirement 11: Integration Testing for Request-Response Cycles

**User Story:** As a developer, I want integration tests that verify complete request parsing → processing → response generation cycles, so that I can ensure the entire ONVIF operation flow works correctly.

#### Acceptance Criteria

1. WHEN creating integration tests THEN the system SHALL provide tests for complete Media service operations (parse GetStreamUri → generate response)
2. WHEN creating integration tests THEN the system SHALL provide tests for complete PTZ service operations (parse AbsoluteMove → generate response)
3. WHEN running integration tests THEN they SHALL verify parsed request data is correctly used in response generation
4. WHEN running integration tests THEN they SHALL verify response XML is valid and contains expected data
5. IF any integration test fails THEN it SHALL provide clear diagnostics about which phase failed (parsing, processing, or response generation)

## Non-Functional Requirements

### Code Architecture and Modularity

- **Single Responsibility Principle**: Protocol layer handles only SOAP parsing/serialization; service layer handles business logic
- **Modular Design**: Each parsing function is independent and follows consistent patterns
- **Dependency Management**: Protocol layer depends only on gSOAP generated code and core utilities
- **Clear Interfaces**: All parsing functions follow consistent signature: `int parse_operation(ctx, struct **out)`

### Performance

- **Memory Efficiency**: Single-pass parsing using gSOAP's streaming parser
- **Reduced Overhead**: Eliminate redundant string operations and buffer copies from manual parsing
- **Stack Safety**: All large structures allocated on heap via gSOAP's managed memory
- **Parsing Speed**: Complete request parsing SHALL complete in < 1ms on target hardware (Anyka AK3918)

### Security

- **Input Validation**: All parsing relies on gSOAP's WSDL-validated deserialization
- **Buffer Safety**: No manual string manipulation eliminates buffer overflow risks
- **Type Safety**: Structured access prevents type confusion vulnerabilities
- **Error Handling**: All functions return error codes; no silent failures

### Reliability

- **Memory Management**: gSOAP's managed memory ensures proper cleanup
- **Error Propagation**: Clear error codes at all layers
- **NULL Safety**: All pointers validated before dereferencing
- **Test Coverage**: 100% coverage of new parsing functions with both success and failure cases

### Maintainability

- **Code Reduction**: Replace ~800 lines of manual parsing with ~400 lines of structured parsing
- **Modular Organization**: Split monolithic file into 5 service-specific modules, each < 300 lines
- **Doxygen Documentation**: Complete documentation for all new functions following project standards
- **Consistent Patterns**: All parsing functions follow identical structure
- **Service Isolation**: Changes to one service's parsing logic don't affect other services
- **ONVIF Compliance**: Direct mapping to ONVIF 2.5 specification makes future updates straightforward

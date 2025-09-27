# Tasks Document

## Phase 1: Directory Structure Setup

- [ ] 1.1. Create tests subdirectories
  - File: integration-tests/tests/unit/__init__.py
  - Create integration-tests/tests/unit/ directory with __init__.py
  - Purpose: Set up unit tests directory structure
  - _Leverage: Existing integration-tests/ directory_
  - _Requirements: 1.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in project structure | Task: Create integration-tests/tests/unit/ directory and add __init__.py file following requirement 1.1 | Restrictions: Only create this one directory, do not modify existing files | _Leverage: Current integration-tests/ directory structure_ | _Requirements: 1.1 - Test framework restructuring_ | Success: Directory integration-tests/tests/unit/ exists with __init__.py file | Instructions: Mark task in-progress [-], create directory and __init__.py, mark complete [x]_

- [ ] 1.2. Create integration tests subdirectory
  - File: integration-tests/tests/integration/__init__.py
  - Create integration-tests/tests/integration/ directory with __init__.py
  - Purpose: Set up integration tests directory structure
  - _Leverage: Existing integration-tests/ directory_
  - _Requirements: 1.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in project structure | Task: Create integration-tests/tests/integration/ directory and add __init__.py file following requirement 1.1 | Restrictions: Only create this one directory, do not modify existing files | _Leverage: Current integration-tests/ directory structure_ | _Requirements: 1.1 - Test framework restructuring_ | Success: Directory integration-tests/tests/integration/ exists with __init__.py file | Instructions: Mark task in-progress [-], create directory and __init__.py, mark complete [x]_

- [ ] 1.3. Create performance tests subdirectory
  - File: integration-tests/tests/performance/__init__.py
  - Create integration-tests/tests/performance/ directory with __init__.py
  - Purpose: Set up performance tests directory structure
  - _Leverage: Existing integration-tests/ directory_
  - _Requirements: 1.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in project structure | Task: Create integration-tests/tests/performance/ directory and add __init__.py file following requirement 1.1 | Restrictions: Only create this one directory, do not modify existing files | _Leverage: Current integration-tests/ directory structure_ | _Requirements: 1.1 - Test framework restructuring_ | Success: Directory integration-tests/tests/performance/ exists with __init__.py file | Instructions: Mark task in-progress [-], create directory and __init__.py, mark complete [x]_

- [ ] 1.4. Create compliance tests subdirectory
  - File: integration-tests/tests/compliance/__init__.py
  - Create integration-tests/tests/compliance/ directory with __init__.py
  - Purpose: Set up compliance tests directory structure
  - _Leverage: Existing integration-tests/ directory_
  - _Requirements: 1.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in project structure | Task: Create integration-tests/tests/compliance/ directory and add __init__.py file following requirement 1.1 | Restrictions: Only create this one directory, do not modify existing files | _Leverage: Current integration-tests/ directory structure_ | _Requirements: 1.1 - Test framework restructuring_ | Success: Directory integration-tests/tests/compliance/ exists with __init__.py file | Instructions: Mark task in-progress [-], create directory and __init__.py, mark complete [x]_

- [ ] 1.5. Create fixtures directory
  - File: integration-tests/fixtures/__init__.py
  - Create integration-tests/fixtures/ directory with __init__.py
  - Purpose: Set up centralized fixtures directory
  - _Leverage: Existing integration-tests/ directory_
  - _Requirements: 1.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in pytest fixtures | Task: Create integration-tests/fixtures/ directory and add __init__.py file following requirement 1.1 | Restrictions: Only create this one directory, do not modify existing files | _Leverage: Current integration-tests/ directory structure_ | _Requirements: 1.1 - Test framework restructuring_ | Success: Directory integration-tests/fixtures/ exists with __init__.py file | Instructions: Mark task in-progress [-], create directory and __init__.py, mark complete [x]_

- [ ] 1.6. Create utils directory
  - File: integration-tests/utils/__init__.py
  - Create integration-tests/utils/ directory with __init__.py
  - Purpose: Set up utilities directory for helper modules
  - _Leverage: Existing integration-tests/ directory_
  - _Requirements: 1.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in utility modules | Task: Create integration-tests/utils/ directory and add __init__.py file following requirement 1.1 | Restrictions: Only create this one directory, do not modify existing files | _Leverage: Current integration-tests/ directory structure_ | _Requirements: 1.1 - Test framework restructuring_ | Success: Directory integration-tests/utils/ exists with __init__.py file | Instructions: Mark task in-progress [-], create directory and __init__.py, mark complete [x]_

- [ ] 1.7. Create config directory
  - File: integration-tests/config/__init__.py
  - Create integration-tests/config/ directory with __init__.py
  - Purpose: Set up configuration directory for test settings
  - _Leverage: Existing integration-tests/ directory_
  - _Requirements: 1.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in configuration management | Task: Create integration-tests/config/ directory and add __init__.py file following requirement 1.1 | Restrictions: Only create this one directory, do not modify existing files | _Leverage: Current integration-tests/ directory structure_ | _Requirements: 1.1 - Test framework restructuring_ | Success: Directory integration-tests/config/ exists with __init__.py file | Instructions: Mark task in-progress [-], create directory and __init__.py, mark complete [x]_

## Phase 2: Fixture Consolidation

- [ ] 2.1. Analyze existing fixture duplication
  - File: analysis/fixture_duplication_report.md
  - Compare conftest.py and tests/fixtures.py to identify duplicated code
  - Purpose: Document what fixtures need consolidation
  - _Leverage: Existing conftest.py, tests/fixtures.py_
  - _Requirements: 1.3_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Code Analyst with expertise in identifying code duplication | Task: Analyze conftest.py and tests/fixtures.py to identify duplicated fixture code following requirement 1.3 | Restrictions: Only analyze, do not modify files, create analysis document | _Leverage: integration-tests/conftest.py, integration-tests/tests/fixtures.py_ | _Requirements: 1.3 - Code duplication elimination_ | Success: Report clearly identifies all duplicated fixtures and their locations | Instructions: Mark task in-progress [-], analyze fixture files, create report, mark complete [x]_

- [ ] 2.2. Create device fixtures module
  - File: integration-tests/fixtures/device_fixtures.py
  - Extract device-related fixtures from existing files into dedicated module
  - Purpose: Centralize device connection and configuration fixtures
  - _Leverage: ONVIFDeviceClient from tests/fixtures.py_
  - _Requirements: 5.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in pytest fixtures | Task: Create device_fixtures.py module with device-related fixtures extracted from existing files following requirement 5.1 | Restrictions: Only move device-related fixtures, maintain exact same functionality, use proper pytest fixture scoping | _Leverage: ONVIFDeviceClient patterns from integration-tests/tests/fixtures.py_ | _Requirements: 5.1 - Centralized reusable fixtures_ | Success: device_fixtures.py contains all device-related fixtures with proper scoping | Instructions: Mark task in-progress [-], create module and extract fixtures, mark complete [x]_

- [ ] 2.3. Create SOAP fixtures module
  - File: integration-tests/fixtures/soap_fixtures.py
  - Extract SOAP-related fixtures from existing files into dedicated module
  - Purpose: Centralize SOAP request/response fixtures
  - _Leverage: SOAP utilities from tests/fixtures.py_
  - _Requirements: 5.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in SOAP and XML fixtures | Task: Create soap_fixtures.py module with SOAP-related fixtures extracted from existing files following requirement 5.1 | Restrictions: Only move SOAP-related fixtures, maintain exact same functionality, preserve SOAP namespaces | _Leverage: SOAP request patterns from integration-tests/tests/fixtures.py_ | _Requirements: 5.1 - Centralized reusable fixtures_ | Success: soap_fixtures.py contains all SOAP-related fixtures working correctly | Instructions: Mark task in-progress [-], create module and extract SOAP fixtures, mark complete [x]_

- [ ] 2.4. Create performance fixtures module
  - File: integration-tests/fixtures/performance_fixtures.py
  - Extract performance-related fixtures from existing files into dedicated module
  - Purpose: Centralize performance testing fixtures
  - _Leverage: Performance tracking from tests/fixtures.py_
  - _Requirements: 5.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in performance testing fixtures | Task: Create performance_fixtures.py module with performance-related fixtures extracted from existing files following requirement 5.1 | Restrictions: Only move performance-related fixtures, maintain timing accuracy, preserve measurement functionality | _Leverage: performance_tracker patterns from integration-tests/tests/fixtures.py_ | _Requirements: 5.1 - Centralized reusable fixtures_ | Success: performance_fixtures.py contains all performance fixtures functioning correctly | Instructions: Mark task in-progress [-], create module and extract performance fixtures, mark complete [x]_

- [ ] 2.5. Create mock fixtures module
  - File: integration-tests/fixtures/mock_fixtures.py
  - Create mock fixtures for unit testing with mocked dependencies
  - Purpose: Provide mock objects for isolated unit tests
  - _Leverage: Existing mock patterns_
  - _Requirements: 5.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in unittest.mock and pytest fixtures | Task: Create mock_fixtures.py module with mock objects for unit testing following requirement 5.1 | Restrictions: Create new mock fixtures, do not duplicate existing functionality, ensure proper mock isolation | _Leverage: Python unittest.mock patterns, existing test structure_ | _Requirements: 5.1 - Centralized reusable fixtures_ | Success: mock_fixtures.py provides complete mock fixtures for unit testing | Instructions: Mark task in-progress [-], create mock fixtures module, mark complete [x]_

- [ ] 2.6. Update fixture imports and remove duplication
  - Files: integration-tests/conftest.py, integration-tests/tests/fixtures.py
  - Remove duplicated fixtures and update imports to use new fixture modules
  - Purpose: Complete fixture consolidation by removing duplication
  - _Leverage: New fixture modules from previous tasks_
  - _Requirements: 1.3_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Refactoring Expert with expertise in import management | Task: Remove duplicated fixtures from conftest.py and tests/fixtures.py, update imports to use new fixture modules following requirement 1.3 | Restrictions: Do not break existing test functionality, ensure all imports work correctly, maintain backward compatibility | _Leverage: New fixture modules created in tasks 2.2-2.5_ | _Requirements: 1.3 - Code duplication elimination_ | Success: Zero fixture duplication remains, all imports work, existing tests pass | Instructions: Mark task in-progress [-], remove duplicates and update imports, mark complete [x]_

## Phase 3: ONVIF Compliance Framework

- [ ] 3.1. Create base compliance data structures
  - File: integration-tests/utils/compliance_types.py
  - Define ComplianceResult, ComplianceLevel enums and base validation classes
  - Purpose: Establish foundation data structures for compliance validation
  - _Leverage: Python dataclasses and enum patterns_
  - _Requirements: 3.4_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Data Structure Engineer with expertise in dataclasses and type systems | Task: Create compliance_types.py with ComplianceResult dataclass and ComplianceLevel enum following requirement 3.4 | Restrictions: Only create data structures, do not implement validation logic yet, use proper type hints | _Leverage: Python dataclasses, enum, typing modules_ | _Requirements: 3.4 - Detailed compliance reporting structure_ | Success: ComplianceResult and ComplianceLevel properly defined with type hints | Instructions: Mark task in-progress [-], create compliance data structures, mark complete [x]_

- [ ] 3.2. Create SOAP validation utilities
  - File: integration-tests/utils/soap_validator.py
  - Implement SOAP envelope and response format validation utilities
  - Purpose: Validate SOAP format compliance for ONVIF requests/responses
  - _Leverage: Existing XML parsing from tests/fixtures.py_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: XML/SOAP Validation Expert with expertise in SOAP envelope validation | Task: Create soap_validator.py with SOAP format validation utilities following requirement 3.1 | Restrictions: Only validate SOAP format, do not implement ONVIF-specific validation yet, ensure proper XML namespace handling | _Leverage: XML parsing patterns from integration-tests/tests/fixtures.py, ONVIF namespace definitions_ | _Requirements: 3.1 - SOAP envelope and format validation_ | Success: SOAP validator correctly validates envelope structure and namespaces | Instructions: Mark task in-progress [-], create SOAP validation utilities, mark complete [x]_

- [ ] 3.3. Create Profile S validator foundation
  - File: integration-tests/utils/profile_s_validator.py
  - Create base ONVIFProfileSValidator class with basic structure
  - Purpose: Establish foundation for Profile S compliance validation
  - _Leverage: SOAP validator from task 3.2_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Profile S Specialist with expertise in ONVIF compliance requirements | Task: Create profile_s_validator.py with base ONVIFProfileSValidator class following requirement 3.1 | Restrictions: Only create class structure and basic methods, do not implement full validation logic yet, ensure proper inheritance | _Leverage: soap_validator.py from task 3.2, ComplianceResult from task 3.1_ | _Requirements: 3.1 - Profile S compliance validation foundation_ | Success: ONVIFProfileSValidator class exists with basic structure and inheritance | Instructions: Mark task in-progress [-], create Profile S validator foundation, mark complete [x]_

- [ ] 3.4. Implement Profile S user authentication validation
  - File: integration-tests/utils/profile_s_validator.py (extend)
  - Add validate_user_authentication method to Profile S validator
  - Purpose: Validate ONVIF Profile S Section 7.1 user authentication requirement
  - _Leverage: Existing device client authentication patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Security Testing Engineer with expertise in authentication validation | Task: Implement validate_user_authentication method in Profile S validator following requirement 3.1 | Restrictions: Only implement user authentication validation, test both valid and invalid credentials, return ComplianceResult | _Leverage: Device client authentication from existing fixtures_ | _Requirements: 3.1 - Profile S mandatory user authentication_ | Success: Authentication validation tests both positive and negative scenarios correctly | Instructions: Mark task in-progress [-], implement authentication validation, mark complete [x]_

- [ ] 3.5. Implement Profile S capabilities validation
  - File: integration-tests/utils/profile_s_validator.py (extend)
  - Add validate_capabilities method to Profile S validator
  - Purpose: Validate ONVIF Profile S Section 7.2 capabilities requirement
  - _Leverage: Existing capabilities request patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Capabilities Expert with expertise in device capability validation | Task: Implement validate_capabilities method in Profile S validator following requirement 3.1 | Restrictions: Only implement capabilities validation, ensure required categories present, return ComplianceResult | _Leverage: Existing GetCapabilities patterns from device tests_ | _Requirements: 3.1 - Profile S mandatory capabilities_ | Success: Capabilities validation checks all required categories and XAddr fields | Instructions: Mark task in-progress [-], implement capabilities validation, mark complete [x]_

- [ ] 3.6. Implement Profile S discovery validation
  - File: integration-tests/utils/profile_s_validator.py (extend)
  - Add validate_discovery method for WS-Discovery compliance
  - Purpose: Validate ONVIF Profile S Section 7.3 discovery requirement
  - _Leverage: Existing WS-Discovery patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: WS-Discovery Expert with expertise in device discovery protocols | Task: Implement validate_discovery method in Profile S validator following requirement 3.1 | Restrictions: Only implement discovery validation, test device probe/response cycle, validate proper device types and scopes | _Leverage: Existing WS-Discovery patterns from test suite_ | _Requirements: 3.1 - Profile S mandatory discovery_ | Success: Discovery validation tests WS-Discovery probe/response correctly with proper device identification | Instructions: Mark task in-progress [-], implement discovery validation, mark complete [x]_

- [ ] 3.7. Implement Profile S network configuration validation
  - File: integration-tests/utils/profile_s_validator.py (extend)
  - Add validate_network_configuration method for network settings compliance
  - Purpose: Validate ONVIF Profile S Section 7.4 network configuration requirement
  - _Leverage: Device service network operations_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Network Configuration Expert with expertise in IP network validation | Task: Implement validate_network_configuration method in Profile S validator following requirement 3.1 | Restrictions: Only implement network configuration validation, test GetNetworkInterfaces operation, validate interface settings | _Leverage: Device service network operations, SOAP helpers_ | _Requirements: 3.1 - Profile S mandatory network configuration_ | Success: Network configuration validation checks interface settings and network parameters | Instructions: Mark task in-progress [-], implement network validation, mark complete [x]_

- [ ] 3.8. Implement Profile S system datetime validation
  - File: integration-tests/utils/profile_s_validator.py (extend)
  - Add validate_system_datetime method for datetime compliance
  - Purpose: Validate ONVIF Profile S Section 7.5 system datetime requirement
  - _Leverage: GetSystemDateAndTime operation patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: System Time Expert with expertise in datetime synchronization validation | Task: Implement validate_system_datetime method in Profile S validator following requirement 3.1 | Restrictions: Only implement datetime validation, test GetSystemDateAndTime operation, validate timezone and NTP settings | _Leverage: GetSystemDateAndTime operation from device tests_ | _Requirements: 3.1 - Profile S mandatory system datetime_ | Success: Datetime validation checks system time, timezone, and NTP configuration correctly | Instructions: Mark task in-progress [-], implement datetime validation, mark complete [x]_

- [ ] 3.9. Implement Profile S device information validation
  - File: integration-tests/utils/profile_s_validator.py (extend)
  - Add validate_device_information method for device info compliance
  - Purpose: Validate ONVIF Profile S Section 7.6 device information requirement
  - _Leverage: GetDeviceInformation operation patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Device Information Expert with expertise in ONVIF device metadata validation | Task: Implement validate_device_information method in Profile S validator following requirement 3.1 | Restrictions: Only implement device information validation, verify required fields (Manufacturer, Model, etc.), validate field formats | _Leverage: GetDeviceInformation operation from device tests_ | _Requirements: 3.1 - Profile S mandatory device information_ | Success: Device information validation checks all required fields and proper formats | Instructions: Mark task in-progress [-], implement device info validation, mark complete [x]_

- [ ] 3.10. Implement Profile S scopes validation
  - File: integration-tests/utils/profile_s_validator.py (extend)
  - Add validate_scopes method for device scope compliance
  - Purpose: Validate ONVIF Profile S Section 7.7 scopes requirement
  - _Leverage: GetScopes operation patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Scopes Expert with expertise in device scope configuration | Task: Implement validate_scopes method in Profile S validator following requirement 3.1 | Restrictions: Only implement scopes validation, test GetScopes operation, validate scope format and categories | _Leverage: Device service scope operations, SOAP helpers_ | _Requirements: 3.1 - Profile S mandatory scopes_ | Success: Scopes validation checks scope configuration and format compliance | Instructions: Mark task in-progress [-], implement scopes validation, mark complete [x]_

- [ ] 3.11. Implement Profile S services validation
  - File: integration-tests/utils/profile_s_validator.py (extend)
  - Add validate_services method for service endpoint compliance
  - Purpose: Validate ONVIF Profile S Section 7.8 services requirement
  - _Leverage: GetServices operation patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Services Expert with expertise in service endpoint validation | Task: Implement validate_services method in Profile S validator following requirement 3.1 | Restrictions: Only implement services validation, test GetServices operation, validate required service endpoints and versions | _Leverage: GetServices operation from device tests_ | _Requirements: 3.1 - Profile S mandatory services_ | Success: Services validation checks all required service endpoints and proper versions | Instructions: Mark task in-progress [-], implement services validation, mark complete [x]_

- [ ] 3.12. Implement Profile S media profiles validation
  - File: integration-tests/utils/profile_s_validator.py (extend)
  - Add validate_media_profiles method for media profile compliance
  - Purpose: Validate ONVIF Profile S Section 7.9 media profiles requirement
  - _Leverage: Media service patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Media Expert with expertise in media profile validation | Task: Implement validate_media_profiles method in Profile S validator following requirement 3.1 | Restrictions: Only implement media profiles validation, test GetProfiles operation, validate required profile configurations | _Leverage: Media service patterns from existing tests_ | _Requirements: 3.1 - Profile S mandatory media profiles_ | Success: Media profiles validation checks profile configuration and capabilities | Instructions: Mark task in-progress [-], implement media profiles validation, mark complete [x]_

- [ ] 3.13. Implement Profile S streaming validation
  - File: integration-tests/utils/profile_s_validator.py (extend)
  - Add validate_streaming method for streaming URI compliance
  - Purpose: Validate ONVIF Profile S Section 7.10 streaming requirement
  - _Leverage: GetStreamUri operation patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Streaming Expert with expertise in video streaming validation | Task: Implement validate_streaming method in Profile S validator following requirement 3.1 | Restrictions: Only implement streaming validation, test GetStreamUri operation, validate URI format and accessibility | _Leverage: GetStreamUri operation patterns from media tests_ | _Requirements: 3.1 - Profile S mandatory streaming_ | Success: Streaming validation checks URI generation and stream accessibility | Instructions: Mark task in-progress [-], implement streaming validation, mark complete [x]_

- [ ] 3.14. Implement Profile S video encoder configuration validation
  - File: integration-tests/utils/profile_s_validator.py (extend)
  - Add validate_video_encoder_configuration method for encoder compliance
  - Purpose: Validate ONVIF Profile S Section 7.11 video encoder requirement
  - _Leverage: Video encoder configuration patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Video Encoder Expert with expertise in H.264 configuration validation | Task: Implement validate_video_encoder_configuration method in Profile S validator following requirement 3.1 | Restrictions: Only implement video encoder validation, test GetVideoEncoderConfiguration operations, validate encoding parameters | _Leverage: Video encoder patterns from media service tests_ | _Requirements: 3.1 - Profile S mandatory video encoder configuration_ | Success: Video encoder validation checks H.264 configuration and parameters | Instructions: Mark task in-progress [-], implement video encoder validation, mark complete [x]_

- [ ] 3.15. Implement Profile S video source configuration validation
  - File: integration-tests/utils/profile_s_validator.py (extend)
  - Add validate_video_source_configuration method for video source compliance
  - Purpose: Validate ONVIF Profile S Section 7.12 video source requirement
  - _Leverage: Video source configuration patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Video Source Expert with expertise in video input validation | Task: Implement validate_video_source_configuration method in Profile S validator following requirement 3.1 | Restrictions: Only implement video source validation, test GetVideoSourceConfiguration operations, validate source parameters | _Leverage: Video source patterns from media service tests_ | _Requirements: 3.1 - Profile S mandatory video source configuration_ | Success: Video source validation checks source configuration and capabilities | Instructions: Mark task in-progress [-], implement video source validation, mark complete [x]_

- [ ] 3.16. Implement Profile S relay outputs validation
  - File: integration-tests/utils/profile_s_validator.py (extend)
  - Add validate_relay_outputs method for relay output compliance
  - Purpose: Validate ONVIF Profile S Section 7.13 relay outputs requirement
  - _Leverage: Device I/O patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Device I/O Expert with expertise in relay output validation | Task: Implement validate_relay_outputs method in Profile S validator following requirement 3.1 | Restrictions: Only implement relay outputs validation, test GetRelayOutputs operation if supported, mark as conditional if not applicable | _Leverage: Device I/O patterns, conditional feature handling_ | _Requirements: 3.1 - Profile S conditional relay outputs_ | Success: Relay outputs validation properly identifies applicability and validates if present | Instructions: Mark task in-progress [-], implement relay outputs validation, mark complete [x]_

- [ ] 3.17. Create comprehensive Profile S compliance test
  - File: integration-tests/tests/compliance/test_profile_s_complete.py
  - Create complete Profile S compliance test using all validators
  - Purpose: Test all 13 Profile S mandatory features comprehensively
  - _Leverage: Complete Profile S validator from previous tasks_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Compliance Test Engineer with expertise in comprehensive validation | Task: Create test_profile_s_complete.py with all 13 Profile S feature tests following requirement 3.1 | Restrictions: Test all mandatory features, generate detailed compliance report, use proper pytest markers for compliance testing | _Leverage: Complete ONVIFProfileSValidator from tasks 3.3-3.16, device fixtures_ | _Requirements: 3.1 - Complete Profile S compliance testing_ | Success: All 13 Profile S features tested with detailed compliance reporting and clear pass/fail indicators | Instructions: Mark task in-progress [-], create comprehensive compliance tests, mark complete [x]_

- [ ] 3.18. Create Profile T validator foundation
  - File: integration-tests/utils/profile_t_validator.py
  - Create base ONVIFProfileTValidator class with basic structure
  - Purpose: Establish foundation for Profile T compliance validation
  - _Leverage: Profile S validator patterns from task 3.3_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Profile T Specialist with expertise in ONVIF Profile T compliance requirements | Task: Create profile_t_validator.py with base ONVIFProfileTValidator class following requirement 3.2 | Restrictions: Only create class structure and basic methods, do not implement full validation logic yet, ensure proper inheritance from base validator patterns | _Leverage: Profile S validator patterns from task 3.3, ComplianceResult from task 3.1_ | _Requirements: 3.2 - Profile T compliance validation foundation_ | Success: ONVIFProfileTValidator class exists with basic structure and inheritance | Instructions: Mark task in-progress [-], create Profile T validator foundation, mark complete [x]_

- [ ] 3.19. Implement Profile T enhanced video streaming validation
  - File: integration-tests/utils/profile_t_validator.py (extend)
  - Add validate_enhanced_video_streaming method for Profile T streaming requirements
  - Purpose: Validate ONVIF Profile T enhanced video streaming capabilities
  - _Leverage: Media service streaming patterns_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Enhanced Video Streaming Expert with expertise in Profile T streaming requirements | Task: Implement validate_enhanced_video_streaming method in Profile T validator following requirement 3.2 | Restrictions: Only implement enhanced streaming validation, test advanced streaming capabilities, validate Profile T specific features | _Leverage: Media service streaming patterns, GetStreamUri operations_ | _Requirements: 3.2 - Profile T enhanced video streaming_ | Success: Enhanced video streaming validation checks Profile T specific streaming capabilities | Instructions: Mark task in-progress [-], implement enhanced streaming validation, mark complete [x]_

- [ ] 3.20. Implement Profile T PTZ support validation
  - File: integration-tests/utils/profile_t_validator.py (extend)
  - Add validate_ptz_support method for PTZ compliance
  - Purpose: Validate ONVIF Profile T PTZ control requirements
  - _Leverage: PTZ service patterns from existing tests_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: PTZ Control Expert with expertise in ONVIF PTZ validation | Task: Implement validate_ptz_support method in Profile T validator following requirement 3.2 | Restrictions: Only implement PTZ validation, test PTZ capabilities and operations, handle conditional PTZ support properly | _Leverage: PTZ service patterns from existing integration tests_ | _Requirements: 3.2 - Profile T PTZ support_ | Success: PTZ validation checks PTZ capabilities and control operations correctly | Instructions: Mark task in-progress [-], implement PTZ validation, mark complete [x]_

- [ ] 3.21. Implement Profile T imaging service validation
  - File: integration-tests/utils/profile_t_validator.py (extend)
  - Add validate_imaging_service method for imaging compliance
  - Purpose: Validate ONVIF Profile T imaging service requirements
  - _Leverage: Imaging service patterns from existing tests_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Imaging Service Expert with expertise in ONVIF imaging validation | Task: Implement validate_imaging_service method in Profile T validator following requirement 3.2 | Restrictions: Only implement imaging service validation, test imaging capabilities and settings, validate image quality controls | _Leverage: Imaging service patterns from existing integration tests_ | _Requirements: 3.2 - Profile T imaging service_ | Success: Imaging service validation checks imaging capabilities and controls correctly | Instructions: Mark task in-progress [-], implement imaging validation, mark complete [x]_

- [ ] 3.22. Create basic Profile T compliance test
  - File: integration-tests/tests/compliance/test_profile_t_basic.py
  - Create basic Profile T compliance test for core features
  - Purpose: Test core Profile T features (streaming, PTZ, imaging)
  - _Leverage: Profile T validator from previous tasks_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Profile T Test Engineer with expertise in Profile T testing | Task: Create test_profile_t_basic.py with basic Profile T compliance tests following requirement 3.2 | Restrictions: Only test core Profile T features, use proper pytest markers, ensure clear test reporting with conditional feature handling | _Leverage: ONVIFProfileTValidator from previous tasks, device fixtures_ | _Requirements: 3.2 - Profile T compliance testing_ | Success: Profile T basic compliance tests validate core features with proper conditional handling | Instructions: Mark task in-progress [-], create basic Profile T tests, mark complete [x]_

- [ ] 3.23. Create compliance reporting system
  - File: integration-tests/utils/compliance_reporting.py
  - Implement comprehensive compliance report generation
  - Purpose: Generate HTML and JSON compliance reports
  - _Leverage: ComplianceResult data structures_
  - _Requirements: 3.4_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Compliance Reporting Engineer with expertise in test reporting and HTML generation | Task: Create compliance_reporting.py with HTML and JSON report generation following requirement 3.4 | Restrictions: Generate detailed compliance reports, include feature-by-feature status, provide clear pass/fail indicators and recommendations | _Leverage: ComplianceResult from task 3.1, HTML template generation, JSON serialization_ | _Requirements: 3.4 - Detailed compliance reporting_ | Success: Compliance reports generate clear HTML and JSON output with comprehensive feature analysis | Instructions: Mark task in-progress [-], create compliance reporting system, mark complete [x]_

## Profile S Optional Tests

- [ ] 3.24. Implement Profile S audio streaming validation (optional)
  - File: integration-tests/utils/profile_s_validator.py (extend)
  - Add validate_audio_streaming method for optional audio streaming
  - Purpose: Validate ONVIF Profile S audio streaming if supported
  - _Leverage: Audio streaming patterns from media service_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Audio Streaming Expert with expertise in ONVIF audio validation | Task: Implement validate_audio_streaming method for optional Profile S audio streaming following requirement 3.1 | Restrictions: Mark as conditional/optional feature, test audio capabilities if present, handle graceful failure if not supported | _Leverage: Audio streaming patterns from media service tests_ | _Requirements: 3.1 - Profile S optional audio streaming_ | Success: Audio streaming validation properly identifies support and validates if present | Instructions: Mark task in-progress [-], implement audio streaming validation, mark complete [x]_

- [ ] 3.25. Implement Profile S PTZ preset validation (optional)
  - File: integration-tests/utils/profile_s_validator.py (extend)
  - Add validate_ptz_presets method for optional PTZ preset support
  - Purpose: Validate ONVIF Profile S PTZ presets if supported
  - _Leverage: PTZ service preset patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: PTZ Preset Expert with expertise in PTZ preset validation | Task: Implement validate_ptz_presets method for optional Profile S PTZ presets following requirement 3.1 | Restrictions: Mark as conditional/optional feature, test preset operations if PTZ supported, handle non-PTZ devices gracefully | _Leverage: PTZ service patterns from existing tests_ | _Requirements: 3.1 - Profile S optional PTZ presets_ | Success: PTZ preset validation properly identifies PTZ support and validates presets if available | Instructions: Mark task in-progress [-], implement PTZ preset validation, mark complete [x]_

- [ ] 3.26. Implement Profile S event handling validation (optional)
  - File: integration-tests/utils/profile_s_validator.py (extend)
  - Add validate_event_handling method for optional event capabilities
  - Purpose: Validate ONVIF Profile S event handling if supported
  - _Leverage: Event service patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Event Handling Expert with expertise in ONVIF event validation | Task: Implement validate_event_handling method for optional Profile S event handling following requirement 3.1 | Restrictions: Mark as conditional/optional feature, test event capabilities if present, validate subscription mechanisms | _Leverage: Event service patterns, subscription mechanisms_ | _Requirements: 3.1 - Profile S optional event handling_ | Success: Event handling validation properly identifies support and validates mechanisms if present | Instructions: Mark task in-progress [-], implement event handling validation, mark complete [x]_

- [ ] 3.27. Implement Profile S recording control validation (optional)
  - File: integration-tests/utils/profile_s_validator.py (extend)
  - Add validate_recording_control method for optional recording features
  - Purpose: Validate ONVIF Profile S recording control if supported
  - _Leverage: Recording service patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Recording Control Expert with expertise in ONVIF recording validation | Task: Implement validate_recording_control method for optional Profile S recording control following requirement 3.1 | Restrictions: Mark as conditional/optional feature, test recording capabilities if present, validate control operations | _Leverage: Recording service patterns, media control operations_ | _Requirements: 3.1 - Profile S optional recording control_ | Success: Recording control validation properly identifies support and validates operations if present | Instructions: Mark task in-progress [-], implement recording validation, mark complete [x]_

## Complete Profile T Mandatory Tests

- [ ] 3.28. Implement Profile T absolute PTZ movement validation
  - File: integration-tests/utils/profile_t_validator.py (extend)
  - Add validate_absolute_ptz_movement method for absolute PTZ positioning
  - Purpose: Validate ONVIF Profile T mandatory absolute PTZ movement
  - _Leverage: PTZ service absolute movement patterns_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: PTZ Absolute Movement Expert with expertise in precise PTZ positioning | Task: Implement validate_absolute_ptz_movement method for Profile T absolute PTZ following requirement 3.2 | Restrictions: Test absolute positioning operations, validate coordinate systems, ensure precision requirements | _Leverage: PTZ service absolute movement from existing tests_ | _Requirements: 3.2 - Profile T mandatory absolute PTZ movement_ | Success: Absolute PTZ movement validation checks positioning accuracy and coordinate systems | Instructions: Mark task in-progress [-], implement absolute PTZ validation, mark complete [x]_

- [ ] 3.29. Implement Profile T relative PTZ movement validation
  - File: integration-tests/utils/profile_t_validator.py (extend)
  - Add validate_relative_ptz_movement method for relative PTZ positioning
  - Purpose: Validate ONVIF Profile T mandatory relative PTZ movement
  - _Leverage: PTZ service relative movement patterns_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: PTZ Relative Movement Expert with expertise in incremental PTZ positioning | Task: Implement validate_relative_ptz_movement method for Profile T relative PTZ following requirement 3.2 | Restrictions: Test relative positioning operations, validate movement increments, ensure proper vector handling | _Leverage: PTZ service relative movement from existing tests_ | _Requirements: 3.2 - Profile T mandatory relative PTZ movement_ | Success: Relative PTZ movement validation checks incremental positioning and vector operations | Instructions: Mark task in-progress [-], implement relative PTZ validation, mark complete [x]_

- [ ] 3.30. Implement Profile T continuous PTZ movement validation
  - File: integration-tests/utils/profile_t_validator.py (extend)
  - Add validate_continuous_ptz_movement method for continuous PTZ control
  - Purpose: Validate ONVIF Profile T mandatory continuous PTZ movement
  - _Leverage: PTZ service continuous movement patterns_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: PTZ Continuous Movement Expert with expertise in continuous PTZ control | Task: Implement validate_continuous_ptz_movement method for Profile T continuous PTZ following requirement 3.2 | Restrictions: Test continuous movement operations, validate velocity control, ensure proper stop mechanisms | _Leverage: PTZ service continuous movement from existing tests_ | _Requirements: 3.2 - Profile T mandatory continuous PTZ movement_ | Success: Continuous PTZ movement validation checks velocity control and stop operations | Instructions: Mark task in-progress [-], implement continuous PTZ validation, mark complete [x]_

- [ ] 3.31. Implement Profile T PTZ preset operation validation
  - File: integration-tests/utils/profile_t_validator.py (extend)
  - Add validate_ptz_preset_operations method for PTZ preset management
  - Purpose: Validate ONVIF Profile T mandatory PTZ preset operations
  - _Leverage: PTZ service preset operation patterns_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: PTZ Preset Management Expert with expertise in preset operations | Task: Implement validate_ptz_preset_operations method for Profile T PTZ presets following requirement 3.2 | Restrictions: Test create/set/goto/remove preset operations, validate preset storage, ensure proper preset management | _Leverage: PTZ service preset operations from existing tests_ | _Requirements: 3.2 - Profile T mandatory PTZ preset operations_ | Success: PTZ preset operations validation checks complete preset lifecycle management | Instructions: Mark task in-progress [-], implement PTZ preset operations validation, mark complete [x]_

- [ ] 3.32. Implement Profile T PTZ home position validation
  - File: integration-tests/utils/profile_t_validator.py (extend)
  - Add validate_ptz_home_position method for PTZ home position support
  - Purpose: Validate ONVIF Profile T mandatory PTZ home position
  - _Leverage: PTZ service home position patterns_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: PTZ Home Position Expert with expertise in PTZ reference positioning | Task: Implement validate_ptz_home_position method for Profile T PTZ home position following requirement 3.2 | Restrictions: Test home position operations, validate reference positioning, ensure proper home position handling | _Leverage: PTZ service home position from existing tests_ | _Requirements: 3.2 - Profile T mandatory PTZ home position_ | Success: PTZ home position validation checks reference positioning and home operations | Instructions: Mark task in-progress [-], implement PTZ home position validation, mark complete [x]_

- [ ] 3.33. Implement Profile T imaging settings validation
  - File: integration-tests/utils/profile_t_validator.py (extend)
  - Add validate_imaging_settings method for advanced imaging controls
  - Purpose: Validate ONVIF Profile T mandatory imaging settings
  - _Leverage: Imaging service advanced settings patterns_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Advanced Imaging Expert with expertise in imaging parameter validation | Task: Implement validate_imaging_settings method for Profile T imaging settings following requirement 3.2 | Restrictions: Test advanced imaging controls, validate parameter ranges, ensure proper image quality settings | _Leverage: Imaging service advanced patterns from existing tests_ | _Requirements: 3.2 - Profile T mandatory imaging settings_ | Success: Imaging settings validation checks advanced controls and parameter validation | Instructions: Mark task in-progress [-], implement imaging settings validation, mark complete [x]_

- [ ] 3.34. Implement Profile T video analytics validation
  - File: integration-tests/utils/profile_t_validator.py (extend)
  - Add validate_video_analytics method for analytics capabilities
  - Purpose: Validate ONVIF Profile T mandatory video analytics
  - _Leverage: Analytics service patterns_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Video Analytics Expert with expertise in ONVIF analytics validation | Task: Implement validate_video_analytics method for Profile T video analytics following requirement 3.2 | Restrictions: Test analytics capabilities, validate rule configuration, ensure proper analytics output | _Leverage: Analytics service patterns, rule configuration_ | _Requirements: 3.2 - Profile T mandatory video analytics_ | Success: Video analytics validation checks analytics capabilities and rule processing | Instructions: Mark task in-progress [-], implement video analytics validation, mark complete [x]_

- [ ] 3.35. Implement Profile T access control validation
  - File: integration-tests/utils/profile_t_validator.py (extend)
  - Add validate_access_control method for access control features
  - Purpose: Validate ONVIF Profile T mandatory access control
  - _Leverage: Access control service patterns_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Access Control Expert with expertise in ONVIF access control validation | Task: Implement validate_access_control method for Profile T access control following requirement 3.2 | Restrictions: Test access control operations, validate credential management, ensure proper access policies | _Leverage: Access control service patterns, credential management_ | _Requirements: 3.2 - Profile T mandatory access control_ | Success: Access control validation checks credential management and access policies | Instructions: Mark task in-progress [-], implement access control validation, mark complete [x]_

- [ ] 3.36. Implement Profile T door control validation
  - File: integration-tests/utils/profile_t_validator.py (extend)
  - Add validate_door_control method for door control operations
  - Purpose: Validate ONVIF Profile T mandatory door control
  - _Leverage: Door control service patterns_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Door Control Expert with expertise in ONVIF door control validation | Task: Implement validate_door_control method for Profile T door control following requirement 3.2 | Restrictions: Test door control operations, validate door state management, ensure proper access control integration | _Leverage: Door control service patterns, state management_ | _Requirements: 3.2 - Profile T mandatory door control_ | Success: Door control validation checks door operations and state management | Instructions: Mark task in-progress [-], implement door control validation, mark complete [x]_

- [ ] 3.37. Create comprehensive Profile T compliance test
  - File: integration-tests/tests/compliance/test_profile_t_complete.py
  - Create complete Profile T compliance test using all validators
  - Purpose: Test all Profile T mandatory features comprehensively
  - _Leverage: Complete Profile T validator from previous tasks_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Profile T Compliance Test Engineer with expertise in comprehensive Profile T validation | Task: Create test_profile_t_complete.py with all Profile T feature tests following requirement 3.2 | Restrictions: Test all mandatory features, generate detailed compliance report, use proper pytest markers for compliance testing | _Leverage: Complete ONVIFProfileTValidator from tasks 3.18-3.36, device fixtures_ | _Requirements: 3.2 - Complete Profile T compliance testing_ | Success: All Profile T features tested with detailed compliance reporting and clear pass/fail indicators | Instructions: Mark task in-progress [-], create comprehensive Profile T tests, mark complete [x]_

## Profile T Optional Tests

- [ ] 3.38. Implement Profile T audio streaming validation (optional)
  - File: integration-tests/utils/profile_t_validator.py (extend)
  - Add validate_audio_streaming method for optional audio streaming
  - Purpose: Validate ONVIF Profile T audio streaming if supported
  - _Leverage: Audio streaming patterns from media service_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Audio Streaming Expert with expertise in Profile T audio validation | Task: Implement validate_audio_streaming method for optional Profile T audio streaming following requirement 3.2 | Restrictions: Mark as conditional/optional feature, test enhanced audio capabilities if present, handle graceful failure if not supported | _Leverage: Audio streaming patterns from media service tests_ | _Requirements: 3.2 - Profile T optional audio streaming_ | Success: Audio streaming validation properly identifies support and validates enhanced features if present | Instructions: Mark task in-progress [-], implement audio streaming validation, mark complete [x]_

- [ ] 3.39. Implement Profile T two-way audio validation (optional)
  - File: integration-tests/utils/profile_t_validator.py (extend)
  - Add validate_two_way_audio method for optional two-way audio communication
  - Purpose: Validate ONVIF Profile T two-way audio if supported
  - _Leverage: Two-way audio patterns from media service_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Two-Way Audio Expert with expertise in bidirectional audio validation | Task: Implement validate_two_way_audio method for optional Profile T two-way audio following requirement 3.2 | Restrictions: Mark as conditional/optional feature, test bidirectional audio capabilities if present, validate audio input/output | _Leverage: Two-way audio patterns from media service tests_ | _Requirements: 3.2 - Profile T optional two-way audio_ | Success: Two-way audio validation properly identifies support and validates bidirectional audio if present | Instructions: Mark task in-progress [-], implement two-way audio validation, mark complete [x]_

- [ ] 3.40. Implement Profile T edge storage validation (optional)
  - File: integration-tests/utils/profile_t_validator.py (extend)
  - Add validate_edge_storage method for optional edge storage capabilities
  - Purpose: Validate ONVIF Profile T edge storage if supported
  - _Leverage: Edge storage patterns from recording service_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Edge Storage Expert with expertise in local storage validation | Task: Implement validate_edge_storage method for optional Profile T edge storage following requirement 3.2 | Restrictions: Mark as conditional/optional feature, test local storage capabilities if present, validate storage management operations | _Leverage: Edge storage patterns from recording service tests_ | _Requirements: 3.2 - Profile T optional edge storage_ | Success: Edge storage validation properly identifies support and validates storage operations if present | Instructions: Mark task in-progress [-], implement edge storage validation, mark complete [x]_

- [ ] 3.41. Create final compliance integration test
  - File: integration-tests/tests/compliance/test_onvif_full_compliance.py
  - Create comprehensive test combining Profile S and Profile T validation
  - Purpose: Test complete ONVIF compliance across both profiles
  - _Leverage: Both Profile S and Profile T validators_
  - _Requirements: 3.1, 3.2, 3.4_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Full Compliance Engineer with expertise in comprehensive ONVIF validation | Task: Create test_onvif_full_compliance.py with complete Profile S & T compliance testing following requirements 3.1, 3.2, and 3.4 | Restrictions: Test both profiles comprehensively, generate unified compliance report, provide complete ONVIF certification validation | _Leverage: Complete Profile S and Profile T validators from all previous tasks, compliance reporting system_ | _Requirements: 3.1, 3.2, 3.4 - Complete ONVIF compliance testing and reporting_ | Success: Full ONVIF compliance test validates both profiles with comprehensive certification-ready reporting | Instructions: Mark task in-progress [-], create full compliance test, mark complete [x]_

## Phase 4: Performance Testing Framework

- [ ] 4.1. Create performance metrics data structures
  - File: integration-tests/utils/performance_types.py
  - Define PerformanceMetrics dataclass with statistical properties
  - Purpose: Establish foundation for performance data collection and analysis
  - _Leverage: Python dataclasses, statistics module_
  - _Requirements: 4.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Performance Data Engineer with expertise in statistical analysis | Task: Create performance_types.py with PerformanceMetrics dataclass including statistical properties following requirement 4.1 | Restrictions: Only create data structures with properties, do not implement collection logic yet, include mean/median/std dev/p95 calculations | _Leverage: Python dataclasses, statistics module, type hints_ | _Requirements: 4.1 - Statistical analysis with multiple metrics_ | Success: PerformanceMetrics dataclass with statistical properties implemented correctly | Instructions: Mark task in-progress [-], create performance data structures, mark complete [x]_

- [ ] 4.2. Create performance collector class
  - File: integration-tests/utils/performance_collector.py
  - Implement PerformanceCollector for measuring response times and resource usage
  - Purpose: Collect performance metrics during test execution
  - _Leverage: psutil library, time module_
  - _Requirements: 4.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Performance Measurement Engineer with expertise in Python profiling | Task: Create performance_collector.py with PerformanceCollector class for measuring response times and resource usage following requirement 4.1 | Restrictions: Only implement measurement collection, do not add statistical analysis yet, ensure accurate timing measurements | _Leverage: psutil library for CPU/memory, time module for response times_ | _Requirements: 4.1 - Performance metrics collection_ | Success: PerformanceCollector accurately measures response times, CPU and memory usage | Instructions: Mark task in-progress [-], create performance collector, mark complete [x]_

- [ ] 4.3. Create baseline management system
  - File: integration-tests/utils/performance_baselines.py
  - Implement PerformanceBaseline class for saving/loading baseline metrics
  - Purpose: Manage performance baselines for regression detection
  - _Leverage: JSON serialization, datetime module_
  - _Requirements: 4.3_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Performance Baseline Engineer with expertise in regression detection | Task: Create performance_baselines.py with PerformanceBaseline class for baseline management following requirement 4.3 | Restrictions: Only implement baseline save/load/compare functionality, use JSON for persistence, include configurable tolerance thresholds | _Leverage: JSON module, datetime, pathlib for file operations_ | _Requirements: 4.3 - Baseline comparison and regression detection_ | Success: Baseline system saves/loads metrics and detects regressions with configurable tolerance | Instructions: Mark task in-progress [-], create baseline management system, mark complete [x]_

- [ ] 4.4. Create basic performance test for device operations
  - File: integration-tests/tests/performance/test_device_basic_performance.py
  - Implement basic performance test for device operations with statistics
  - Purpose: Test device operation performance with statistical analysis
  - _Leverage: Performance collector and device fixtures_
  - _Requirements: 4.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Performance Test Engineer with expertise in statistical testing | Task: Create test_device_basic_performance.py with basic device operation performance tests following requirement 4.1 | Restrictions: Test only basic device operations (GetDeviceInformation), ensure 20+ iterations for statistical significance, use proper pytest markers | _Leverage: PerformanceCollector from task 4.2, device fixtures from Phase 2_ | _Requirements: 4.1 - Statistical analysis with 20+ iterations_ | Success: Performance test provides statistically significant results with proper metrics | Instructions: Mark task in-progress [-], create basic performance test, mark complete [x]_

- [ ] 4.5. Add concurrent load testing capability
  - File: integration-tests/tests/performance/test_concurrent_load.py
  - Implement concurrent load testing using asyncio or threading
  - Purpose: Test system performance under concurrent load
  - _Leverage: concurrent.futures or asyncio_
  - _Requirements: 4.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Concurrent Testing Engineer with expertise in Python concurrency | Task: Create test_concurrent_load.py with concurrent load testing following requirement 4.2 | Restrictions: Test with 50+ concurrent operations, ensure thread safety, measure success rates and response times under load | _Leverage: concurrent.futures or asyncio, performance collector from task 4.2_ | _Requirements: 4.2 - Concurrent load testing with 50+ operations_ | Success: Concurrent tests validate system stability under realistic load conditions | Instructions: Mark task in-progress [-], create concurrent load tests, mark complete [x]_

- [ ] 4.6. Create performance configuration module
  - File: integration-tests/config/performance_config.py
  - Define performance thresholds and test configuration
  - Purpose: Centralize performance testing configuration and thresholds
  - _Leverage: dataclasses, environment variables_
  - _Requirements: 4.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Configuration Engineer with expertise in test configuration management | Task: Create performance_config.py with performance thresholds and configuration following requirement 4.1 | Restrictions: Define clear thresholds for response times, success rates, resource usage, allow environment variable overrides | _Leverage: dataclasses, os.environ for environment variables_ | _Requirements: 4.1 - Performance thresholds and configuration_ | Success: Performance configuration provides clear thresholds and environment customization | Instructions: Mark task in-progress [-], create performance configuration, mark complete [x]_

## Phase 5: Device Service Test Enhancement

- [ ] 5.1. Create SOAP helper utilities
  - File: integration-tests/utils/soap_helpers.py
  - Implement SOAP request creation and response parsing utilities
  - Purpose: Provide reusable SOAP utilities for device service tests
  - _Leverage: Existing SOAP patterns from tests/fixtures.py_
  - _Requirements: 5.3_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: SOAP Protocol Engineer with expertise in XML and SOAP message handling | Task: Create soap_helpers.py with SOAP request creation and response parsing utilities following requirement 5.3 | Restrictions: Only create utilities, do not implement device-specific logic yet, ensure proper namespace handling | _Leverage: SOAP patterns from integration-tests/tests/fixtures.py, XML parsing utilities_ | _Requirements: 5.3 - SOAP validation for all operations_ | Success: SOAP helpers provide reusable utilities for request/response handling | Instructions: Mark task in-progress [-], create SOAP helper utilities, mark complete [x]_

- [ ] 5.2. Create test data manager for device tests
  - File: integration-tests/utils/test_data_manager.py
  - Implement test data setup and cleanup management
  - Purpose: Manage test data lifecycle for device service tests
  - _Leverage: Python context managers, tempfile module_
  - _Requirements: 5.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Test Data Engineer with expertise in test data management and cleanup | Task: Create test_data_manager.py with test data lifecycle management following requirement 5.1 | Restrictions: Use context managers for cleanup, ensure no test data leakage, provide clean setup/teardown | _Leverage: Python contextlib, tempfile, pathlib modules_ | _Requirements: 5.1 - Test data setup and teardown_ | Success: Test data manager provides clean data lifecycle with proper cleanup | Instructions: Mark task in-progress [-], create test data manager, mark complete [x]_

- [ ] 5.3. Refactor existing device service tests
  - File: integration-tests/tests/integration/test_device_service_basic.py
  - Refactor existing GetDeviceInformation, GetCapabilities tests
  - Purpose: Improve existing device tests with enhanced validation
  - _Leverage: SOAP helpers from task 5.1, existing device tests_
  - _Requirements: 5.3_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Test Refactoring Engineer with expertise in improving existing test quality | Task: Refactor existing device service tests using new SOAP helpers following requirement 5.3 | Restrictions: Only refactor existing tests (GetDeviceInformation, GetCapabilities), maintain same test coverage, enhance validation | _Leverage: SOAP helpers from task 5.1, existing device service tests_ | _Requirements: 5.3 - Enhanced SOAP validation_ | Success: Existing device tests use new helpers and provide better validation | Instructions: Mark task in-progress [-], refactor existing tests, mark complete [x]_

- [ ] 5.4. Add missing SetSystemDateAndTime operation test
  - File: integration-tests/tests/integration/test_device_service_datetime.py
  - Implement test for SetSystemDateAndTime operation
  - Purpose: Add coverage for missing datetime operation
  - _Leverage: SOAP helpers and device fixtures_
  - _Requirements: 5.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Device Operation Engineer with expertise in datetime operations | Task: Create test for SetSystemDateAndTime operation following requirement 5.2 | Restrictions: Only implement this one operation test, test both valid and invalid datetime formats, include error scenarios | _Leverage: SOAP helpers from task 5.1, device fixtures from Phase 2_ | _Requirements: 5.2 - Missing operation coverage_ | Success: SetSystemDateAndTime operation fully tested with positive and negative scenarios | Instructions: Mark task in-progress [-], implement datetime operation test, mark complete [x]_

- [ ] 5.5. Add missing user management operations tests
  - File: integration-tests/tests/integration/test_device_service_users.py
  - Implement tests for CreateUsers and DeleteUsers operations
  - Purpose: Add coverage for missing user management operations
  - _Leverage: SOAP helpers and device fixtures_
  - _Requirements: 5.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF User Management Engineer with expertise in user operation testing | Task: Create tests for CreateUsers and DeleteUsers operations following requirement 5.2 | Restrictions: Only implement user management tests, ensure proper user creation/deletion flow, test error scenarios | _Leverage: SOAP helpers from task 5.1, device fixtures from Phase 2_ | _Requirements: 5.2 - Missing operation coverage_ | Success: User management operations fully tested with complete lifecycle validation | Instructions: Mark task in-progress [-], implement user management tests, mark complete [x]_

- [ ] 5.6. Add comprehensive error handling tests
  - File: integration-tests/tests/integration/test_device_service_errors.py
  - Implement comprehensive error handling and SOAP fault tests
  - Purpose: Validate proper error responses and SOAP fault handling
  - _Leverage: SOAP helpers and error scenarios_
  - _Requirements: 5.4_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Error Handling Test Engineer with expertise in SOAP fault validation | Task: Create comprehensive error handling tests for device service following requirement 5.4 | Restrictions: Test invalid operations, malformed requests, authentication failures, ensure proper SOAP fault responses | _Leverage: SOAP helpers from task 5.1, device fixtures from Phase 2_ | _Requirements: 5.4 - Error handling and fault response testing_ | Success: All error scenarios tested with proper SOAP fault validation | Instructions: Mark task in-progress [-], create error handling tests, mark complete [x]_

## Phase 6: Code Quality Enforcement

- [ ] 6.1. Create development requirements file
  - File: integration-tests/requirements-dev.txt
  - Define development dependencies for code quality tools
  - Purpose: Specify all development dependencies with pinned versions
  - _Leverage: Python package management best practices_
  - _Requirements: 2.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Package Manager with expertise in dependency management | Task: Create requirements-dev.txt with development dependencies following requirement 2.1 | Restrictions: Pin exact versions, organize by purpose (formatting, linting, testing), include all necessary quality tools | _Leverage: Python packaging best practices, quality tool ecosystem_ | _Requirements: 2.1 - Development dependencies specification_ | Success: requirements-dev.txt contains all quality tools with pinned versions | Instructions: Mark task in-progress [-], create development requirements, mark complete [x]_

- [ ] 6.2. Create project configuration file
  - File: integration-tests/pyproject.toml
  - Configure Black, isort, Pylint, mypy, coverage settings
  - Purpose: Centralize all tool configuration in one file
  - _Leverage: Python project configuration standards_
  - _Requirements: 2.1_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Configuration Engineer with expertise in tool configuration | Task: Create pyproject.toml with tool configurations following requirement 2.1 | Restrictions: Configure for 88-char line length, Pylint ≥8.0, 90% coverage, strict type checking, maintain consistency | _Leverage: Tool-specific configuration best practices_ | _Requirements: 2.1 - Tool configuration and standards_ | Success: pyproject.toml provides consistent configuration for all quality tools | Instructions: Mark task in-progress [-], create project configuration, mark complete [x]_

- [ ] 6.3. Create pre-commit hooks configuration
  - File: integration-tests/.pre-commit-config.yaml
  - Set up pre-commit hooks for automated quality enforcement
  - Purpose: Automatically enforce code quality before commits
  - _Leverage: pre-commit framework and hooks ecosystem_
  - _Requirements: 2.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Git Hooks Engineer with expertise in pre-commit automation | Task: Create .pre-commit-config.yaml with automated quality enforcement following requirement 2.2 | Restrictions: Include Black, isort, Pylint, mypy, bandit hooks, ensure hooks work together, set appropriate failure thresholds | _Leverage: pre-commit framework, quality tool integrations_ | _Requirements: 2.2 - Pre-commit hook automation_ | Success: Pre-commit hooks automatically enforce all quality standards before commits | Instructions: Mark task in-progress [-], create pre-commit configuration, mark complete [x]_

- [ ] 6.4. Create quality validation script
  - File: integration-tests/scripts/quality_check.py
  - Implement comprehensive quality validation script
  - Purpose: Run all quality checks with detailed reporting
  - _Leverage: subprocess module, quality tools_
  - _Requirements: 2.5_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Quality Validation Engineer with expertise in automated quality checking | Task: Create quality_check.py with comprehensive quality validation following requirement 2.5 | Restrictions: Run all quality tools, provide clear pass/fail reporting, return proper exit codes, include helpful error messages | _Leverage: subprocess module, pathlib, existing quality tool configuration_ | _Requirements: 2.5 - Quality validation scripts_ | Success: Quality check script runs all tools and provides clear validation results | Instructions: Mark task in-progress [-], create quality validation script, mark complete [x]_

## Phase 7: Final Integration and Documentation

- [ ] 7.1. Create test execution configuration
  - File: integration-tests/config/test_execution.py
  - Define test execution settings and category configurations
  - Purpose: Centralize test execution configuration
  - _Leverage: dataclasses, pytest configuration_
  - _Requirements: 1.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Test Configuration Engineer with expertise in pytest configuration | Task: Create test_execution.py with execution settings following requirement 1.2 | Restrictions: Define clear category markers, execution timeouts, parallel execution settings | _Leverage: dataclasses, pytest marker system_ | _Requirements: 1.2 - Test categorization and selective execution_ | Success: Test execution configuration supports all categories and selective execution | Instructions: Mark task in-progress [-], create execution configuration, mark complete [x]_

- [ ] 7.2. Create comprehensive test runner script
  - File: integration-tests/scripts/run_comprehensive_tests.py
  - Implement enhanced test runner with category selection
  - Purpose: Provide unified test execution with category filtering
  - _Leverage: Existing run_tests.py patterns, pytest execution_
  - _Requirements: 1.2_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Test Execution Engineer with expertise in pytest automation | Task: Create run_comprehensive_tests.py with category-based execution following requirement 1.2 | Restrictions: Maintain compatibility with existing run_tests.py, add category selection, provide clear execution reporting | _Leverage: Existing integration-tests/run_tests.py patterns, pytest marker system_ | _Requirements: 1.2 - Selective test execution by category_ | Success: Test runner supports selective execution by category with clear reporting | Instructions: Mark task in-progress [-], create comprehensive test runner, mark complete [x]_

- [ ] 7.3. Create validation script for framework completeness
  - File: integration-tests/scripts/validate_framework.py
  - Implement framework validation that checks all components work together
  - Purpose: Validate complete framework functionality
  - _Leverage: All components from previous phases_
  - _Requirements: All requirements validation_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: System Validation Engineer with expertise in integration testing | Task: Create validate_framework.py that validates all framework components work together covering all requirements | Restrictions: Check directory structure, fixtures, validators, performance tools, quality enforcement - ensure everything functions correctly | _Leverage: All implemented components from Phases 1-6_ | _Requirements: All requirements - comprehensive framework validation_ | Success: Validation script confirms all components work together and requirements are met | Instructions: Mark task in-progress [-], create framework validation, mark complete [x]_

- [ ] 7.4. Create development environment setup script
  - File: integration-tests/scripts/setup_dev_environment.sh
  - Create automated development environment setup
  - Purpose: Automate developer onboarding and environment setup
  - _Leverage: Virtual environment, package installation_
  - _Requirements: Developer productivity_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: DevOps Engineer with expertise in development environment automation | Task: Create setup_dev_environment.sh for automated development setup | Restrictions: Create virtual environment, install dependencies, configure pre-commit hooks, provide clear success confirmation | _Leverage: Python venv, pip, pre-commit installation_ | _Requirements: Development environment automation_ | Success: Setup script creates complete development environment from scratch | Instructions: Mark task in-progress [-], create environment setup script, mark complete [x]_

- [ ] 7.5. Create framework documentation
  - File: integration-tests/docs/framework_guide.md
  - Document the complete testing framework with usage examples
  - Purpose: Provide comprehensive developer documentation
  - _Leverage: All framework components and best practices_
  - _Requirements: Documentation for developer adoption_
  - _Prompt: Implement the task for spec integration-test-improvements, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Technical Writer with expertise in testing framework documentation | Task: Create framework_guide.md with comprehensive testing framework documentation | Restrictions: Include setup instructions, usage examples, category explanations, troubleshooting guide, maintain clear structure for developers | _Leverage: All implemented framework components, Python documentation standards_ | _Requirements: Complete framework documentation_ | Success: Documentation provides clear guidance for framework usage and development | Instructions: Mark task in-progress [-], create comprehensive documentation, mark complete [x]_
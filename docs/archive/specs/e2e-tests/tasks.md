# Tasks Document

## Phase 1: Directory Structure Setup

- [ ] 1.1. Create tests subdirectories
  - Files: e2e/tests/unit/__init__.py
  - Description: Create e2e/tests/unit/ directory with __init__.py
  - Purpose: Set up unit tests directory structure
  - _Leverage: Existing e2e/ directory_
  - _Requirements: 1.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in project structure | Task: Create e2e/tests/unit/ directory and add __init__.py file following requirement 1.1 | Restrictions: Only create this one directory, do not modify existing files | _Leverage: Current e2e/ directory structure_ | _Requirements: 1.1 - Test framework restructuring_ | Success: Directory e2e/tests/unit/ exists with __init__.py file | Instructions: Mark task in-progress [-], create directory and __init__.py, mark complete [x]_

- [ ] 1.2. Create integration tests subdirectory
  - Files: e2e/tests/integration/__init__.py
  - Description: Create e2e/tests/integration/ directory with __init__.py
  - Purpose: Set up integration tests directory structure
  - _Leverage: Existing e2e/ directory_
  - _Requirements: 1.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in project structure | Task: Create e2e/tests/integration/ directory and add __init__.py file following requirement 1.1 | Restrictions: Only create this one directory, do not modify existing files | _Leverage: Current e2e/ directory structure_ | _Requirements: 1.1 - Test framework restructuring_ | Success: Directory e2e/tests/integration/ exists with __init__.py file | Instructions: Mark task in-progress [-], create directory and __init__.py, mark complete [x]_

- [ ] 1.3. Create performance tests subdirectory
  - Files: e2e/tests/performance/__init__.py
  - Description: Create e2e/tests/performance/ directory with __init__.py
  - Purpose: Set up performance tests directory structure
  - _Leverage: Existing e2e/ directory_
  - _Requirements: 1.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in project structure | Task: Create e2e/tests/performance/ directory and add __init__.py file following requirement 1.1 | Restrictions: Only create this one directory, do not modify existing files | _Leverage: Current e2e/ directory structure_ | _Requirements: 1.1 - Test framework restructuring_ | Success: Directory e2e/tests/performance/ exists with __init__.py file | Instructions: Mark task in-progress [-], create directory and __init__.py, mark complete [x]_

- [ ] 1.4. Create compliance tests subdirectory
  - Files: e2e/tests/compliance/__init__.py
  - Description: Create e2e/tests/compliance/ directory with __init__.py
  - Purpose: Set up compliance tests directory structure
  - _Leverage: Existing e2e/ directory_
  - _Requirements: 1.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in project structure | Task: Create e2e/tests/compliance/ directory and add __init__.py file following requirement 1.1 | Restrictions: Only create this one directory, do not modify existing files | _Leverage: Current e2e/ directory structure_ | _Requirements: 1.1 - Test framework restructuring_ | Success: Directory e2e/tests/compliance/ exists with __init__.py file | Instructions: Mark task in-progress [-], create directory and __init__.py, mark complete [x]_

- [ ] 1.5. Create fixtures directory
  - Files: e2e/fixtures/__init__.py
  - Description: Create e2e/fixtures/ directory with __init__.py
  - Purpose: Set up centralized fixtures directory
  - _Leverage: Existing e2e/ directory_
  - _Requirements: 1.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in pytest fixtures | Task: Create e2e/fixtures/ directory and add __init__.py file following requirement 1.1 | Restrictions: Only create this one directory, do not modify existing files | _Leverage: Current e2e/ directory structure_ | _Requirements: 1.1 - Test framework restructuring_ | Success: Directory e2e/fixtures/ exists with __init__.py file | Instructions: Mark task in-progress [-], create directory and __init__.py, mark complete [x]_

- [ ] 1.6. Create utils directory
  - Files: e2e/utils/__init__.py
  - Description: Create e2e/utils/ directory with __init__.py
  - Purpose: Set up utilities directory for helper modules
  - _Leverage: Existing e2e/ directory_
  - _Requirements: 1.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in utility modules | Task: Create e2e/utils/ directory and add __init__.py file following requirement 1.1 | Restrictions: Only create this one directory, do not modify existing files | _Leverage: Current e2e/ directory structure_ | _Requirements: 1.1 - Test framework restructuring_ | Success: Directory e2e/utils/ exists with __init__.py file | Instructions: Mark task in-progress [-], create directory and __init__.py, mark complete [x]_

- [ ] 1.7. Create config directory
  - Files: e2e/config/__init__.py
  - Description: Create e2e/config/ directory with __init__.py
  - Purpose: Set up configuration directory for test settings
  - _Leverage: Existing e2e/ directory_
  - _Requirements: 1.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in configuration management | Task: Create e2e/config/ directory and add __init__.py file following requirement 1.1 | Restrictions: Only create this one directory, do not modify existing files | _Leverage: Current e2e/ directory structure_ | _Requirements: 1.1 - Test framework restructuring_ | Success: Directory e2e/config/ exists with __init__.py file | Instructions: Mark task in-progress [-], create directory and __init__.py, mark complete [x]_

## Phase 2: Fixture Consolidation

- [ ] 2.1. Analyze existing fixture duplication
  - Files: analysis/fixture_duplication_report.md
  - Description: Compare conftest.py and tests/fixtures.py to identify duplicated code
  - Purpose: Document what fixtures need consolidation
  - _Leverage: Existing conftest.py, tests/fixtures.py_
  - _Requirements: 1.3_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Code Analyst with expertise in identifying code duplication | Task: Analyze conftest.py and tests/fixtures.py to identify duplicated fixture code following requirement 1.3 | Restrictions: Only analyze, do not modify files, create analysis document | _Leverage: e2e/conftest.py, e2e/tests/fixtures.py_ | _Requirements: 1.3 - Code duplication elimination_ | Success: Report clearly identifies all duplicated fixtures and their locations | Instructions: Mark task in-progress [-], analyze fixture files, create report, mark complete [x]_

- [ ] 2.2. Create device fixtures module
  - Files: e2e/fixtures/device_fixtures.py
  - Description: Extract device-related fixtures from existing files into dedicated module
  - Purpose: Centralize device connection and configuration fixtures
  - _Leverage: ONVIFDeviceClient from tests/fixtures.py_
  - _Requirements: 5.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in pytest fixtures | Task: Create device_fixtures.py module with device-related fixtures extracted from existing files following requirement 5.1 | Restrictions: Only move device-related fixtures, maintain exact same functionality, use proper pytest fixture scoping | _Leverage: ONVIFDeviceClient patterns from e2e/tests/fixtures.py_ | _Requirements: 5.1 - Centralized reusable fixtures_ | Success: device_fixtures.py contains all device-related fixtures with proper scoping | Instructions: Mark task in-progress [-], create module and extract fixtures, mark complete [x]_

- [ ] 2.3. Create SOAP fixtures module
  - Files: e2e/fixtures/soap_fixtures.py
  - Description: Extract SOAP-related fixtures from existing files into dedicated module
  - Purpose: Centralize SOAP request/response fixtures
  - _Leverage: SOAP utilities from tests/fixtures.py_
  - _Requirements: 5.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in SOAP and XML fixtures | Task: Create soap_fixtures.py module with SOAP-related fixtures extracted from existing files following requirement 5.1 | Restrictions: Only move SOAP-related fixtures, maintain exact same functionality, preserve SOAP namespaces | _Leverage: SOAP request patterns from e2e/tests/fixtures.py_ | _Requirements: 5.1 - Centralized reusable fixtures_ | Success: soap_fixtures.py contains all SOAP-related fixtures working correctly | Instructions: Mark task in-progress [-], create module and extract SOAP fixtures, mark complete [x]_

- [ ] 2.4. Create performance fixtures module
  - Files: e2e/fixtures/performance_fixtures.py
  - Description: Extract performance-related fixtures from existing files into dedicated module
  - Purpose: Centralize performance testing fixtures
  - _Leverage: Performance tracking from tests/fixtures.py_
  - _Requirements: 5.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in performance testing fixtures | Task: Create performance_fixtures.py module with performance-related fixtures extracted from existing files following requirement 5.1 | Restrictions: Only move performance-related fixtures, maintain timing accuracy, preserve measurement functionality | _Leverage: performance_tracker patterns from e2e/tests/fixtures.py_ | _Requirements: 5.1 - Centralized reusable fixtures_ | Success: performance_fixtures.py contains all performance fixtures functioning correctly | Instructions: Mark task in-progress [-], create module and extract performance fixtures, mark complete [x]_

- [ ] 2.5. Create mock fixtures module
  - Files: e2e/fixtures/mock_fixtures.py
  - Description: Create mock fixtures for unit testing with mocked dependencies
  - Purpose: Provide mock objects for isolated unit tests
  - _Leverage: Existing mock patterns_
  - _Requirements: 5.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Developer with expertise in unittest.mock and pytest fixtures | Task: Create mock_fixtures.py module with mock objects for unit testing following requirement 5.1 | Restrictions: Create new mock fixtures, do not duplicate existing functionality, ensure proper mock isolation | _Leverage: Python unittest.mock patterns, existing test structure_ | _Requirements: 5.1 - Centralized reusable fixtures_ | Success: mock_fixtures.py provides complete mock fixtures for unit testing | Instructions: Mark task in-progress [-], create mock fixtures module, mark complete [x]_

- [ ] 2.6. Update fixture imports and remove duplication
  - Files: e2e/conftest.py, e2e/tests/fixtures.py
  - Description: Remove duplicated fixtures and update imports to use new fixture modules
  - Purpose: Complete fixture consolidation by removing duplication
  - _Leverage: New fixture modules from previous tasks_
  - _Requirements: 1.3_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Refactoring Expert with expertise in import management | Task: Remove duplicated fixtures from conftest.py and tests/fixtures.py, update imports to use new fixture modules following requirement 1.3 | Restrictions: Do not break existing test functionality, ensure all imports work correctly, maintain backward compatibility | _Leverage: New fixture modules created in tasks 2.2-2.5_ | _Requirements: 1.3 - Code duplication elimination_ | Success: Zero fixture duplication remains, all imports work, existing tests pass | Instructions: Mark task in-progress [-], remove duplicates and update imports, mark complete [x]_

## Phase 3: ONVIF Compliance Framework

- [ ] 3.1. Create base compliance data structures
  - Files: e2e/utils/compliance_types.py
  - Description: Define ComplianceResult, ComplianceLevel enums and base validation classes
  - Purpose: Establish foundation data structures for compliance validation
  - _Leverage: Python dataclasses and enum patterns_
  - _Requirements: 3.4_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Data Structure Engineer with expertise in dataclasses and type systems | Task: Create compliance_types.py with ComplianceResult dataclass and ComplianceLevel enum following requirement 3.4 | Restrictions: Only create data structures, do not implement validation logic yet, use proper type hints | _Leverage: Python dataclasses, enum, typing modules_ | _Requirements: 3.4 - Detailed compliance reporting structure_ | Success: ComplianceResult and ComplianceLevel properly defined with type hints | Instructions: Mark task in-progress [-], create compliance data structures, mark complete [x]_

- [ ] 3.2. Create SOAP validation utilities
  - Files: e2e/utils/soap_validator.py
  - Description: Implement SOAP envelope and response format validation utilities
  - Purpose: Validate SOAP format compliance for ONVIF requests/responses
  - _Leverage: Existing XML parsing from tests/fixtures.py_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: XML/SOAP Validation Expert with expertise in SOAP envelope validation | Task: Create soap_validator.py with SOAP format validation utilities following requirement 3.1 | Restrictions: Only validate SOAP format, do not implement ONVIF-specific validation yet, ensure proper XML namespace handling | _Leverage: XML parsing patterns from e2e/tests/fixtures.py, ONVIF namespace definitions_ | _Requirements: 3.1 - SOAP envelope and format validation_ | Success: SOAP validator correctly validates envelope structure and namespaces | Instructions: Mark task in-progress [-], create SOAP validation utilities, mark complete [x]_

- [ ] 3.3. Create Profile S validator foundation
  - Files: e2e/utils/profile_s_validator.py
  - Description: Create base ONVIFProfileSValidator class with basic structure
  - Purpose: Establish foundation for Profile S compliance validation
  - _Leverage: SOAP validator from task 3.2_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Profile S Specialist with expertise in ONVIF compliance requirements | Task: Create profile_s_validator.py with base ONVIFProfileSValidator class following requirement 3.1 | Restrictions: Only create class structure and basic methods, do not implement full validation logic yet, ensure proper inheritance | _Leverage: soap_validator.py from task 3.2, ComplianceResult from task 3.1_ | _Requirements: 3.1 - Profile S compliance validation foundation_ | Success: ONVIFProfileSValidator class exists with basic structure and inheritance | Instructions: Mark task in-progress [-], create Profile S validator foundation, mark complete [x]_

- [ ] 3.4. Implement Profile S user authentication validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_user_authentication method to Profile S validator
  - Purpose: Validate ONVIF Profile S Section 7.1 user authentication requirement
  - _Leverage: Existing device client authentication patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Security Testing Engineer with expertise in authentication validation | Task: Implement validate_user_authentication method in Profile S validator following requirement 3.1 | Restrictions: Only implement user authentication validation, test both valid and invalid credentials, return ComplianceResult | _Leverage: Device client authentication from existing fixtures_ | _Requirements: 3.1 - Profile S mandatory user authentication_ | Success: Authentication validation tests both positive and negative scenarios correctly | Instructions: Mark task in-progress [-], implement authentication validation, mark complete [x]_

- [ ] 3.5. Implement Profile S capabilities validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_capabilities method to Profile S validator
  - Purpose: Validate ONVIF Profile S Section 7.2 capabilities requirement
  - _Leverage: Existing capabilities request patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Capabilities Expert with expertise in device capability validation | Task: Implement validate_capabilities method in Profile S validator following requirement 3.1 | Restrictions: Only implement capabilities validation, ensure required categories present, return ComplianceResult | _Leverage: Existing GetCapabilities patterns from device tests_ | _Requirements: 3.1 - Profile S mandatory capabilities_ | Success: Capabilities validation checks all required categories and XAddr fields | Instructions: Mark task in-progress [-], implement capabilities validation, mark complete [x]_

- [ ] 3.6. Implement Profile S discovery validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_discovery method for WS-Discovery compliance
  - Purpose: Validate ONVIF Profile S Section 7.3 discovery requirement
  - _Leverage: Existing WS-Discovery patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: WS-Discovery Expert with expertise in device discovery protocols | Task: Implement validate_discovery method in Profile S validator following requirement 3.1 | Restrictions: Only implement discovery validation, test device probe/response cycle, validate proper device types and scopes | _Leverage: Existing WS-Discovery patterns from test suite_ | _Requirements: 3.1 - Profile S mandatory discovery_ | Success: Discovery validation tests WS-Discovery probe/response correctly with proper device identification | Instructions: Mark task in-progress [-], implement discovery validation, mark complete [x]_

- [ ] 3.7. Implement Profile S network configuration validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_network_configuration method for network settings compliance
  - Purpose: Validate ONVIF Profile S Section 7.4 network configuration requirement
  - _Leverage: Device service network operations_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Network Configuration Expert with expertise in IP network validation | Task: Implement validate_network_configuration method in Profile S validator following requirement 3.1 | Restrictions: Only implement network configuration validation, test GetNetworkInterfaces operation, validate interface settings | _Leverage: Device service network operations, SOAP helpers_ | _Requirements: 3.1 - Profile S mandatory network configuration_ | Success: Network configuration validation checks interface settings and network parameters | Instructions: Mark task in-progress [-], implement network validation, mark complete [x]_

- [ ] 3.8. Implement Profile S system datetime validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_system_datetime method for datetime compliance
  - Purpose: Validate ONVIF Profile S Section 7.5 system datetime requirement
  - _Leverage: GetSystemDateAndTime operation patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: System Time Expert with expertise in datetime synchronization validation | Task: Implement validate_system_datetime method in Profile S validator following requirement 3.1 | Restrictions: Only implement datetime validation, test GetSystemDateAndTime operation, validate timezone and NTP settings | _Leverage: GetSystemDateAndTime operation from device tests_ | _Requirements: 3.1 - Profile S mandatory system datetime_ | Success: Datetime validation checks system time, timezone, and NTP configuration correctly | Instructions: Mark task in-progress [-], implement datetime validation, mark complete [x]_

- [ ] 3.9. Implement Profile S device information validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_device_information method for device info compliance
  - Purpose: Validate ONVIF Profile S Section 7.6 device information requirement
  - _Leverage: GetDeviceInformation operation patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Device Information Expert with expertise in ONVIF device metadata validation | Task: Implement validate_device_information method in Profile S validator following requirement 3.1 | Restrictions: Only implement device information validation, verify required fields (Manufacturer, Model, etc.), validate field formats | _Leverage: GetDeviceInformation operation from device tests_ | _Requirements: 3.1 - Profile S mandatory device information_ | Success: Device information validation checks all required fields and proper formats | Instructions: Mark task in-progress [-], implement device info validation, mark complete [x]_

- [ ] 3.10. Implement Profile S scopes validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_scopes method for device scope compliance
  - Purpose: Validate ONVIF Profile S Section 7.7 scopes requirement
  - _Leverage: GetScopes operation patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Scopes Expert with expertise in device scope configuration | Task: Implement validate_scopes method in Profile S validator following requirement 3.1 | Restrictions: Only implement scopes validation, test GetScopes operation, validate scope format and categories | _Leverage: Device service scope operations, SOAP helpers_ | _Requirements: 3.1 - Profile S mandatory scopes_ | Success: Scopes validation checks scope configuration and format compliance | Instructions: Mark task in-progress [-], implement scopes validation, mark complete [x]_

- [ ] 3.11. Implement Profile S services validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_services method for service endpoint compliance
  - Purpose: Validate ONVIF Profile S Section 7.8 services requirement
  - _Leverage: GetServices operation patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Services Expert with expertise in service endpoint validation | Task: Implement validate_services method in Profile S validator following requirement 3.1 | Restrictions: Only implement services validation, test GetServices operation, validate required service endpoints and versions | _Leverage: GetServices operation from device tests_ | _Requirements: 3.1 - Profile S mandatory services_ | Success: Services validation checks all required service endpoints and proper versions | Instructions: Mark task in-progress [-], implement services validation, mark complete [x]_

- [ ] 3.12. Implement Profile S media profiles validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_media_profiles method for media profile compliance
  - Purpose: Validate ONVIF Profile S Section 7.9 media profiles requirement
  - _Leverage: Media service patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Media Expert with expertise in media profile validation | Task: Implement validate_media_profiles method in Profile S validator following requirement 3.1 | Restrictions: Only implement media profiles validation, test GetProfiles operation, validate required profile configurations | _Leverage: Media service patterns from existing tests_ | _Requirements: 3.1 - Profile S mandatory media profiles_ | Success: Media profiles validation checks profile configuration and capabilities | Instructions: Mark task in-progress [-], implement media profiles validation, mark complete [x]_

- [ ] 3.13. Implement Profile S streaming entrypoint validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_streaming method that orchestrates stream validation workflow
  - Purpose: Invoke stream validator to cover Real Time Streaming v24.12 checkpoints
  - _Leverage: GetStreamUri operation patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Streaming Expert with expertise in video streaming validation | Task: Implement validate_streaming method in Profile S validator so it resolves stream URIs, triggers RTSP handshake tests, and aggregates compliance results following requirement 3.1 | Restrictions: Do not embed handshake logic here—delegate to dedicated stream validator utilities | _Leverage: GetStreamUri operation patterns from media tests, new stream validator utilities_ | _Requirements: 3.1 - Profile S mandatory streaming_ | Success: Streaming validation method returns comprehensive ComplianceResult populated from downstream stream validator outcomes | Instructions: Mark task in-progress [-], implement streaming entrypoint, mark complete [x]_

- [ ] 3.13a. Create Profile S stream validator foundation
  - Files: e2e/utils/profile_s_stream_validator.py
  - Description: Implement ProfileSStreamValidator class with configuration, RTSP client scaffolding, and result model
  - Purpose: Establish reusable component for RTSP/RTP compliance checks
  - _Leverage: Device configuration utilities, ComplianceResult data structures_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: RTSP/RTP Streaming Specialist | Task: Create ProfileSStreamValidator class scaffolding handling session setup, authentication, and result aggregation following requirement 3.1 | Restrictions: Do not yet implement handshake verification—focus on class structure, dependency injection, and diagnostics collection | _Leverage: DeviceConfig, ONVIF client session helpers_ | _Requirements: 3.1 - Profile S streaming validator foundation_ | Success: Stream validator class exists with configurable parameters, authentication handling, and placeholder methods for handshake/SDP/RTP validation | Instructions: Mark task in-progress [-], implement stream validator foundation, mark complete [x]_

- [ ] 3.13b. Implement RTSP handshake validation
  - Files: e2e/utils/profile_s_stream_validator.py (extend)
  - Description: Implement validation of OPTIONS/DESCRIBE/SETUP/PLAY/TEARDOWN sequence with authentication support
  - Purpose: Verify Real Time Streaming v24.12 handshake behaviour
  - _Leverage: HTTP Digest utilities, RTSP client scaffolding_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: RTSP Protocol Analyst with expertise in ONVIF streaming | Task: Implement RTSP handshake validation covering OPTIONS, DESCRIBE (with SDP capture), SETUP, PLAY, and TEARDOWN including timeout/error handling following requirement 3.1 | Restrictions: Ensure Digest and WS-UsernameToken authentication paths, record status codes/headers, and return structured diagnostics | _Leverage: Requests session helpers, Profile S validator entrypoint_ | _Requirements: 3.1 - Profile S mandatory RTSP handshake_ | Success: Handshake validation detects missing verbs, incorrect status codes, authentication failures, or TEARDOWN issues and reports them via ComplianceResult | Instructions: Mark task in-progress [-], implement handshake validation, mark complete [x]_

- [ ] 3.13c. Implement SDP inspection and validation
  - Files: e2e/utils/profile_s_stream_validator.py (extend)
  - Description: Parse SDP from DESCRIBE response and validate media/control attributes against spec
  - Purpose: Enforce Real Time Streaming v24.12 SDP requirements
  - _Leverage: lxml/pyparsing SDP utilities, media configuration data_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: SDP Specialist with expertise in ONVIF streaming descriptors | Task: Validate SDP contents including media sections, codecs, control URLs, metadata channels, and transport profiles following requirement 3.1 | Restrictions: Reject mismatched connection data, missing attributes, or unsupported codecs; capture remediation hints | _Leverage: Media configuration validators, DeviceConfig expectations_ | _Requirements: 3.1 - Profile S SDP validation_ | Success: SDP validation passes all compliant descriptors and flags deviations with actionable diagnostics | Instructions: Mark task in-progress [-], implement SDP validation, mark complete [x]_

- [ ] 3.13d. Implement RTP continuity and live playback validation
  - Files: e2e/utils/profile_s_stream_validator.py (extend)
  - Description: Monitor RTP/RTCP packets during PLAY and confirm continuity, jitter, and media decodability
  - Purpose: Validate ongoing media delivery per Real Time Streaming v24.12
  - _Leverage: aiortc/ffmpeg-python, scapy/pyshark packet capture helpers_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: RTP Media Analyst with expertise in packet-level video validation | Task: Capture RTP streams, verify sequence/timestamp progression, decode sample frames, compute packet loss/jitter, and assert TEARDOWN resource cleanup following requirement 3.1 | Restrictions: Support configurable observation window, log RTCP statistics, and surface decoded frame hashes for regression tracking | _Leverage: Performance metrics utilities, RTSP handshake output_ | _Requirements: 3.1 - Profile S RTP continuity and playback validation_ | Success: Validation detects packet loss, jitter, decode failures, or missing RTCP reports and returns detailed diagnostics while passing compliant streams | Instructions: Mark task in-progress [-], implement RTP continuity validation, mark complete [x]_

- [ ] 3.14. Implement Profile S video encoder configuration validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_video_encoder_configuration method for encoder compliance
  - Purpose: Validate ONVIF Profile S Section 7.11 video encoder requirement
  - _Leverage: Video encoder configuration patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Video Encoder Expert with expertise in H.264 configuration validation | Task: Implement validate_video_encoder_configuration method in Profile S validator following requirement 3.1 | Restrictions: Only implement video encoder validation, test GetVideoEncoderConfiguration operations, validate encoding parameters | _Leverage: Video encoder patterns from media service tests_ | _Requirements: 3.1 - Profile S mandatory video encoder configuration_ | Success: Video encoder validation checks H.264 configuration and parameters | Instructions: Mark task in-progress [-], implement video encoder validation, mark complete [x]_

- [ ] 3.15. Implement Profile S video source configuration validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_video_source_configuration method for video source compliance
  - Purpose: Validate ONVIF Profile S Section 7.12 video source requirement
  - _Leverage: Video source configuration patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Video Source Expert with expertise in video input validation | Task: Implement validate_video_source_configuration method in Profile S validator following requirement 3.1 | Restrictions: Only implement video source validation, test GetVideoSourceConfiguration operations, validate source parameters | _Leverage: Video source patterns from media service tests_ | _Requirements: 3.1 - Profile S mandatory video source configuration_ | Success: Video source validation checks source configuration and capabilities | Instructions: Mark task in-progress [-], implement video source validation, mark complete [x]_

- [ ] 3.16. Implement Profile S relay outputs validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_relay_outputs method for relay output compliance
  - Purpose: Validate ONVIF Profile S Section 7.13 relay outputs requirement
  - _Leverage: Device I/O patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Device I/O Expert with expertise in relay output validation | Task: Implement validate_relay_outputs method in Profile S validator following requirement 3.1 | Restrictions: Only implement relay outputs validation, test GetRelayOutputs operation if supported, mark as conditional if not applicable | _Leverage: Device I/O patterns, conditional feature handling_ | _Requirements: 3.1 - Profile S conditional relay outputs_ | Success: Relay outputs validation properly identifies applicability and validates if present | Instructions: Mark task in-progress [-], implement relay outputs validation, mark complete [x]_

- [ ] 3.16a. Parse Authentication Behavior spec artifacts
  - Files: e2e/utils/authentication_profile_spec.py
  - Description: Extract ONVIF Authentication Behavior Device Test Specification v19.06 metadata into reusable structures
  - Purpose: Provide programmable access to Authentication Profile Info/Management test cases for Profile S compliance
  - _Leverage: testspecs/device/ONVIF_Authentication_Behavior_Device_Test_Specification.xml_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Specification Analyst with expertise in DocBook processing | Task: Create authentication_profile_spec.py that loads testspecs/device/ONVIF_Authentication_Behavior_Device_Test_Specification.xml, normalizes chapter/section identifiers (Authentication Profile Info, Authentication Profile, Authentication Profile Management), and exposes structured data for required operations, negative cases, and event expectations | Restrictions: Use safe XML parsing (defusedxml or lxml with secure settings), cache parsed results, document spec version constants | _Leverage: DocBook parsing patterns, ComplianceResult data contracts_ | _Requirements: 3.1 - Authentication Behavior specification coverage foundation_ | Success: Module returns typed objects mapping operation names to spec IDs, inputs, expected results, and event requirements ready for validator consumption | Instructions: Mark task in-progress [-], implement spec parsing module, mark complete [x]_

- [ ] 3.16b. Implement Profile S authentication profile validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_authentication_profiles method leveraging Authentication Behavior spec data
  - Purpose: Enforce Profile S credential management rules (GetAuthenticationProfileInfo*, Create/Modify/Delete/Set, Changed/Removed events)
  - _Leverage: authentication_profile_spec.py from task 3.16a, SOAP helpers, event fixtures_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Authentication Compliance Engineer with expertise in ONVIF device security | Task: Implement validate_authentication_profiles in Profile S validator to execute Authentication Profile Info and Authentication Profile Management flows defined in the v19.06 spec, including positive/negative credential scenarios, MaxLimit/Limit enforcement, duplicate token checks, and PullPoint event verification for Changed/Removed notifications | Restrictions: Support both Digest and WS-UsernameToken credentials, ensure cleanup of created profiles/schedules/security levels, surface detailed ComplianceResult artifacts | _Leverage: Device client authentication flows, event subscription utilities_ | _Requirements: 3.1 - Authentication Behavior compliance_ | Success: Validation method reports pass/fail with per-step diagnostics covering all mandatory spec cases and restores DUT to baseline | Instructions: Mark task in-progress [-], implement authentication validation, mark complete [x]_

- [ ] 3.16c. Create Profile S Authentication Behavior compliance tests
  - Files: e2e/tests/compliance/test_profile_s_authentication_behavior.py
  - Description: Implement pytest suite invoking Profile S authentication validator and asserting spec coverage
  - Purpose: Automate Authentication Behavior Device Test Specification checks for Profile S profile
  - _Leverage: Profile S validator method from task 3.16b, spec metadata module, event fixtures_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Authentication Test Engineer with expertise in security regression testing | Task: Build test_profile_s_authentication_behavior.py that wires profile_s_validator.validate_authentication_profiles() into pytest with @pytest.mark.integration and a new @pytest.mark.authentication tag, exercises both valid and invalid credential flows, asserts ComplianceResult contents, and verifies event delivery timelines per spec | Restrictions: Ensure tests are idempotent, perform resource cleanup via fixtures/finalizers, skip gracefully when Authentication Profile feature not supported | _Leverage: pytest fixtures for device sessions, compliance reporting utilities_ | _Requirements: 3.1 - Automated Authentication Behavior coverage_ | Success: Test suite produces pass/fail output mapped to spec IDs and integrates with compliance reporting pipeline | Instructions: Mark task in-progress [-], implement authentication behavior tests, mark complete [x]_

- [ ] 3.16d. Parse Base Device spec artifacts
  - Files: e2e/utils/base_device_spec.py
  - Description: Extract ONVIF Base Device Test Specification v21.12 chapters into structured metadata
  - Purpose: Provide machine-readable definitions for Device Information, System, Network, and User Management test cases
  - _Leverage: testspecs/device/ONVIF_Base_Device_Test_Specification.xml_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Specification Analyst with expertise in DocBook parsing | Task: Build base_device_spec.py that parses the Base Device Test Specification (v21.12) extracting section IDs, required operations (GetDeviceInformation, GetCapabilities, GetSystemDateAndTime, SetSystemDateAndTime, SetNTP, SetHostname, SetNetworkInterfaces, CreateUsers/DeleteUsers/SetUser, system reboot/logout), expected SOAP faults (TooManyUsers, InvalidArgs, ActionNotSupported), and pre/post conditions | Restrictions: Use secure XML parsing (defusedxml/lxml with hardened settings), cache parsed content, expose typed dataclasses for lookup by validator modules, document spec version constants | _Leverage: DocBook parsing patterns from authentication_profile_spec.py, ComplianceResult data structures_ | _Requirements: 3.1 - Base Device specification metadata foundation_ | Success: Module returns deterministic structures mapping spec test IDs to operations, parameters, and pass/fail criteria ready for validator consumption | Instructions: Mark task in-progress [-], implement spec parsing module, mark complete [x]_

- [ ] 3.16e. Implement Profile S base device validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_base_device_compliance method consuming base_device_spec metadata
  - Purpose: Execute Base Device Test Specification flows for Profile S mandatory features
  - _Leverage: base_device_spec.py from task 3.16d, SOAP helpers, device fixtures_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Device Compliance Engineer with expertise in core device services | Task: Implement validate_base_device_compliance to run spec-defined sequences covering GetDeviceInformation, GetCapabilities, GetSystemDateAndTime/SetSystemDateAndTime (including DST and manual adjustments), NTP/Hostname/Network interface configuration, user lifecycle (CreateUsers/SetUser/DeleteUsers with TooManyUsers handling), system reboot, and auxiliary operations required by Profile S | Restrictions: Ensure rollback/cleanup for configuration changes, support Digest and WS-UsernameToken authentication, capture SOAP requests/responses for reporting, annotate ComplianceResult entries with spec IDs and severity | _Leverage: Existing validator helpers, authentication fixtures, event subscription utilities_ | _Requirements: 3.1 - Base Device compliance coverage_ | Success: Validation method produces comprehensive ComplianceResult with per-test diagnostics, restores DUT to baseline, and honors spec-defined pass/fail conditions | Instructions: Mark task in-progress [-], implement base device validation, mark complete [x]_

- [ ] 3.16f. Create Profile S Base Device compliance tests
  - Files: e2e/tests/compliance/test_profile_s_base_device.py
  - Description: Implement pytest suite executing Profile S base device validator flows
  - Purpose: Automate Base Device Test Specification coverage for Profile S
  - _Leverage: validate_base_device_compliance from task 3.16e, base_device_spec metadata, compliance reporting utilities_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Device Test Engineer with expertise in system configuration validation | Task: Create test_profile_s_base_device.py using pytest markers (@pytest.mark.integration, @pytest.mark.base_device) to run Profile S base device checks, assert ComplianceResult contents, verify SOAP fault expectations (TooManyUsers, InvalidArgs), and aggregate results into compliance reports | Restrictions: Ensure tests are idempotent, clean up created users/network settings, provide configurable fixture hooks for administrative credentials | _Leverage: pytest fixtures, compliance reporting system, spec metadata module_ | _Requirements: 3.1 - Automated Base Device coverage_ | Success: Test suite maps pass/fail outcomes to spec IDs, integrates with reports, and gracefully skips when required capabilities absent | Instructions: Mark task in-progress [-], implement base device compliance tests, mark complete [x]_

- [ ] 3.16g. Parse Credential Device spec artifacts
  - Files: e2e/utils/credential_device_spec.py
  - Description: Extract ONVIF Credential Device Test Specification v20.12 functional blocks into reusable data structures
  - Purpose: Provide structured metadata for credential info, management, identifiers, access profiles, and events
  - _Leverage: testspecs/device/ONVIF_Credential_Device_Test_Specification.xml_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Specification Analyst with expertise in DocBook parsing | Task: Build credential_device_spec.py that parses the v20.12 Credential Device Test Specification, capturing section and test IDs (Credential Info, Credential, Credential States, Credential Identifiers, Credential Access Profiles, Credential Events), required SOAP operations (GetCredentialInfo*, CreateCredential, ModifyCredential, Enable/DisableCredential, SetCredentialIdentifier, SetCredentialAccessProfile, DeleteCredential*, SetCredential), capability prerequisites (MaxWhitelistedItems/MaxBlacklistedItems, DefaultCredentialSuspensionDuration), helper procedures, expected event tokens, and positive/negative result criteria | Restrictions: Use hardened XML parsing (defusedxml or secured lxml), cache parsed representation, expose typed dataclasses for validators, document spec version constants | _Leverage: Parsing patterns from authentication_profile_spec.py and base_device_spec.py, ComplianceResult data contracts_ | _Requirements: 3.1 - Credential specification metadata foundation_ | Success: Module returns deterministic structures mapping test cases to operations, inputs, expected faults/events, and cleanup requirements ready for validator use | Instructions: Mark task in-progress [-], implement spec parsing module, mark complete [x]_

- [ ] 3.16h. Implement Profile S credential service validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_credential_service method leveraging credential spec metadata
  - Purpose: Ensure Profile S devices meet Credential Service requirements (info, lifecycle, identifiers, access profiles, events)
  - _Leverage: credential_device_spec.py from task 3.16g, SOAP helpers, event fixtures_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Credential Compliance Engineer with expertise in ONVIF security services | Task: Implement validate_credential_service to execute spec-defined flows covering GetCredentialInfo/List, Create/Modify/Delete/SetCredential, Enable/DisableCredential, identifier whitelist/blacklist updates, access profile assignments, and Changed/Removed events; verify capability constraints (MaxWhitelistedItems, MaxCredentialStates, DefaultCredentialSuspensionDuration), fault behavior, and rollback cleanup | Restrictions: Support Digest and WS-UsernameToken authentication, ensure DUT state cleanup, annotate ComplianceResult with spec IDs and severity, reuse helper procedures for identifier generation and event polling | _Leverage: Existing validator scaffolding, authentication utilities, event subscription helpers_ | _Requirements: 3.1 - Credential Service compliance coverage_ | Success: Validation returns detailed ComplianceResult entries for each spec test case, restores DUT to baseline, and flags missing capabilities vs failures appropriately | Instructions: Mark task in-progress [-], implement credential validation, mark complete [x]_

- [ ] 3.16i. Create Profile S Credential Device compliance tests
  - Files: e2e/tests/compliance/test_profile_s_credential_device.py
  - Description: Implement pytest suite executing Profile S credential validator workflows
  - Purpose: Automate Credential Device Test Specification coverage for Profile S
  - _Leverage: validate_credential_service from task 3.16h, credential spec metadata, compliance reporting utilities_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Credential Test Engineer with expertise in credential lifecycle validation | Task: Create test_profile_s_credential_device.py with pytest markers (@pytest.mark.integration, @pytest.mark.credential) executing credential compliance flows, asserting ComplianceResult mapping to spec IDs, verifying identifier whitelist/blacklist behavior, and confirming event delivery timelines | Restrictions: Ensure tests are idempotent with robust fixture cleanup for created credentials/identifiers, skip gracefully when credential service unsupported, provide admin credential hooks | _Leverage: pytest fixtures, compliance reporting system, credential spec metadata module_ | _Requirements: 3.1 - Automated Credential Device coverage_ | Success: Test suite produces actionable pass/fail results linked to spec cases and feeds compliance reports | Instructions: Mark task in-progress [-], implement credential compliance tests, mark complete [x]_

- [ ] 3.16j. Parse Feature Discovery spec artifacts
  - Files: e2e/utils/feature_discovery_spec.py
  - Description: Extract ONVIF Device Feature Discovery Specification v??.?? feature definitions into structured metadata
  - Purpose: Provide reusable mapping of service/features to discovery procedures and validation criteria
  - _Leverage: testspecs/device/ONVIF_Device_Feature_Discovery_Specification.xml_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Specification Analyst with expertise in feature discovery | Task: Build feature_discovery_spec.py that parses the Device Feature Discovery Specification, capturing feature tables for GetServices and GetCapabilities per service (Device, Media, Media2, PTZ, Analytics, Credential, Access Control, Door Control, Security Configuration, etc.), prerequisites, capability flags, entity-level feature requirements, and expected SOAP operations | Restrictions: Use secure XML parsing (defusedxml or hardened lxml), retain spec version constants, expose typed dataclasses for validators, cache parsed content for reuse | _Leverage: Parsing patterns from authentication_profile_spec.py/base_device_spec.py, ComplianceResult data structures_ | _Requirements: 3.1 - Feature discovery specification metadata foundation_ | Success: Module returns deterministic structures mapping feature IDs to required services, operations, capability fields, and qualification rules for validator consumption | Instructions: Mark task in-progress [-], implement feature discovery spec parser, mark complete [x]_

- [ ] 3.16k. Implement Profile S feature discovery validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_feature_discovery method using feature discovery metadata
  - Purpose: Ensure Profile S devices advertise mandatory features via GetServices/GetCapabilities per spec
  - _Leverage: feature_discovery_spec.py from task 3.16j, SOAP helpers, compliance reporting_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Feature Discovery Compliance Engineer with expertise in ONVIF service discovery | Task: Implement validate_feature_discovery to evaluate service feature tables (Device, Media, Media2, PTZ, Credential, Access Control, Security, Door Control, etc.), confirm required capabilities (ClientSuppliedToken, Metadata, EnabledTLSVersions, Whitelist/Blacklist support, etc.), handle entity-specific feature checks (per relay/PTZ node), and report missing/extra features aligned with spec | Restrictions: Support Digest/WS-UsernameToken authentication, reuse GetServices/GetCapabilities caching, annotate ComplianceResult with spec IDs and severity, differentiate unsupported services vs missing mandatory features | _Leverage: Existing validator infrastructure, service capability utilities_ | _Requirements: 3.1 - Feature discovery compliance coverage_ | Success: Validation produces detailed ComplianceResult entries covering all Profile S relevant features, highlighting deviations and restoring baseline state | Instructions: Mark task in-progress [-], implement feature discovery validation, mark complete [x]_

- [ ] 3.16l. Create Profile S Feature Discovery compliance tests
  - Files: e2e/tests/compliance/test_profile_s_feature_discovery.py
  - Description: Implement pytest suite executing Profile S feature discovery validator
  - Purpose: Automate Device Feature Discovery Specification coverage for Profile S
  - _Leverage: validate_feature_discovery from task 3.16k, feature discovery spec metadata, compliance reporting utilities_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Discovery Test Engineer with expertise in service capability validation | Task: Create test_profile_s_feature_discovery.py with pytest markers (@pytest.mark.integration, @pytest.mark.discovery) to run feature discovery checks, assert ComplianceResult mapping to spec IDs, verify per-entity feature handling, and integrate results into compliance reports | Restrictions: Ensure tests are idempotent, reuse cached GetServices/GetCapabilities responses when possible, provide fixture hooks for capability overrides, skip gracefully when optional services absent | _Leverage: pytest fixtures, compliance reporting system, feature discovery spec metadata module_ | _Requirements: 3.1 - Automated Feature Discovery coverage_ | Success: Test suite outputs actionable feature discovery compliance status linked to spec references | Instructions: Mark task in-progress [-], implement feature discovery tests, mark complete [x]_

- [ ] 3.16m. Parse Imaging Device spec artifacts
  - Files: e2e/utils/imaging_device_spec.py
  - Description: Extract ONVIF Imaging Device Test Specification v20.06 sections into structured metadata
  - Purpose: Provide reusable mapping for imaging service test cases (GetImagingSettings, SetImagingSettings, Move, FocusControl, etc.)
  - _Leverage: testspecs/device/ONVIF_Imaging_Device_Test_Specification.xml_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Specification Analyst with expertise in imaging services | Task: Build imaging_device_spec.py that parses the Imaging Device Test Specification, capturing section/test IDs for imaging configuration, move/focus operations, imaging presets, MoveFocus/Stop/AutoFocus, MoveToPosition, GetServiceCapabilities, and negative scenarios; record capability prerequisites (ContinuousFocus, MoveFocusSupported, Preset support), helper procedures, parameter ranges, and expected faults | Restrictions: Use secure XML parsing (defusedxml or hardened lxml), expose typed dataclasses with cached results, document spec version constants | _Leverage: Parsing patterns from authentication_profile_spec.py/base_device_spec.py, ComplianceResult data structures_ | _Requirements: 3.1 - Imaging specification metadata foundation_ | Success: Module returns deterministic structures mapping test cases to required operations, parameters, and pass/fail expectations ready for validator consumption | Instructions: Mark task in-progress [-], implement imaging spec parser, mark complete [x]_

- [ ] 3.16n. Implement Profile S imaging device validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_imaging_device_compliance method leveraging imaging spec metadata
  - Purpose: Ensure Profile S devices satisfy Imaging Device Test Specification requirements
  - _Leverage: imaging_device_spec.py from task 3.16m, SOAP helpers, device imaging fixtures_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Imaging Compliance Engineer with expertise in ONVIF imaging controls | Task: Implement validate_imaging_device_compliance to execute spec workflows covering Get/SetImagingSettings (including parameter boundary checks), Move/Stop/MoveToPosition, focus controls (Auto/Manual/Relative), imaging presets (Get/Set/Delete), and capability verification; validate error handling for unsupported parameters and ensure state rollback | Restrictions: Support Digest/WS-UsernameToken authentication, respect conditional features (focus capabilities, presets), annotate ComplianceResult with spec IDs and severity, capture before/after imaging settings | _Leverage: Existing imaging validation utilities, compliance reporting framework_ | _Requirements: 3.1 - Imaging Device compliance coverage_ | Success: Validation produces comprehensive ComplianceResult entries per spec case, restores original imaging settings, and highlights unsupported vs failing features | Instructions: Mark task in-progress [-], implement imaging compliance validator, mark complete [x]_

- [ ] 3.16o. Create Profile S Imaging Device compliance tests
  - Files: e2e/tests/compliance/test_profile_s_imaging_device.py
  - Description: Implement pytest suite executing Profile S imaging compliance validator
  - Purpose: Automate Imaging Device Test Specification coverage for Profile S
  - _Leverage: validate_imaging_device_compliance from task 3.16n, imaging spec metadata, compliance reporting utilities_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Imaging Test Engineer with expertise in imaging parameter validation | Task: Create test_profile_s_imaging_device.py with pytest markers (@pytest.mark.integration, @pytest.mark.imaging) executing imaging compliance flows, asserting ComplianceResult mapping to spec IDs, verifying focus/move/preset behavior, and integrating outcomes into compliance reports | Restrictions: Ensure tests are idempotent by restoring imaging settings/presets, provide configurable fixtures for target focus positions, skip gracefully when optional features unavailable | _Leverage: pytest fixtures, compliance reporting system, imaging spec metadata module_ | _Requirements: 3.1 - Automated Imaging Device coverage_ | Success: Test suite produces actionable imaging compliance results tied to spec references | Instructions: Mark task in-progress [-], implement imaging compliance tests, mark complete [x]_

- [ ] 3.16p. Parse Media2 Configuration Device spec artifacts
  - Files: e2e/utils/media2_configuration_spec.py
  - Description: Extract ONVIF Media2 Configuration Device Test Specification v21.12 sections into structured metadata
  - Purpose: Provide reusable mapping for Media2 configuration test cases (Get/SetMedia2 configurations, tracks, transport, metadata)
  - _Leverage: testspecs/device/ONVIF_Media2_Configuration_Device_Test_Specification.xml_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Specification Analyst with expertise in Media2 services | Task: Build media2_configuration_spec.py that parses the Media2 Configuration Device Test Specification, capturing section/test IDs for Media2 profile handling, video/audio configuration management, multicast/unicast transport, metadata tracks, snapshot settings, ROI, analytics, and conditional features; record capability prerequisites, helper procedures, parameter ranges, and expected SOAP faults | Restrictions: Use secure XML parsing (defusedxml or hardened lxml), expose typed dataclasses with cached data, document spec version constants | _Leverage: Parsing patterns from imaging_device_spec.py and base_device_spec.py, ComplianceResult data structures_ | _Requirements: 3.1 - Media2 configuration specification metadata foundation_ | Success: Module returns deterministic structures mapping spec test cases to operations, parameters, and pass/fail expectations ready for validator use | Instructions: Mark task in-progress [-], implement Media2 spec parser, mark complete [x]_

- [ ] 3.16q. Implement Profile S Media2 configuration validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_media2_configuration_compliance method using Media2 spec metadata
  - Purpose: Ensure Profile S devices satisfy Media2 configuration requirements relevant to Profile S capabilities
  - _Leverage: media2_configuration_spec.py from task 3.16p, Media2 service helpers, device fixtures_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Media2 Compliance Engineer with expertise in stream configuration | Task: Implement validate_media2_configuration_compliance to execute spec workflows covering profile creation, Get/SetConfigurations (video/audio/metadata), transport protocols, multicast/unicast toggles, metadata channels, and negative cases; ensure capability validation and proper cleanup | Restrictions: Support Digest/WS-UsernameToken authentication, handle conditional features (audio, metadata, ROI), annotate ComplianceResult with spec IDs and severity, restore original configurations | _Leverage: Existing media validation utilities, compliance reporting_ | _Requirements: 3.1 - Media2 configuration compliance coverage_ | Success: Validation produces detailed ComplianceResult entries per Media2 spec scenario and restores DUT to baseline | Instructions: Mark task in-progress [-], implement Media2 configuration validator, mark complete [x]_

- [ ] 3.16r. Create Profile S Media2 configuration compliance tests
  - Files: e2e/tests/compliance/test_profile_s_media2_configuration.py
  - Description: Implement pytest suite executing Profile S Media2 configuration validator
  - Purpose: Automate Media2 Configuration Device Test Specification coverage for Profile S
  - _Leverage: validate_media2_configuration_compliance from task 3.16q, Media2 spec metadata, compliance reporting utilities_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Media2 Test Engineer with expertise in stream configuration | Task: Create test_profile_s_media2_configuration.py with pytest markers (@pytest.mark.integration, @pytest.mark.media2) executing Media2 configuration compliance flows, asserting ComplianceResult mapping to spec IDs, verifying cleanup of created profiles/configurations, and integrating results into compliance reports | Restrictions: Ensure tests are idempotent, provide fixtures for baseline configuration snapshots, skip gracefully when Media2 service unsupported | _Leverage: pytest fixtures, compliance reporting system, Media2 spec metadata module_ | _Requirements: 3.1 - Automated Media2 configuration coverage_ | Success: Test suite produces actionable Media2 configuration compliance results tied to spec references | Instructions: Mark task in-progress [-], implement Media2 configuration tests, mark complete [x]_

- [ ] 3.16s. Parse Media Configuration Device spec artifacts
  - Files: e2e/utils/media_configuration_spec.py
  - Description: Extract ONVIF Media Configuration Device Test Specification sections into structured metadata
  - Purpose: Provide reusable mapping for legacy Media service configuration test cases (profiles, streams, video/audio settings, PTZ configuration)
  - _Leverage: testspecs/device/ONVIF_Media_Configuration_Device_Test_Specification.xml_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Specification Analyst with expertise in Media services | Task: Build media_configuration_spec.py that parses the Media Configuration Device Test Specification, capturing section/test IDs for profile creation/deletion, Get/SetVideoEncoderConfiguration, Get/SetAudioEncoderConfiguration, multicast/unicast transport, snapshot, PTZ configuration linkage, and error cases; record capability prerequisites, helper procedures, parameter ranges, and expected SOAP faults | Restrictions: Use secure XML parsing (defusedxml or hardened lxml), expose typed dataclasses with cached content, document spec version constants | _Leverage: Parsing patterns from media2_configuration_spec.py and base_device_spec.py, ComplianceResult data structures_ | _Requirements: 3.1 - Media configuration specification metadata foundation_ | Success: Module returns deterministic structures mapping spec scenarios to operations, parameters, and pass/fail expectations, ready for validator consumption | Instructions: Mark task in-progress [-], implement Media spec parser, mark complete [x]_

- [ ] 3.16t. Implement Profile S Media configuration validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_media_configuration_compliance method leveraging Media spec metadata
  - Purpose: Ensure Profile S devices satisfy legacy Media configuration requirements
  - _Leverage: media_configuration_spec.py from task 3.16s, Media service helpers, device fixtures_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Media Compliance Engineer with expertise in ONVIF streaming | Task: Implement validate_media_configuration_compliance executing spec workflows covering profile management, video/audio encoder settings, multicast/unicast transport, RTP/RTSP config, snapshot, PTZ linkage, and negative scenarios; ensure capability validation and configuration rollback | Restrictions: Support Digest/WS-UsernameToken authentication, handle conditional features (audio/PTZ), annotate ComplianceResult with spec IDs and severity, restore original profiles/configurations | _Leverage: Existing media validator utilities, compliance reporting framework_ | _Requirements: 3.1 - Media configuration compliance coverage_ | Success: Validation produces detailed ComplianceResult entries per Media spec scenario and restores DUT to baseline | Instructions: Mark task in-progress [-], implement Media configuration validator, mark complete [x]_

- [ ] 3.16u. Create Profile S Media configuration compliance tests
  - Files: e2e/tests/compliance/test_profile_s_media_configuration.py
  - Description: Implement pytest suite executing Profile S Media configuration validator
  - Purpose: Automate Media Configuration Device Test Specification coverage for Profile S
  - _Leverage: validate_media_configuration_compliance from task 3.16t, Media spec metadata, compliance reporting utilities_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Media Test Engineer with expertise in streaming configuration | Task: Create test_profile_s_media_configuration.py with pytest markers (@pytest.mark.integration, @pytest.mark.media) executing Media configuration compliance flows, asserting ComplianceResult mapping to spec IDs, verifying cleanup of created profiles/configurations, and integrating results into compliance reports | Restrictions: Ensure tests are idempotent, provide fixtures for baseline snapshots, skip gracefully when Media service unsupported | _Leverage: pytest fixtures, compliance reporting system, Media spec metadata module_ | _Requirements: 3.1 - Automated Media configuration coverage_ | Success: Test suite produces actionable Media configuration compliance results tied to spec references | Instructions: Mark task in-progress [-], implement Media configuration tests, mark complete [x]_

- [ ] 3.16v. Parse PTZ Device spec artifacts
  - Files: e2e/utils/ptz_device_spec.py
  - Description: Extract ONVIF PTZ Device Test Specification v20.06 sections into structured metadata
  - Purpose: Provide reusable mapping for PTZ test cases (continuous/relative/absolute move, presets, tours, guard tours, node capabilities)
  - _Leverage: testspecs/device/ONVIF_PTZ_Device_Test_Specification.xml_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Specification Analyst with expertise in PTZ services | Task: Build ptz_device_spec.py that parses the PTZ Device Test Specification, capturing section/test IDs for Get/SetConfigurations, Move operations, Stop, SetHomePosition/GoToHomePosition, presets (Set/Get/Goto/Remove), guard tours, speed/limit validation, status polling, and fault scenarios; record capability prerequisites, helper procedures, parameter ranges, and expected SOAP faults/events | Restrictions: Use secure XML parsing (defusedxml or hardened lxml), expose typed dataclasses with cached content, document spec version constants | _Leverage: Parsing patterns from media_configuration_spec.py and base_device_spec.py, ComplianceResult data structures_ | _Requirements: 3.1 - PTZ specification metadata foundation_ | Success: Module returns deterministic structures mapping spec scenarios to operations, parameters, capability requirements, and pass/fail expectations ready for validator use | Instructions: Mark task in-progress [-], implement PTZ spec parser, mark complete [x]_

- [ ] 3.16w. Implement Profile S PTZ device validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_ptz_device_compliance method leveraging PTZ spec metadata
  - Purpose: Ensure Profile S devices with PTZ support satisfy PTZ Device Test Specification requirements
  - _Leverage: ptz_device_spec.py from task 3.16v, PTZ service helpers, device fixtures_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: PTZ Compliance Engineer with expertise in ONVIF PTZ operations | Task: Implement validate_ptz_device_compliance to execute spec workflows covering Get/SetConfigurations, absolute/relative/continuous moves, velocity limits, Stop, presets (create/goto/delete), home position, guard tours, status polling, and negative cases; ensure capability validation and state rollback | Restrictions: Support Digest/WS-UsernameToken authentication, handle conditional PTZ support (skip gracefully when PTZ absent), annotate ComplianceResult with spec IDs and severity, restore PTZ state after tests | _Leverage: Existing PTZ validator utilities, compliance reporting framework_ | _Requirements: 3.1 - PTZ Device compliance coverage_ | Success: Validation produces detailed ComplianceResult entries per PTZ spec scenario and restores DUT to baseline | Instructions: Mark task in-progress [-], implement PTZ validator, mark complete [x]_

- [ ] 3.16x. Create Profile S PTZ device compliance tests
  - Files: e2e/tests/compliance/test_profile_s_ptz_device.py
  - Description: Implement pytest suite executing Profile S PTZ validator
  - Purpose: Automate PTZ Device Test Specification coverage for Profile S
  - _Leverage: validate_ptz_device_compliance from task 3.16w, PTZ spec metadata, compliance reporting utilities_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF PTZ Test Engineer with expertise in PTZ validation | Task: Create test_profile_s_ptz_device.py with pytest markers (@pytest.mark.integration, @pytest.mark.ptz) executing PTZ compliance flows, asserting ComplianceResult mapping to spec IDs, validating guard tours/preset behavior, and integrating results into compliance reports | Restrictions: Ensure tests are idempotent by restoring PTZ presets/home/guard tours, provide fixtures for safe PTZ ranges, skip gracefully when PTZ unsupported | _Leverage: pytest fixtures, compliance reporting system, PTZ spec metadata module_ | _Requirements: 3.1 - Automated PTZ Device coverage_ | Success: Test suite produces actionable PTZ compliance results tied to spec references | Instructions: Mark task in-progress [-], implement PTZ tests, mark complete [x]_

- [ ] 3.16y. Parse Profiles Conformance spec artifacts
  - Files: e2e/utils/profiles_conformance_spec.py
  - Description: Extract ONVIF Profiles Conformance Device Test Specification sections into structured metadata for Profiles S and T
  - Purpose: Provide canonical mapping between profile requirements and underlying service test cases
  - _Leverage: testspecs/device/ONVIF_Profiles_Conformance_Device_Test_Specification.xml_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Specification Analyst with expertise in profile conformance | Task: Build profiles_conformance_spec.py that parses the Profiles Conformance Device Test Specification, capturing section/test IDs for Profile S and Profile T (device, media, PTZ, imaging, security, eventing, analytics, recording), their prerequisite capabilities, referenced service specs, and pass/fail criteria; expose typed dataclasses tying each profile requirement to the corresponding validator modules | Restrictions: Use secure XML parsing (defusedxml or hardened lxml), cache parsed data, document spec version constants, ensure lookups support cross-profile reuse | _Leverage: Parsing patterns from base_device_spec.py and credential_device_spec.py, ComplianceResult data contracts_ | _Requirements: 3.1 - Profiles conformance metadata foundation_ | Success: Module returns deterministic structures mapping profile requirements to validation hooks and spec references ready for validator consumption | Instructions: Mark task in-progress [-], implement profile spec parser, mark complete [x]_

- [ ] 3.16z. Implement Profile S profiles conformance aggregation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_profile_s_conformance method orchestrating profile-wide checks using profiles_conformance_spec metadata
  - Purpose: Correlate service-level validation results against Profiles Conformance specification requirements
  - _Leverage: profiles_conformance_spec.py from task 3.16y, existing Profile S validators (device, media, imaging, PTZ, credential, feature discovery, authentication, performance)_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile S Conformance Engineer with expertise in end-to-end validation | Task: Implement validate_profile_s_conformance to pull requirement definitions from profiles_conformance_spec, invoke or reference the relevant validator modules, aggregate ComplianceResult data, flag missing coverage, and emit per-requirement diagnostics (including spec IDs, requirement text, severity, and remediation hints); ensure the method enforces prerequisite capability checks and differentiates not-applicable vs failure | Restrictions: Reuse cached validation outputs where possible, avoid duplicating service-level logic, support configuration for optional features, restore DUT state after orchestrated checks | _Leverage: ComplianceResult aggregation utilities, existing validator registry, reporting framework_ | _Requirements: 3.1 - Profile S conformance coverage_ | Success: Aggregated result maps every Profile S requirement to pass/fail/skipped status with detailed context and integrates with reporting | Instructions: Mark task in-progress [-], implement Profile S conformance aggregation, mark complete [x]_

- [ ] 3.16aa. Parse Provisioning Device spec artifacts
  - Files: e2e/utils/provisioning_device_spec.py
  - Description: Extract ONVIF Provisioning Device Test Specification v18.06 sections into structured metadata
  - Purpose: Provide reusable mapping for provisioning service test cases (certificate enrollment, key update, credential synchronization)
  - _Leverage: testspecs/device/ONVIF_Provisioning_Device_Test_Specification.xml_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Specification Analyst with expertise in provisioning workflows | Task: Build provisioning_device_spec.py that parses the Provisioning Device Test Specification, capturing section/test IDs for provisioning service discovery, certificate enrollment (PKCS#10, EST), key/certificate update, provisioning sessions, credential synchronization, error handling, and helper procedures; record capability prerequisites (supported enrollment modes, security levels), expected SOAP faults, and cleanup requirements | Restrictions: Use secure XML parsing (defusedxml or hardened lxml), expose typed dataclasses with cached content, document spec version constants, and ensure data structures align with validator consumption | _Leverage: Parsing patterns from credential_device_spec.py and profiles_conformance_spec.py, ComplianceResult data contracts_ | _Requirements: 3.1 - Provisioning specification metadata foundation_ | Success: Module returns deterministic structures mapping provisioning test cases to operations, parameters, capabilities, and pass/fail expectations ready for validators | Instructions: Mark task in-progress [-], implement provisioning spec parser, mark complete [x]_

- [ ] 3.16ab. Implement Profile S provisioning validation
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_provisioning_service method leveraging provisioning spec metadata
  - Purpose: Ensure Profile S devices supporting provisioning meet Provisioning Device Test Specification requirements
  - _Leverage: provisioning_device_spec.py from task 3.16aa, security utilities, certificate management helpers_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Provisioning Compliance Engineer with expertise in ONVIF security provisioning | Task: Implement validate_provisioning_service to execute spec workflows covering provisioning service discovery, enrollment modes (CSR/EST), certificate/key rollout, credential synchronization, provisioning session management, and negative cases; ensure capability validation, secure transport usage, and proper cleanup of issued credentials | Restrictions: Support Digest/WS-UsernameToken and TLS requirements, reuse security fixtures, annotate ComplianceResult with spec IDs and severity, skip gracefully if provisioning unsupported | _Leverage: Security configuration utilities, credential validators, compliance reporting framework_ | _Requirements: 3.1 - Provisioning compliance coverage_ | Success: Validation produces detailed ComplianceResult entries per provisioning scenario, restores DUT to baseline state, and differentiates unsupported vs failing features | Instructions: Mark task in-progress [-], implement provisioning validator, mark complete [x]_

- [ ] 3.16ac. Create Profile S Provisioning Device compliance tests
  - Files: e2e/tests/compliance/test_profile_s_provisioning_device.py
  - Description: Implement pytest suite executing Profile S provisioning validator
  - Purpose: Automate Provisioning Device Test Specification coverage for Profile S
  - _Leverage: validate_provisioning_service from task 3.16ab, provisioning spec metadata, compliance reporting utilities_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Provisioning Test Engineer with expertise in certificate management | Task: Create test_profile_s_provisioning_device.py with pytest markers (@pytest.mark.integration, @pytest.mark.provisioning) executing provisioning compliance flows, asserting ComplianceResult mapping to spec IDs, verifying certificate issuance/rotation, and integrating results into compliance reports | Restrictions: Ensure tests are idempotent by revoking/cleaning issued credentials, provide fixtures for CA endpoints, skip gracefully when provisioning service unsupported | _Leverage: pytest fixtures, compliance reporting system, provisioning spec metadata module_ | _Requirements: 3.1 - Automated Provisioning Device coverage_ | Success: Test suite produces actionable provisioning compliance results tied to spec references | Instructions: Mark task in-progress [-], implement provisioning tests, mark complete [x]_

- [ ] 3.16ad. Parse Real-Time Streaming Device spec artifacts
  - Files: e2e/utils/real_time_streaming_spec.py
  - Description: Extract ONVIF Real-Time Streaming Device Test Specification sections into structured metadata
  - Purpose: Provide reusable mapping for RTSP/RTP compliance checkpoints, SDP requirements, and streaming diagnostics
  - _Leverage: testspecs/device/ONVIF_Real_Time_Streaming_Device_Test_Specification.xml_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Streaming Specification Analyst with expertise in RTSP/RTP | Task: Build real_time_streaming_spec.py that parses the Real-Time Streaming Device Test Specification, capturing section/test IDs for OPTIONS/DESCRIBE/SETUP/PLAY/TEARDOWN flows, SDP validation, RTP continuity, multicast/unicast behavior, authentication paths, timing tolerances, and helper procedures; record capability prerequisites and expected fault handling | Restrictions: Use secure XML parsing (defusedxml or hardened lxml), expose typed dataclasses with cached content, document spec version constants, align metadata with stream validator consumption | _Leverage: Parsing patterns from feature_discovery_spec.py and media2_configuration_spec.py, ComplianceResult data structures_ | _Requirements: 3.1 - Real-Time Streaming specification metadata foundation_ | Success: Module returns deterministic structures mapping streaming test cases to operations, parameters, tolerances, and pass/fail expectations ready for validators | Instructions: Mark task in-progress [-], implement streaming spec parser, mark complete [x]_

- [ ] 3.16ae. Integrate Real-Time Streaming spec into Profile S stream validator
  - Files: e2e/utils/profile_s_stream_validator.py (extend)
  - Description: Incorporate spec metadata for handshake, SDP, RTP, and error handling checks
  - Purpose: Align streaming validation logic with Real-Time Streaming Device Test Specification criteria
  - _Leverage: real_time_streaming_spec.py from task 3.16ad, existing handshake/SDP/RTP validation methods_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: RTSP Compliance Engineer with expertise in ONVIF streaming | Task: Update ProfileSStreamValidator to consume spec metadata, enforce per-test tolerances (latency, packet loss thresholds), ensure authentication coverage, and annotate ComplianceResult entries with spec IDs while reusing existing validation steps | Restrictions: Avoid duplicating logic, support configurable stream profiles, handle multicast/unicast requirements, maintain current diagnostics structure | _Leverage: Current stream validator infrastructure, compliance reporting utilities_ | _Requirements: 3.1 - Real-Time Streaming compliance alignment_ | Success: Streaming validator outputs link directly to spec requirements with detailed pass/fail context and performance metrics | Instructions: Mark task in-progress [-], integrate spec metadata into validator, mark complete [x]_

- [ ] 3.16af. Update Profile S streaming compliance tests
  - Files: e2e/tests/compliance/test_profile_s_streaming.py (create or extend)
  - Description: Ensure test suite references Real-Time Streaming spec metadata and spec-linked assertions
  - Purpose: Automate Real-Time Streaming Device Test Specification coverage for Profile S streams
  - _Leverage: ProfileSStreamValidator updates from task 3.16ae, streaming spec metadata, compliance reporting utilities_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Streaming Test Engineer with expertise in RTSP validation | Task: Enhance test_profile_s_streaming.py to drive updated streaming validator, expose pytest markers (@pytest.mark.integration, @pytest.mark.streaming), surface spec IDs in assertion messages, and parameterize multicast/unicast scenarios | Restrictions: Maintain idempotent behavior, reuse cached handshake/SDP data when possible, support skip for unsupported features | _Leverage: pytest fixtures, compliance reporting system, streaming spec metadata module_ | _Requirements: 3.1 - Automated Real-Time Streaming coverage_ | Success: Test suite produces actionable streaming compliance results tied to spec references and handles optional streaming capabilities gracefully | Instructions: Mark task in-progress [-], update streaming tests, mark complete [x]_

- [ ] 3.16ag. Parse Real-Time Streaming using Media2 spec artifacts
  - Files: e2e/utils/real_time_streaming_media2_spec.py
  - Description: Extract ONVIF Real-Time Streaming using Media2 Device Test Specification sections into structured metadata
  - Purpose: Provide reusable mapping for Media2 streaming (WS APIs, SRTP/RTP, metadata tracks) compliance checkpoints
  - _Leverage: testspecs/device/ONVIF_Real_Time_Streaming_using_Media2_Device_Test_Specification.xml_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Media2 Streaming Specification Analyst with expertise in ONVIF streaming | Task: Build real_time_streaming_media2_spec.py that parses the Media2 streaming spec, capturing section/test IDs for Media2 service negotiation, SetStreamingUri, Start/StopLive, SRTP/RTSP/WS streaming, SDP requirements, RTP/metadata continuity, authentication modes, and helper procedures; record capability prerequisites and expected faults | Restrictions: Use secure XML parsing (defusedxml or hardened lxml), expose typed dataclasses with cached content, document spec version constants, align metadata structure with Media2 streaming validator needs | _Leverage: Parsing patterns from real_time_streaming_spec.py and media2_configuration_spec.py, ComplianceResult data structures_ | _Requirements: 3.1 - Media2 streaming specification metadata foundation_ | Success: Module returns deterministic structures mapping Media2 streaming test cases to operations, parameters, tolerances, and pass/fail expectations ready for validators | Instructions: Mark task in-progress [-], implement Media2 streaming spec parser, mark complete [x]_

- [ ] 3.16ah. Implement Profile S Media2 streaming validation
  - Files: e2e/utils/profile_s_stream_validator_media2.py (create) or extend existing stream validator with Media2 flows
  - Description: Add validation routines leveraging Media2 streaming spec metadata for Profile S
  - Purpose: Ensure Profile S devices supporting Media2 streaming satisfy Media2 Real-Time Streaming spec requirements
  - _Leverage: real_time_streaming_media2_spec.py from task 3.16ag, Media2 configuration utilities, existing streaming helpers_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Media2 Streaming Compliance Engineer with expertise in ONVIF streaming | Task: Implement Media2 streaming validator to exercise Media2 Start/StopStreaming flows, validate SDP generation, SRTP/RTSP outputs, metadata channel continuity, latency/packet loss tolerances, and security modes per spec; annotate ComplianceResult entries with spec IDs and ensure cleanup | Restrictions: Support Digest/WS-UsernameToken authentication, optional SRTP/multicast handling, reuse shared streaming infrastructure, and restore baseline configurations | _Leverage: Media2 configuration validator, compliance reporting framework_ | _Requirements: 3.1 - Media2 streaming compliance coverage_ | Success: Validator produces detailed ComplianceResult records for Media2 streaming scenarios with spec-aligned diagnostics | Instructions: Mark task in-progress [-], implement Media2 streaming validator, mark complete [x]_

- [ ] 3.16ai. Create Profile S Media2 streaming compliance tests
  - Files: e2e/tests/compliance/test_profile_s_media2_streaming.py
  - Description: Implement pytest suite executing Profile S Media2 streaming validator
  - Purpose: Automate Real-Time Streaming using Media2 Device Test Specification coverage for Profile S
  - _Leverage: Media2 streaming validator from task 3.16ah, Media2 streaming spec metadata, compliance reporting utilities_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Media2 Streaming Test Engineer with expertise in ONVIF streaming | Task: Create test_profile_s_media2_streaming.py with pytest markers (@pytest.mark.integration, @pytest.mark.media2_streaming) to drive Media2 streaming validator, verify SRTP/RTSP flows, assert ComplianceResult mapping to spec IDs, and aggregate results for reporting | Restrictions: Maintain idempotent behavior, provide fixtures for stream endpoint configuration, skip gracefully when Media2 streaming unsupported | _Leverage: pytest fixtures, compliance reporting system, Media2 streaming spec metadata module_ | _Requirements: 3.1 - Automated Media2 streaming coverage_ | Success: Test suite delivers actionable Media2 streaming compliance results tied to spec references | Instructions: Mark task in-progress [-], implement Media2 streaming tests, mark complete [x]_

- [ ] 3.17. Create comprehensive Profile S compliance test
  - Files: e2e/tests/compliance/test_profile_s_complete.py
  - Description: Create complete Profile S compliance test using all validators
  - Purpose: Validate every Profile S requirement from Profiles Conformance spec with consolidated reporting
  - _Leverage: Complete Profile S validator and profile conformance aggregator from tasks 3.3-3.16z_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Compliance Test Engineer with expertise in comprehensive validation | Task: Create test_profile_s_complete.py that drives validate_profile_s_conformance(), ensures all prerequisite validators run, and produces a spec-referenced compliance report following requirement 3.1 | Restrictions: Ensure pytest markers distinguish full conformance runs, surface per-requirement diagnostics using spec IDs, and fail fast on critical gaps while still collecting remaining results | _Leverage: ONVIFProfileSValidator aggregation from tasks 3.3-3.16z, device fixtures_ | _Requirements: 3.1 - Complete Profile S compliance testing_ | Success: Test run maps every Profile S requirement to pass/fail/skipped with detailed spec references and exports summary for reporting | Instructions: Mark task in-progress [-], create comprehensive compliance tests, mark complete [x]_

- [ ] 3.18. Create Profile T validator foundation
  - Files: e2e/utils/profile_t_validator.py
  - Description: Create base ONVIFProfileTValidator class with basic structure
  - Purpose: Establish foundation for Profile T compliance validation
  - _Leverage: Profile S validator patterns from task 3.3_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Profile T Specialist with expertise in ONVIF Profile T compliance requirements | Task: Create profile_t_validator.py with base ONVIFProfileTValidator class following requirement 3.2 | Restrictions: Only create class structure and basic methods, do not implement full validation logic yet, ensure proper inheritance from base validator patterns | _Leverage: Profile S validator patterns from task 3.3, ComplianceResult from task 3.1_ | _Requirements: 3.2 - Profile T compliance validation foundation_ | Success: ONVIFProfileTValidator class exists with basic structure and inheritance | Instructions: Mark task in-progress [-], create Profile T validator foundation, mark complete [x]_

- [ ] 3.19. Implement Profile T enhanced video streaming validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_enhanced_video_streaming method for Profile T streaming requirements
  - Purpose: Validate ONVIF Profile T enhanced video streaming capabilities
  - _Leverage: Media service streaming patterns_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Enhanced Video Streaming Expert with expertise in Profile T streaming requirements | Task: Implement validate_enhanced_video_streaming method in Profile T validator following requirement 3.2 | Restrictions: Only implement enhanced streaming validation, test advanced streaming capabilities, validate Profile T specific features | _Leverage: Media service streaming patterns, GetStreamUri operations_ | _Requirements: 3.2 - Profile T enhanced video streaming_ | Success: Enhanced video streaming validation checks Profile T specific streaming capabilities | Instructions: Mark task in-progress [-], implement enhanced streaming validation, mark complete [x]_

- [ ] 3.20. Implement Profile T PTZ support validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_ptz_support method for PTZ compliance
  - Purpose: Validate ONVIF Profile T PTZ control requirements
  - _Leverage: PTZ service patterns from existing tests_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: PTZ Control Expert with expertise in ONVIF PTZ validation | Task: Implement validate_ptz_support method in Profile T validator following requirement 3.2 | Restrictions: Only implement PTZ validation, test PTZ capabilities and operations, handle conditional PTZ support properly | _Leverage: PTZ service patterns from existing e2e tests_ | _Requirements: 3.2 - Profile T PTZ support_ | Success: PTZ validation checks PTZ capabilities and control operations correctly | Instructions: Mark task in-progress [-], implement PTZ validation, mark complete [x]_

- [ ] 3.21. Implement Profile T imaging service validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_imaging_service method for imaging compliance
  - Purpose: Validate ONVIF Profile T imaging service requirements
  - _Leverage: Imaging service patterns from existing tests_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Imaging Service Expert with expertise in ONVIF imaging validation | Task: Implement validate_imaging_service method in Profile T validator following requirement 3.2 | Restrictions: Only implement imaging service validation, test imaging capabilities and settings, validate image quality controls | _Leverage: Imaging service patterns from existing e2e tests_ | _Requirements: 3.2 - Profile T imaging service_ | Success: Imaging service validation checks imaging capabilities and controls correctly | Instructions: Mark task in-progress [-], implement imaging validation, mark complete [x]_

- [ ] 3.22. Create basic Profile T compliance test
  - Files: e2e/tests/compliance/test_profile_t_basic.py
  - Description: Create basic Profile T compliance test for core features
  - Purpose: Test core Profile T features (streaming, PTZ, imaging)
  - _Leverage: Profile T validator from previous tasks_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Profile T Test Engineer with expertise in Profile T testing | Task: Create test_profile_t_basic.py with basic Profile T compliance tests following requirement 3.2 | Restrictions: Only test core Profile T features, use proper pytest markers, ensure clear test reporting with conditional feature handling | _Leverage: ONVIFProfileTValidator from previous tasks, device fixtures_ | _Requirements: 3.2 - Profile T compliance testing_ | Success: Profile T basic compliance tests validate core features with proper conditional handling | Instructions: Mark task in-progress [-], create basic Profile T tests, mark complete [x]_

- [ ] 3.22a. Implement Profile T authentication profile validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_authentication_profiles method incorporating Authentication Behavior spec requirements and Profile T security extensions
  - Purpose: Validate Profile T credential policies, including advanced authentication modes and metadata handling
  - _Leverage: authentication_profile_spec.py from task 3.16a, Profile S validator patterns, WS-UsernameToken utilities_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T Security Specialist with expertise in ONVIF authentication | Task: Extend Profile T validator to reuse Authentication Behavior test flows, add WS-UsernameToken mandatory checks, validate security level transitions, and ensure metadata channel authentication alignment per Profile T requirements | Restrictions: Respect conditional features (e.g., metadata streaming), reuse cleanup helpers, propagate ComplianceResult annotations with spec IDs | _Leverage: Profile S authentication validator, event handling utilities, security configuration helpers_ | _Requirements: 3.2 - Profile T authentication behavior compliance_ | Success: Validation method covers all Authentication Behavior spec flows plus Profile T security enhancements and reports comprehensive diagnostics | Instructions: Mark task in-progress [-], implement Profile T authentication validation, mark complete [x]_

- [ ] 3.22b. Create Profile T Authentication Behavior compliance tests
  - Files: e2e/tests/compliance/test_profile_t_authentication_behavior.py
  - Description: Implement pytest suite validating Profile T authentication behavior and security upgrades
  - Purpose: Execute Authentication Behavior Device Test Specification scenarios under Profile T constraints
  - _Leverage: Profile T validator method from task 3.22a, spec metadata module, authentication fixtures_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Profile T Compliance Tester with expertise in secure video systems | Task: Build test_profile_t_authentication_behavior.py using pytest markers (@pytest.mark.integration, @pytest.mark.authentication, @pytest.mark.profile_t) to run WS-UsernameToken, Digest, and token rotation scenarios, assert ComplianceResult mapping to spec IDs, and ensure metadata stream authentication is validated | Restrictions: Ensure graceful skip when Authentication Profile feature unavailable, coordinate teardown of created authentication profiles and security levels, integrate with compliance reporting | _Leverage: pytest fixtures, Profile T validator, compliance reporting utilities_ | _Requirements: 3.2 - Automated Profile T Authentication Behavior coverage_ | Success: Test suite validates Authentication Behavior spec flows for Profile T, produces actionable failure diagnostics, and feeds compliance reports | Instructions: Mark task in-progress [-], implement Profile T authentication tests, mark complete [x]_

- [ ] 3.22c. Implement Profile T base device validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_base_device_compliance method adapting base device spec flows to Profile T security requirements
  - Purpose: Ensure Profile T devices satisfy Base Device Test Specification with enhanced security checks
  - _Leverage: base_device_spec.py from task 3.16d, Profile S validator patterns, WS-UsernameToken enforcement_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T Device Compliance Engineer with expertise in secure ONVIF operations | Task: Extend Profile T validator to execute base device spec flows (device info, capabilities, time sync, network configuration, user lifecycle, system controls) while verifying Profile T security constraints such as TLS requirements, metadata channel authentication, and stricter SOAP fault handling | Restrictions: Reuse shared helpers, ensure teardown/cleanup, annotate ComplianceResult entries with spec IDs and Profile T security notes, support dual-stack network scenarios if applicable | _Leverage: base_device_spec metadata, security configuration utilities, compliance reporting_ | _Requirements: 3.2 - Profile T base device compliance_ | Success: Validation method reports spec-aligned diagnostics, confirms security behaviors, and restores DUT to baseline | Instructions: Mark task in-progress [-], implement Profile T base device validation, mark complete [x]_

- [ ] 3.22d. Create Profile T Base Device compliance tests
  - Files: e2e/tests/compliance/test_profile_t_base_device.py
  - Description: Implement pytest suite executing Profile T base device validator flows
  - Purpose: Automate Base Device Test Specification coverage for Profile T deployments
  - _Leverage: validate_base_device_compliance from task 3.22c, base_device_spec metadata, compliance reporting utilities_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T Test Engineer with expertise in secure device validation | Task: Create test_profile_t_base_device.py with pytest markers (@pytest.mark.integration, @pytest.mark.base_device, @pytest.mark.profile_t) executing Profile T base device checks, asserting ComplianceResult contents, verifying security annotations, and integrating results into reports | Restrictions: Handle conditional features (metadata streams, TLS enforcement) gracefully, ensure environment cleanup, align failure messaging with spec terminology | _Leverage: pytest fixtures, compliance reporting system, spec metadata module_ | _Requirements: 3.2 - Automated Profile T Base Device coverage_ | Success: Test suite produces actionable base device compliance results for Profile T, feeding combined reports and handling unsupported features robustly | Instructions: Mark task in-progress [-], implement Profile T base device tests, mark complete [x]_

- [ ] 3.22e. Implement Profile T credential service validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_credential_service method extending Profile S flows with Profile T security requirements
  - Purpose: Ensure Profile T implementations satisfy Credential Device Test Specification with enhanced security controls
  - _Leverage: credential_device_spec.py from task 3.16g, Profile S credential validator patterns, WS-UsernameToken enforcement_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T Credential Compliance Engineer with expertise in secure credential management | Task: Extend Profile T validator to run credential spec flows, validate TLS/metadata channel authentication, enforce security level constraints, and verify credential state transitions under Profile T policies | Restrictions: Reuse shared helper utilities, ensure cleanup of credential artifacts, annotate ComplianceResult entries with spec IDs and Profile T security notes, support conditional features (whitelist/blacklist, suspension durations) | _Leverage: credential spec metadata, security configuration helpers, compliance reporting_ | _Requirements: 3.2 - Profile T credential compliance_ | Success: Validation method reports detailed diagnostics for credential operations and security checks, restoring DUT to baseline | Instructions: Mark task in-progress [-], implement Profile T credential validation, mark complete [x]_

- [ ] 3.22f. Create Profile T Credential Device compliance tests
  - Files: e2e/tests/compliance/test_profile_t_credential_device.py
  - Description: Implement pytest suite executing Profile T credential validator workflows
  - Purpose: Automate Credential Device Test Specification coverage for Profile T
  - _Leverage: validate_credential_service from task 3.22e, credential spec metadata, compliance reporting utilities_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T Credential Test Engineer with expertise in secure provisioning | Task: Create test_profile_t_credential_device.py using pytest markers (@pytest.mark.integration, @pytest.mark.credential, @pytest.mark.profile_t) to run Profile T credential compliance flows, assert ComplianceResult outputs, verify security annotations, and integrate with reporting | Restrictions: Handle optional capabilities gracefully, ensure identifier/credential cleanup, align failure messaging with spec terminology | _Leverage: pytest fixtures, compliance reporting system, credential spec metadata module_ | _Requirements: 3.2 - Automated Profile T Credential Device coverage_ | Success: Test suite delivers actionable Profile T credential compliance results tied to spec IDs | Instructions: Mark task in-progress [-], implement Profile T credential tests, mark complete [x]_

- [ ] 3.22g. Implement Profile T feature discovery validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_feature_discovery method aligning Device Feature Discovery spec with Profile T security requirements
  - Purpose: Ensure Profile T devices advertise mandatory features with enhanced security considerations
  - _Leverage: feature_discovery_spec.py from task 3.16j, Profile S discovery validator patterns, security utilities_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T Feature Discovery Specialist with expertise in ONVIF security | Task: Extend Profile T validator to evaluate service feature tables including Profile T-specific features (Media2 metadata, advanced security, ClientSuppliedToken, TLS versions), enforce security annotations, and differentiate optional vs mandatory Profile T features | Restrictions: Reuse cached discovery data, annotate ComplianceResult entries with spec IDs and security notes, ensure coverage for metadata channels and advanced security settings, support conditional service availability | _Leverage: feature discovery spec metadata, security configuration helpers, compliance reporting_ | _Requirements: 3.2 - Profile T feature discovery compliance_ | Success: Validation reports detailed feature discovery diagnostics with security context and restores DUT baseline | Instructions: Mark task in-progress [-], implement Profile T feature discovery validation, mark complete [x]_

- [ ] 3.22h. Create Profile T Feature Discovery compliance tests
  - Files: e2e/tests/compliance/test_profile_t_feature_discovery.py
  - Description: Implement pytest suite executing Profile T feature discovery validator
  - Purpose: Automate Device Feature Discovery Specification coverage for Profile T
  - _Leverage: validate_feature_discovery from task 3.22g, feature discovery spec metadata, compliance reporting utilities_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T Discovery Test Engineer with expertise in secure feature validation | Task: Create test_profile_t_feature_discovery.py using pytest markers (@pytest.mark.integration, @pytest.mark.discovery, @pytest.mark.profile_t) to execute feature discovery checks, assert ComplianceResult mapping to spec IDs and security annotations, and integrate outputs into compliance reports | Restrictions: Ensure idempotent execution, accommodate optional services, provide fixture hooks for security configuration, reuse cached discovery data | _Leverage: pytest fixtures, compliance reporting system, feature discovery spec metadata module_ | _Requirements: 3.2 - Automated Profile T Feature Discovery coverage_ | Success: Test suite delivers actionable Profile T feature discovery compliance results tied to spec references | Instructions: Mark task in-progress [-], implement Profile T feature discovery tests, mark complete [x]_

- [ ] 3.22i. Implement Profile T imaging device validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_imaging_device_compliance method adapting imaging spec flows to Profile T security and advanced imaging requirements
  - Purpose: Ensure Profile T devices satisfy Imaging Device Test Specification with additional Profile T imaging expectations
  - _Leverage: imaging_device_spec.py from task 3.16m, Profile S imaging validator patterns, advanced imaging utilities_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T Imaging Compliance Engineer with expertise in advanced imaging controls | Task: Extend Profile T validator to execute imaging spec workflows, validate advanced imaging parameters (wide dynamic range, exposure gain, metadata synchronization), enforce security requirements for imaging commands, and report spec-aligned diagnostics | Restrictions: Support Digest/WS-UsernameToken authentication, preserve/restore imaging settings, annotate ComplianceResult with spec IDs and Profile T notes, handle conditional capabilities such as advanced focus modes | _Leverage: imaging spec metadata, security utilities, compliance reporting_ | _Requirements: 3.2 - Profile T imaging compliance_ | Success: Validation produces detailed imaging compliance diagnostics incorporating Profile T-specific checks and restores DUT baseline | Instructions: Mark task in-progress [-], implement Profile T imaging validation, mark complete [x]_

- [ ] 3.22j. Create Profile T Imaging Device compliance tests
  - Files: e2e/tests/compliance/test_profile_t_imaging_device.py
  - Description: Implement pytest suite executing Profile T imaging compliance validator
  - Purpose: Automate Imaging Device Test Specification coverage for Profile T
  - _Leverage: validate_imaging_device_compliance from task 3.22i, imaging spec metadata, compliance reporting utilities_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T Imaging Test Engineer with expertise in advanced imaging validation | Task: Create test_profile_t_imaging_device.py using pytest markers (@pytest.mark.integration, @pytest.mark.imaging, @pytest.mark.profile_t) to execute imaging compliance flows, assert ComplianceResult outputs, verify security annotations, and integrate results into compliance reports | Restrictions: Ensure idempotent execution with fixtures restoring imaging settings/presets, accommodate optional advanced features, reuse metadata from spec parser | _Leverage: pytest fixtures, compliance reporting system, imaging spec metadata module_ | _Requirements: 3.2 - Automated Profile T Imaging Device coverage_ | Success: Test suite delivers actionable imaging compliance results tied to spec IDs and Profile T notes | Instructions: Mark task in-progress [-], implement Profile T imaging tests, mark complete [x]_

- [ ] 3.22k. Implement Profile T Media2 configuration validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_media2_configuration_compliance method aligning Media2 spec flows with Profile T advanced requirements
  - Purpose: Ensure Profile T devices satisfy Media2 configuration spec with enhanced streaming/security expectations
  - _Leverage: media2_configuration_spec.py from task 3.16p, Profile S Media2 validator patterns, advanced streaming utilities_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T Media2 Compliance Engineer with expertise in advanced streaming | Task: Extend Profile T validator to execute Media2 configuration workflows, validate Profile T-specific features (H.265, metadata synchronization, secure transport), enforce security policies, and report spec-aligned diagnostics | Restrictions: Support Digest/WS-UsernameToken authentication, restore Media2 configurations after tests, annotate ComplianceResult with spec IDs and Profile T notes, handle conditional features (audio metadata, ROI) | _Leverage: Media2 spec metadata, security utilities, compliance reporting_ | _Requirements: 3.2 - Profile T Media2 configuration compliance_ | Success: Validation produces detailed diagnostics for Media2 configurations with Profile T security context and restores DUT baseline | Instructions: Mark task in-progress [-], implement Profile T Media2 validator, mark complete [x]_

- [ ] 3.22l. Create Profile T Media2 configuration compliance tests
  - Files: e2e/tests/compliance/test_profile_t_media2_configuration.py
  - Description: Implement pytest suite executing Profile T Media2 configuration validator
  - Purpose: Automate Media2 Configuration Device Test Specification coverage for Profile T
  - _Leverage: validate_media2_configuration_compliance from task 3.22k, Media2 spec metadata, compliance reporting utilities_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T Media2 Test Engineer with expertise in secure streaming | Task: Create test_profile_t_media2_configuration.py using pytest markers (@pytest.mark.integration, @pytest.mark.media2, @pytest.mark.profile_t) to execute Media2 configuration compliance flows, assert ComplianceResult mapping to spec IDs and security notes, and integrate results into compliance reports | Restrictions: Ensure idempotent execution, handle optional features gracefully, reuse baseline snapshots, integrate with reporting pipeline | _Leverage: pytest fixtures, compliance reporting system, Media2 spec metadata module_ | _Requirements: 3.2 - Automated Profile T Media2 configuration coverage_ | Success: Test suite produces actionable Profile T Media2 configuration compliance outcomes tied to spec references | Instructions: Mark task in-progress [-], implement Profile T Media2 tests, mark complete [x]_

- [ ] 3.22m. Implement Profile T Media configuration validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_media_configuration_compliance method aligning legacy Media spec flows with Profile T security requirements
  - Purpose: Ensure Profile T devices satisfy Media configuration spec with additional security/advanced streaming expectations
  - _Leverage: media_configuration_spec.py from task 3.16s, Profile S Media validator patterns, advanced streaming utilities_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T Media Compliance Engineer with expertise in secure streaming | Task: Extend Profile T validator to execute Media configuration workflows, validate Profile T-specific features (metadata, advanced codecs, secure transport), enforce security policies, and report spec-aligned diagnostics | Restrictions: Support Digest/WS-UsernameToken authentication, restore Media configurations after tests, annotate ComplianceResult with spec IDs and Profile T notes, handle conditional features (audio/PTZ) | _Leverage: Media spec metadata, security utilities, compliance reporting_ | _Requirements: 3.2 - Profile T Media configuration compliance_ | Success: Validation produces detailed diagnostics for Media configurations with Profile T security context and restores DUT baseline | Instructions: Mark task in-progress [-], implement Profile T Media validator, mark complete [x]_

- [ ] 3.22n. Create Profile T Media configuration compliance tests
  - Files: e2e/tests/compliance/test_profile_t_media_configuration.py
  - Description: Implement pytest suite executing Profile T Media configuration validator
  - Purpose: Automate Media Configuration Device Test Specification coverage for Profile T
  - _Leverage: validate_media_configuration_compliance from task 3.22m, Media spec metadata, compliance reporting utilities_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T Media Test Engineer with expertise in secure streaming | Task: Create test_profile_t_media_configuration.py using pytest markers (@pytest.mark.integration, @pytest.mark.media, @pytest.mark.profile_t) to execute Media configuration compliance flows, assert ComplianceResult mapping to spec IDs and security annotations, and integrate results into compliance reports | Restrictions: Ensure idempotent execution, accommodate optional features gracefully, reuse baseline snapshots, integrate with reporting pipeline | _Leverage: pytest fixtures, compliance reporting system, Media spec metadata module_ | _Requirements: 3.2 - Automated Profile T Media configuration coverage_ | Success: Test suite produces actionable Profile T Media configuration compliance outcomes tied to spec references | Instructions: Mark task in-progress [-], implement Profile T Media tests, mark complete [x]_

- [ ] 3.22o. Implement Profile T PTZ device validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_ptz_device_compliance method aligning PTZ spec flows with Profile T requirements
  - Purpose: Ensure Profile T devices with PTZ support satisfy PTZ Device Test Specification plus Profile T security considerations
  - _Leverage: ptz_device_spec.py from task 3.16v, Profile S PTZ validator patterns, security utilities_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T PTZ Compliance Engineer with expertise in secure PTZ operations | Task: Extend Profile T validator to execute PTZ spec workflows, validate advanced PTZ requirements (metadata synchronization, secure command handling), enforce security policies, and report spec-aligned diagnostics | Restrictions: Support Digest/WS-UsernameToken authentication, restore PTZ state after tests, annotate ComplianceResult with spec IDs and Profile T notes, handle conditional PTZ support gracefully | _Leverage: PTZ spec metadata, security utilities, compliance reporting_ | _Requirements: 3.2 - Profile T PTZ compliance_ | Success: Validation produces detailed PTZ compliance diagnostics with security context and restores DUT baseline | Instructions: Mark task in-progress [-], implement Profile T PTZ validator, mark complete [x]_

- [ ] 3.22p. Create Profile T PTZ device compliance tests
  - Files: e2e/tests/compliance/test_profile_t_ptz_device.py
  - Description: Implement pytest suite executing Profile T PTZ validator
  - Purpose: Automate PTZ Device Test Specification coverage for Profile T
  - _Leverage: validate_ptz_device_compliance from task 3.22o, PTZ spec metadata, compliance reporting utilities_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T PTZ Test Engineer with expertise in secure PTZ validation | Task: Create test_profile_t_ptz_device.py using pytest markers (@pytest.mark.integration, @pytest.mark.ptz, @pytest.mark.profile_t) executing PTZ compliance flows, asserting ComplianceResult mapping to spec IDs and security annotations, and integrating results into compliance reports | Restrictions: Ensure idempotent execution, accommodate optional PTZ features, reuse baseline state snapshots, integrate with reporting pipeline | _Leverage: pytest fixtures, compliance reporting system, PTZ spec metadata module_ | _Requirements: 3.2 - Automated Profile T PTZ Device coverage_ | Success: Test suite produces actionable Profile T PTZ compliance outcomes tied to spec references | Instructions: Mark task in-progress [-], implement Profile T PTZ tests, mark complete [x]_

- [ ] 3.22q. Implement Profile T profiles conformance aggregation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_profile_t_conformance method orchestrating profile-wide checks using profiles_conformance_spec metadata
  - Purpose: Correlate Profile T requirements from the Profiles Conformance spec with validator outputs and security expectations
  - _Leverage: profiles_conformance_spec.py from task 3.16y, existing Profile T validators (device, media/media2, imaging, PTZ, credential, feature discovery, authentication)_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T Conformance Engineer with expertise in holistic validation | Task: Implement validate_profile_t_conformance to iterate over Profile T requirements, invoke relevant validators, merge ComplianceResult data, enforce security prerequisites (TLS, metadata authentication), and report per-requirement diagnostics with spec IDs and severity while distinguishing not-applicable features | Restrictions: Reuse cached results where possible, avoid duplicate validator execution, support configuration overrides for optional capabilities, restore DUT state after orchestration | _Leverage: ComplianceResult aggregation utilities, security helpers, reporting framework_ | _Requirements: 3.2 - Profile T conformance coverage_ | Success: Aggregated result maps every Profile T requirement to pass/fail/skipped with detailed security context and integrates with reporting | Instructions: Mark task in-progress [-], implement Profile T conformance aggregation, mark complete [x]_

- [ ] 3.22r. Implement Profile T provisioning validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_provisioning_service method aligning provisioning spec flows with Profile T security requirements
  - Purpose: Ensure Profile T devices supporting provisioning meet Provisioning Device Test Specification with advanced security policies
  - _Leverage: provisioning_device_spec.py from task 3.16aa, Profile S provisioning validator patterns, TLS/credential utilities_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T Provisioning Compliance Engineer with expertise in secure provisioning | Task: Extend Profile T validator to execute provisioning spec workflows, enforce Profile T security expectations (mandatory TLS, metadata authentication), validate certificate lifecycle management, and report spec-aligned diagnostics | Restrictions: Reuse security fixtures, ensure cleanup of issued credentials, annotate ComplianceResult with spec IDs and Profile T notes, handle conditional provisioning support gracefully | _Leverage: provisioning spec metadata, security helpers, compliance reporting_ | _Requirements: 3.2 - Profile T provisioning compliance_ | Success: Validation produces detailed provisioning diagnostics with security context and restores DUT baseline | Instructions: Mark task in-progress [-], implement Profile T provisioning validator, mark complete [x]_

- [ ] 3.22s. Create Profile T Provisioning Device compliance tests
  - Files: e2e/tests/compliance/test_profile_t_provisioning_device.py
  - Description: Implement pytest suite executing Profile T provisioning validator
  - Purpose: Automate Provisioning Device Test Specification coverage for Profile T
  - _Leverage: validate_provisioning_service from task 3.22r, provisioning spec metadata, compliance reporting utilities_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T Provisioning Test Engineer with expertise in secure provisioning | Task: Create test_profile_t_provisioning_device.py using pytest markers (@pytest.mark.integration, @pytest.mark.provisioning, @pytest.mark.profile_t) executing provisioning compliance flows, asserting ComplianceResult mapping to spec IDs and security annotations, and integrating results into compliance reports | Restrictions: Ensure idempotent execution by revoking/cleaning issued credentials, support configurable CA endpoints, skip gracefully when provisioning unsupported | _Leverage: pytest fixtures, compliance reporting system, provisioning spec metadata module_ | _Requirements: 3.2 - Automated Profile T provisioning coverage_ | Success: Test suite produces actionable provisioning compliance outcomes tied to spec references and security expectations | Instructions: Mark task in-progress [-], implement provisioning tests, mark complete [x]_

- [ ] 3.22t. Integrate Real-Time Streaming spec into Profile T stream validator
  - Files: e2e/utils/profile_t_stream_validator.py (extend)
  - Description: Align Profile T streaming validation with Real-Time Streaming Device Test Specification using spec metadata
  - Purpose: Ensure Profile T streaming checks meet advanced security and performance requirements defined by the spec
  - _Leverage: real_time_streaming_spec.py from task 3.16ad, existing streaming validation logic_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Secure Streaming Compliance Engineer with expertise in Profile T streaming | Task: Update Profile T stream validator to consume spec metadata, enforce Profile T requirements (TLS, WS-UsernameToken, metadata channels), apply spec tolerances, and annotate ComplianceResult entries with spec IDs | Restrictions: Reuse shared streaming utilities, maintain diagnostic reporting, support optional streaming features, and ensure baseline restoration | _Leverage: Profile S streaming validator patterns, security utilities, compliance reporting_ | _Requirements: 3.2 - Real-Time Streaming compliance alignment_ | Success: Streaming validator outputs link directly to spec requirements with Profile T security context and detailed diagnostics | Instructions: Mark task in-progress [-], integrate spec metadata into Profile T validator, mark complete [x]_

- [ ] 3.22u. Update Profile T streaming compliance tests
  - Files: e2e/tests/compliance/test_profile_t_streaming.py (create or extend)
  - Description: Ensure test suite exercises updated Profile T streaming validator with spec-linked assertions
  - Purpose: Automate Real-Time Streaming Device Test Specification coverage for Profile T streams
  - _Leverage: Profile T streaming validator updates from task 3.22t, streaming spec metadata, compliance reporting utilities_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T Streaming Test Engineer with expertise in secure RTSP validation | Task: Enhance test_profile_t_streaming.py to drive updated streaming validator, configure pytest markers (@pytest.mark.integration, @pytest.mark.streaming, @pytest.mark.profile_t), and surface spec IDs in assertion messages for both authenticated and secure transport scenarios | Restrictions: Maintain idempotent behavior, support optional features (multicast, metadata streaming), reuse cached handshake data when possible | _Leverage: pytest fixtures, compliance reporting system, streaming spec metadata module_ | _Requirements: 3.2 - Automated Real-Time Streaming coverage_ | Success: Test suite produces actionable Profile T streaming compliance results tied to spec references and security annotations | Instructions: Mark task in-progress [-], update streaming tests, mark complete [x]_

- [ ] 3.22v. Integrate Real-Time Streaming using Media2 spec into Profile T stream validator
  - Files: e2e/utils/profile_t_stream_validator_media2.py (create) or extend existing stream validator with Media2 flows
  - Description: Align Profile T Media2 streaming validation with Media2 Real-Time Streaming Device Test Specification using spec metadata
  - Purpose: Ensure Profile T Media2 streaming checks satisfy advanced security/performance requirements defined by the spec
  - _Leverage: real_time_streaming_media2_spec.py from task 3.16ag, Profile T streaming validator infrastructure, security utilities_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T Media2 Streaming Compliance Engineer with expertise in secure ONVIF streaming | Task: Extend Profile T stream validator to execute Media2 streaming flows (Start/Stop, SRTP, metadata channels), enforce spec tolerances, ensure TLS/WS-UsernameToken handling, and annotate ComplianceResult entries with spec IDs | Restrictions: Reuse shared streaming helpers, support optional features (multicast, redundant streaming), maintain detailed diagnostics, and restore baseline state | _Leverage: Media2 configuration validator, compliance reporting framework_ | _Requirements: 3.2 - Media2 streaming compliance alignment_ | Success: Validator provides spec-referenced diagnostics for all Media2 streaming scenarios under Profile T security constraints | Instructions: Mark task in-progress [-], integrate Media2 streaming spec metadata into Profile T validator, mark complete [x]_

- [ ] 3.22w. Create Profile T Media2 streaming compliance tests
  - Files: e2e/tests/compliance/test_profile_t_media2_streaming.py
  - Description: Implement pytest suite executing Profile T Media2 streaming validator
  - Purpose: Automate Real-Time Streaming using Media2 Device Test Specification coverage for Profile T
  - _Leverage: Media2 streaming validator from task 3.22v, Media2 streaming spec metadata, compliance reporting utilities_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Profile T Media2 Streaming Test Engineer with expertise in secure streaming | Task: Create test_profile_t_media2_streaming.py with pytest markers (@pytest.mark.integration, @pytest.mark.media2_streaming, @pytest.mark.profile_t) to run Media2 streaming validator, verify SRTP/RTSP outputs, assert ComplianceResult mapping to spec IDs, and integrate results into compliance reports | Restrictions: Maintain idempotent execution, provide fixtures for CA and stream endpoints, skip gracefully when Media2 streaming unsupported | _Leverage: pytest fixtures, compliance reporting system, Media2 streaming spec metadata module_ | _Requirements: 3.2 - Automated Media2 streaming coverage_ | Success: Test suite delivers actionable Profile T Media2 streaming compliance outcomes tied to spec references and security annotations | Instructions: Mark task in-progress [-], implement Media2 streaming tests, mark complete [x]_

- [ ] 3.23. Create compliance reporting system
  - Files: e2e/utils/compliance_reporting.py
  - Description: Implement comprehensive compliance report generation
  - Purpose: Generate HTML and JSON compliance reports
  - _Leverage: ComplianceResult data structures_
  - _Requirements: 3.4_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Compliance Reporting Engineer with expertise in test reporting and HTML generation | Task: Create compliance_reporting.py with HTML and JSON report generation following requirement 3.4 | Restrictions: Generate detailed compliance reports, include feature-by-feature status, provide clear pass/fail indicators and recommendations | _Leverage: ComplianceResult from task 3.1, HTML template generation, JSON serialization_ | _Requirements: 3.4 - Detailed compliance reporting_ | Success: Compliance reports generate clear HTML and JSON output with comprehensive feature analysis | Instructions: Mark task in-progress [-], create compliance reporting system, mark complete [x]_

## Profile S Optional Tests

- [ ] 3.24. Implement Profile S audio streaming validation (optional)
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_audio_streaming method for optional audio streaming
  - Purpose: Validate ONVIF Profile S audio streaming if supported
  - _Leverage: Audio streaming patterns from media service_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Audio Streaming Expert with expertise in ONVIF audio validation | Task: Implement validate_audio_streaming method for optional Profile S audio streaming following requirement 3.1 | Restrictions: Mark as conditional/optional feature, test audio capabilities if present, handle graceful failure if not supported | _Leverage: Audio streaming patterns from media service tests_ | _Requirements: 3.1 - Profile S optional audio streaming_ | Success: Audio streaming validation properly identifies support and validates if present | Instructions: Mark task in-progress [-], implement audio streaming validation, mark complete [x]_

- [ ] 3.25. Implement Profile S PTZ preset validation (optional)
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_ptz_presets method for optional PTZ preset support
  - Purpose: Validate ONVIF Profile S PTZ presets if supported
  - _Leverage: PTZ service preset patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: PTZ Preset Expert with expertise in PTZ preset validation | Task: Implement validate_ptz_presets method for optional Profile S PTZ presets following requirement 3.1 | Restrictions: Mark as conditional/optional feature, test preset operations if PTZ supported, handle non-PTZ devices gracefully | _Leverage: PTZ service patterns from existing tests_ | _Requirements: 3.1 - Profile S optional PTZ presets_ | Success: PTZ preset validation properly identifies PTZ support and validates presets if available | Instructions: Mark task in-progress [-], implement PTZ preset validation, mark complete [x]_

- [ ] 3.26. Implement Profile S event handling validation (optional)
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_event_handling method for optional event capabilities
  - Purpose: Validate ONVIF Profile S event handling if supported
  - _Leverage: Event service patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Event Handling Expert with expertise in ONVIF event validation | Task: Implement validate_event_handling method for optional Profile S event handling following requirement 3.1 | Restrictions: Mark as conditional/optional feature, test event capabilities if present, validate subscription mechanisms | _Leverage: Event service patterns, subscription mechanisms_ | _Requirements: 3.1 - Profile S optional event handling_ | Success: Event handling validation properly identifies support and validates mechanisms if present | Instructions: Mark task in-progress [-], implement event handling validation, mark complete [x]_

- [ ] 3.27. Implement Profile S recording control validation (optional)
  - Files: e2e/utils/profile_s_validator.py (extend)
  - Description: Add validate_recording_control method for optional recording features
  - Purpose: Validate ONVIF Profile S recording control if supported
  - _Leverage: Recording service patterns_
  - _Requirements: 3.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Recording Control Expert with expertise in ONVIF recording validation | Task: Implement validate_recording_control method for optional Profile S recording control following requirement 3.1 | Restrictions: Mark as conditional/optional feature, test recording capabilities if present, validate control operations | _Leverage: Recording service patterns, media control operations_ | _Requirements: 3.1 - Profile S optional recording control_ | Success: Recording control validation properly identifies support and validates operations if present | Instructions: Mark task in-progress [-], implement recording validation, mark complete [x]_

## Complete Profile T Mandatory Tests

- [ ] 3.28. Implement Profile T absolute PTZ movement validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_absolute_ptz_movement method for absolute PTZ positioning
  - Purpose: Validate ONVIF Profile T mandatory absolute PTZ movement
  - _Leverage: PTZ service absolute movement patterns_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: PTZ Absolute Movement Expert with expertise in precise PTZ positioning | Task: Implement validate_absolute_ptz_movement method for Profile T absolute PTZ following requirement 3.2 | Restrictions: Test absolute positioning operations, validate coordinate systems, ensure precision requirements | _Leverage: PTZ service absolute movement from existing tests_ | _Requirements: 3.2 - Profile T mandatory absolute PTZ movement_ | Success: Absolute PTZ movement validation checks positioning accuracy and coordinate systems | Instructions: Mark task in-progress [-], implement absolute PTZ validation, mark complete [x]_

- [ ] 3.29. Implement Profile T relative PTZ movement validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_relative_ptz_movement method for relative PTZ positioning
  - Purpose: Validate ONVIF Profile T mandatory relative PTZ movement
  - _Leverage: PTZ service relative movement patterns_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: PTZ Relative Movement Expert with expertise in incremental PTZ positioning | Task: Implement validate_relative_ptz_movement method for Profile T relative PTZ following requirement 3.2 | Restrictions: Test relative positioning operations, validate movement increments, ensure proper vector handling | _Leverage: PTZ service relative movement from existing tests_ | _Requirements: 3.2 - Profile T mandatory relative PTZ movement_ | Success: Relative PTZ movement validation checks incremental positioning and vector operations | Instructions: Mark task in-progress [-], implement relative PTZ validation, mark complete [x]_

- [ ] 3.30. Implement Profile T continuous PTZ movement validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_continuous_ptz_movement method for continuous PTZ control
  - Purpose: Validate ONVIF Profile T mandatory continuous PTZ movement
  - _Leverage: PTZ service continuous movement patterns_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: PTZ Continuous Movement Expert with expertise in continuous PTZ control | Task: Implement validate_continuous_ptz_movement method for Profile T continuous PTZ following requirement 3.2 | Restrictions: Test continuous movement operations, validate velocity control, ensure proper stop mechanisms | _Leverage: PTZ service continuous movement from existing tests_ | _Requirements: 3.2 - Profile T mandatory continuous PTZ movement_ | Success: Continuous PTZ movement validation checks velocity control and stop operations | Instructions: Mark task in-progress [-], implement continuous PTZ validation, mark complete [x]_

- [ ] 3.31. Implement Profile T PTZ preset operation validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_ptz_preset_operations method for PTZ preset management
  - Purpose: Validate ONVIF Profile T mandatory PTZ preset operations
  - _Leverage: PTZ service preset operation patterns_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: PTZ Preset Management Expert with expertise in preset operations | Task: Implement validate_ptz_preset_operations method for Profile T PTZ presets following requirement 3.2 | Restrictions: Test create/set/goto/remove preset operations, validate preset storage, ensure proper preset management | _Leverage: PTZ service preset operations from existing tests_ | _Requirements: 3.2 - Profile T mandatory PTZ preset operations_ | Success: PTZ preset operations validation checks complete preset lifecycle management | Instructions: Mark task in-progress [-], implement PTZ preset operations validation, mark complete [x]_

- [ ] 3.32. Implement Profile T PTZ home position validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_ptz_home_position method for PTZ home position support
  - Purpose: Validate ONVIF Profile T mandatory PTZ home position
  - _Leverage: PTZ service home position patterns_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: PTZ Home Position Expert with expertise in PTZ reference positioning | Task: Implement validate_ptz_home_position method for Profile T PTZ home position following requirement 3.2 | Restrictions: Test home position operations, validate reference positioning, ensure proper home position handling | _Leverage: PTZ service home position from existing tests_ | _Requirements: 3.2 - Profile T mandatory PTZ home position_ | Success: PTZ home position validation checks reference positioning and home operations | Instructions: Mark task in-progress [-], implement PTZ home position validation, mark complete [x]_

- [ ] 3.33. Implement Profile T imaging settings validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_imaging_settings method for advanced imaging controls
  - Purpose: Validate ONVIF Profile T mandatory imaging settings
  - _Leverage: Imaging service advanced settings patterns_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Advanced Imaging Expert with expertise in imaging parameter validation | Task: Implement validate_imaging_settings method for Profile T imaging settings following requirement 3.2 | Restrictions: Test advanced imaging controls, validate parameter ranges, ensure proper image quality settings | _Leverage: Imaging service advanced patterns from existing tests_ | _Requirements: 3.2 - Profile T mandatory imaging settings_ | Success: Imaging settings validation checks advanced controls and parameter validation | Instructions: Mark task in-progress [-], implement imaging settings validation, mark complete [x]_

- [ ] 3.34. Implement Profile T video analytics validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_video_analytics method for analytics capabilities
  - Purpose: Validate ONVIF Profile T mandatory video analytics
  - _Leverage: Analytics service patterns_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Video Analytics Expert with expertise in ONVIF analytics validation | Task: Implement validate_video_analytics method for Profile T video analytics following requirement 3.2 | Restrictions: Test analytics capabilities, validate rule configuration, ensure proper analytics output | _Leverage: Analytics service patterns, rule configuration_ | _Requirements: 3.2 - Profile T mandatory video analytics_ | Success: Video analytics validation checks analytics capabilities and rule processing | Instructions: Mark task in-progress [-], implement video analytics validation, mark complete [x]_

- [ ] 3.35. Implement Profile T access control validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_access_control method for access control features
  - Purpose: Validate ONVIF Profile T mandatory access control
  - _Leverage: Access control service patterns_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Access Control Expert with expertise in ONVIF access control validation | Task: Implement validate_access_control method for Profile T access control following requirement 3.2 | Restrictions: Test access control operations, validate credential management, ensure proper access policies | _Leverage: Access control service patterns, credential management_ | _Requirements: 3.2 - Profile T mandatory access control_ | Success: Access control validation checks credential management and access policies | Instructions: Mark task in-progress [-], implement access control validation, mark complete [x]_

- [ ] 3.36. Implement Profile T door control validation
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_door_control method for door control operations
  - Purpose: Validate ONVIF Profile T mandatory door control
  - _Leverage: Door control service patterns_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Door Control Expert with expertise in ONVIF door control validation | Task: Implement validate_door_control method for Profile T door control following requirement 3.2 | Restrictions: Test door control operations, validate door state management, ensure proper access control integration | _Leverage: Door control service patterns, state management_ | _Requirements: 3.2 - Profile T mandatory door control_ | Success: Door control validation checks door operations and state management | Instructions: Mark task in-progress [-], implement door control validation, mark complete [x]_

- [ ] 3.37. Create comprehensive Profile T compliance test
  - Files: e2e/tests/compliance/test_profile_t_complete.py
  - Description: Create complete Profile T compliance test using all validators
  - Purpose: Validate every Profile T requirement from Profiles Conformance spec with consolidated reporting
  - _Leverage: Complete Profile T validator and profile conformance aggregation from tasks 3.18-3.22q_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Profile T Compliance Test Engineer with expertise in comprehensive Profile T validation | Task: Create test_profile_t_complete.py that drives validate_profile_t_conformance(), ensures prerequisite validators execute, and produces spec-referenced compliance reports following requirement 3.2 | Restrictions: Maintain pytest marker structure, surface spec IDs in failure output, support graceful skips for optional capabilities, and preserve detailed reporting | _Leverage: ONVIFProfileTValidator aggregation from tasks 3.18-3.22q, device fixtures_ | _Requirements: 3.2 - Complete Profile T compliance testing_ | Success: Full Profile T run maps every requirement to pass/fail/skipped status with detailed spec references and summary artifacts | Instructions: Mark task in-progress [-], create comprehensive Profile T tests, mark complete [x]_

## Profile T Optional Tests

- [ ] 3.38. Implement Profile T audio streaming validation (optional)
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_audio_streaming method for optional audio streaming
  - Purpose: Validate ONVIF Profile T audio streaming if supported
  - _Leverage: Audio streaming patterns from media service_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Audio Streaming Expert with expertise in Profile T audio validation | Task: Implement validate_audio_streaming method for optional Profile T audio streaming following requirement 3.2 | Restrictions: Mark as conditional/optional feature, test enhanced audio capabilities if present, handle graceful failure if not supported | _Leverage: Audio streaming patterns from media service tests_ | _Requirements: 3.2 - Profile T optional audio streaming_ | Success: Audio streaming validation properly identifies support and validates enhanced features if present | Instructions: Mark task in-progress [-], implement audio streaming validation, mark complete [x]_

- [ ] 3.39. Implement Profile T two-way audio validation (optional)
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_two_way_audio method for optional two-way audio communication
  - Purpose: Validate ONVIF Profile T two-way audio if supported
  - _Leverage: Two-way audio patterns from media service_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Two-Way Audio Expert with expertise in bidirectional audio validation | Task: Implement validate_two_way_audio method for optional Profile T two-way audio following requirement 3.2 | Restrictions: Mark as conditional/optional feature, test bidirectional audio capabilities if present, validate audio input/output | _Leverage: Two-way audio patterns from media service tests_ | _Requirements: 3.2 - Profile T optional two-way audio_ | Success: Two-way audio validation properly identifies support and validates bidirectional audio if present | Instructions: Mark task in-progress [-], implement two-way audio validation, mark complete [x]_

- [ ] 3.40. Implement Profile T edge storage validation (optional)
  - Files: e2e/utils/profile_t_validator.py (extend)
  - Description: Add validate_edge_storage method for optional edge storage capabilities
  - Purpose: Validate ONVIF Profile T edge storage if supported
  - _Leverage: Edge storage patterns from recording service_
  - _Requirements: 3.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Edge Storage Expert with expertise in local storage validation | Task: Implement validate_edge_storage method for optional Profile T edge storage following requirement 3.2 | Restrictions: Mark as conditional/optional feature, test local storage capabilities if present, validate storage management operations | _Leverage: Edge storage patterns from recording service tests_ | _Requirements: 3.2 - Profile T optional edge storage_ | Success: Edge storage validation properly identifies support and validates storage operations if present | Instructions: Mark task in-progress [-], implement edge storage validation, mark complete [x]_

- [ ] 3.41. Create final compliance integration test
  - Files: e2e/tests/compliance/test_onvif_full_compliance.py
  - Description: Create comprehensive test combining Profile S and Profile T validation
  - Purpose: Test complete ONVIF compliance across both profiles
  - _Leverage: Both Profile S and Profile T validators_
  - _Requirements: 3.1, 3.2, 3.4_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Full Compliance Engineer with expertise in comprehensive ONVIF validation | Task: Create test_onvif_full_compliance.py with complete Profile S & T compliance testing following requirements 3.1, 3.2, and 3.4 | Restrictions: Test both profiles comprehensively, generate unified compliance report, provide complete ONVIF certification validation | _Leverage: Complete Profile S and Profile T validators from all previous tasks, compliance reporting system_ | _Requirements: 3.1, 3.2, 3.4 - Complete ONVIF compliance testing and reporting_ | Success: Full ONVIF compliance test validates both profiles with comprehensive certification-ready reporting | Instructions: Mark task in-progress [-], create full compliance test, mark complete [x]_

## Phase 4: Performance Testing Framework

- [ ] 4.1. Create performance metrics data structures
  - Files: e2e/utils/performance_types.py
  - Description: Define PerformanceMetrics dataclass with statistical properties
  - Purpose: Establish foundation for performance data collection and analysis
  - _Leverage: Python dataclasses, statistics module_
  - _Requirements: 4.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Performance Data Engineer with expertise in statistical analysis | Task: Create performance_types.py with PerformanceMetrics dataclass including statistical properties following requirement 4.1 | Restrictions: Only create data structures with properties, do not implement collection logic yet, include mean/median/std dev/p95 calculations | _Leverage: Python dataclasses, statistics module, type hints_ | _Requirements: 4.1 - Statistical analysis with multiple metrics_ | Success: PerformanceMetrics dataclass with statistical properties implemented correctly | Instructions: Mark task in-progress [-], create performance data structures, mark complete [x]_

- [ ] 4.2. Create performance collector class
  - Files: e2e/utils/performance_collector.py
  - Description: Implement PerformanceCollector for measuring response times and resource usage
  - Purpose: Collect performance metrics during test execution
  - _Leverage: psutil library, time module_
  - _Requirements: 4.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Performance Measurement Engineer with expertise in Python profiling | Task: Create performance_collector.py with PerformanceCollector class for measuring response times and resource usage following requirement 4.1 | Restrictions: Only implement measurement collection, do not add statistical analysis yet, ensure accurate timing measurements | _Leverage: psutil library for CPU/memory, time module for response times_ | _Requirements: 4.1 - Performance metrics collection_ | Success: PerformanceCollector accurately measures response times, CPU and memory usage | Instructions: Mark task in-progress [-], create performance collector, mark complete [x]_

- [ ] 4.3. Create baseline management system
  - Files: e2e/utils/performance_baselines.py
  - Description: Implement PerformanceBaseline class for saving/loading baseline metrics
  - Purpose: Manage performance baselines for regression detection
  - _Leverage: JSON serialization, datetime module_
  - _Requirements: 4.3_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Performance Baseline Engineer with expertise in regression detection | Task: Create performance_baselines.py with PerformanceBaseline class for baseline management following requirement 4.3 | Restrictions: Only implement baseline save/load/compare functionality, use JSON for persistence, include configurable tolerance thresholds | _Leverage: JSON module, datetime, pathlib for file operations_ | _Requirements: 4.3 - Baseline comparison and regression detection_ | Success: Baseline system saves/loads metrics and detects regressions with configurable tolerance | Instructions: Mark task in-progress [-], create baseline management system, mark complete [x]_

- [ ] 4.4. Create basic performance test for device operations
  - Files: e2e/tests/performance/test_device_basic_performance.py
  - Description: Implement basic performance test for device operations with statistics
  - Purpose: Test device operation performance with statistical analysis
  - _Leverage: Performance collector and device fixtures_
  - _Requirements: 4.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Performance Test Engineer with expertise in statistical testing | Task: Create test_device_basic_performance.py with basic device operation performance tests following requirement 4.1 | Restrictions: Test only basic device operations (GetDeviceInformation), ensure 20+ iterations for statistical significance, use proper pytest markers | _Leverage: PerformanceCollector from task 4.2, device fixtures from Phase 2_ | _Requirements: 4.1 - Statistical analysis with 20+ iterations_ | Success: Performance test provides statistically significant results with proper metrics | Instructions: Mark task in-progress [-], create basic performance test, mark complete [x]_

- [ ] 4.5. Add concurrent load testing capability
  - Files: e2e/tests/performance/test_concurrent_load.py
  - Description: Implement concurrent load testing using asyncio or threading
  - Purpose: Test system performance under concurrent load
  - _Leverage: concurrent.futures or asyncio_
  - _Requirements: 4.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Concurrent Testing Engineer with expertise in Python concurrency | Task: Create test_concurrent_load.py with concurrent load testing following requirement 4.2 | Restrictions: Test with 50+ concurrent operations, ensure thread safety, measure success rates and response times under load | _Leverage: concurrent.futures or asyncio, performance collector from task 4.2_ | _Requirements: 4.2 - Concurrent load testing with 50+ operations_ | Success: Concurrent tests validate system stability under realistic load conditions | Instructions: Mark task in-progress [-], create concurrent load tests, mark complete [x]_

- [ ] 4.6. Create performance configuration module
  - Files: e2e/config/performance_config.py
  - Description: Define performance thresholds and test configuration
  - Purpose: Centralize performance testing configuration and thresholds
  - _Leverage: dataclasses, environment variables_
  - _Requirements: 4.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Configuration Engineer with expertise in test configuration management | Task: Create performance_config.py with performance thresholds and configuration following requirement 4.1 | Restrictions: Define clear thresholds for response times, success rates, resource usage, allow environment variable overrides | _Leverage: dataclasses, os.environ for environment variables_ | _Requirements: 4.1 - Performance thresholds and configuration_ | Success: Performance configuration provides clear thresholds and environment customization | Instructions: Mark task in-progress [-], create performance configuration, mark complete [x]_

## Phase 5: Device Service Test Enhancement

- [ ] 5.1. Create SOAP helper utilities
  - Files: e2e/utils/soap_helpers.py
  - Description: Implement SOAP request creation and response parsing utilities
  - Purpose: Provide reusable SOAP utilities for device service tests
  - _Leverage: Existing SOAP patterns from tests/fixtures.py_
  - _Requirements: 5.3_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: SOAP Protocol Engineer with expertise in XML and SOAP message handling | Task: Create soap_helpers.py with SOAP request creation and response parsing utilities following requirement 5.3 | Restrictions: Only create utilities, do not implement device-specific logic yet, ensure proper namespace handling | _Leverage: SOAP patterns from e2e/tests/fixtures.py, XML parsing utilities_ | _Requirements: 5.3 - SOAP validation for all operations_ | Success: SOAP helpers provide reusable utilities for request/response handling | Instructions: Mark task in-progress [-], create SOAP helper utilities, mark complete [x]_

- [ ] 5.2. Create test data manager for device tests
  - Files: e2e/utils/test_data_manager.py
  - Description: Implement test data setup and cleanup management
  - Purpose: Manage test data lifecycle for device service tests
  - _Leverage: Python context managers, tempfile module_
  - _Requirements: 5.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Test Data Engineer with expertise in test data management and cleanup | Task: Create test_data_manager.py with test data lifecycle management following requirement 5.1 | Restrictions: Use context managers for cleanup, ensure no test data leakage, provide clean setup/teardown | _Leverage: Python contextlib, tempfile, pathlib modules_ | _Requirements: 5.1 - Test data setup and teardown_ | Success: Test data manager provides clean data lifecycle with proper cleanup | Instructions: Mark task in-progress [-], create test data manager, mark complete [x]_

- [ ] 5.3. Refactor existing device service tests
  - Files: e2e/tests/integration/test_device_service_basic.py
  - Description: Refactor existing GetDeviceInformation, GetCapabilities tests
  - Purpose: Improve existing device tests with enhanced validation
  - _Leverage: SOAP helpers from task 5.1, existing device tests_
  - _Requirements: 5.3_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Test Refactoring Engineer with expertise in improving existing test quality | Task: Refactor existing device service tests using new SOAP helpers following requirement 5.3 | Restrictions: Only refactor existing tests (GetDeviceInformation, GetCapabilities), maintain same test coverage, enhance validation | _Leverage: SOAP helpers from task 5.1, existing device service tests_ | _Requirements: 5.3 - Enhanced SOAP validation_ | Success: Existing device tests use new helpers and provide better validation | Instructions: Mark task in-progress [-], refactor existing tests, mark complete [x]_

- [ ] 5.4. Add missing SetSystemDateAndTime operation test
  - Files: e2e/tests/integration/test_device_service_datetime.py
  - Description: Implement test for SetSystemDateAndTime operation
  - Purpose: Add coverage for missing datetime operation
  - _Leverage: SOAP helpers and device fixtures_
  - _Requirements: 5.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Device Operation Engineer with expertise in datetime operations | Task: Create test for SetSystemDateAndTime operation following requirement 5.2 | Restrictions: Only implement this one operation test, test both valid and invalid datetime formats, include error scenarios | _Leverage: SOAP helpers from task 5.1, device fixtures from Phase 2_ | _Requirements: 5.2 - Missing operation coverage_ | Success: SetSystemDateAndTime operation fully tested with positive and negative scenarios | Instructions: Mark task in-progress [-], implement datetime operation test, mark complete [x]_

- [ ] 5.5. Add missing user management operations tests
  - Files: e2e/tests/integration/test_device_service_users.py
  - Description: Implement tests for CreateUsers and DeleteUsers operations
  - Purpose: Add coverage for missing user management operations
  - _Leverage: SOAP helpers and device fixtures_
  - _Requirements: 5.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF User Management Engineer with expertise in user operation testing | Task: Create tests for CreateUsers and DeleteUsers operations following requirement 5.2 | Restrictions: Only implement user management tests, ensure proper user creation/deletion flow, test error scenarios | _Leverage: SOAP helpers from task 5.1, device fixtures from Phase 2_ | _Requirements: 5.2 - Missing operation coverage_ | Success: User management operations fully tested with complete lifecycle validation | Instructions: Mark task in-progress [-], implement user management tests, mark complete [x]_

- [ ] 5.6. Add comprehensive error handling tests
  - Files: e2e/tests/integration/test_device_service_errors.py
  - Description: Implement comprehensive error handling and SOAP fault tests
  - Purpose: Validate proper error responses and SOAP fault handling
  - _Leverage: SOAP helpers and error scenarios_
  - _Requirements: 5.4_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Error Handling Test Engineer with expertise in SOAP fault validation | Task: Create comprehensive error handling tests for device service following requirement 5.4 | Restrictions: Test invalid operations, malformed requests, authentication failures, ensure proper SOAP fault responses | _Leverage: SOAP helpers from task 5.1, device fixtures from Phase 2_ | _Requirements: 5.4 - Error handling and fault response testing_ | Success: All error scenarios tested with proper SOAP fault validation | Instructions: Mark task in-progress [-], create error handling tests, mark complete [x]_

## Phase 6: Code Quality Enforcement

- [ ] 6.1. Create development requirements file
  - Files: e2e/requirements-dev.txt
  - Description: Define development dependencies for code quality tools
  - Purpose: Specify all development dependencies with pinned versions
  - _Leverage: Python package management best practices_
  - _Requirements: 2.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Package Manager with expertise in dependency management | Task: Create requirements-dev.txt with development dependencies following requirement 2.1 | Restrictions: Pin exact versions, organize by purpose (formatting, linting, testing), include all necessary quality tools | _Leverage: Python packaging best practices, quality tool ecosystem_ | _Requirements: 2.1 - Development dependencies specification_ | Success: requirements-dev.txt contains all quality tools with pinned versions | Instructions: Mark task in-progress [-], create development requirements, mark complete [x]_

- [ ] 6.2. Create project configuration file
  - Files: e2e/pyproject.toml
  - Description: Configure Black, isort, Pylint, mypy, coverage settings
  - Purpose: Centralize all tool configuration in one file
  - _Leverage: Python project configuration standards_
  - _Requirements: 2.1_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Python Configuration Engineer with expertise in tool configuration | Task: Create pyproject.toml with tool configurations following requirement 2.1 | Restrictions: Configure for 88-char line length, Pylint ≥8.0, 90% coverage, strict type checking, maintain consistency | _Leverage: Tool-specific configuration best practices_ | _Requirements: 2.1 - Tool configuration and standards_ | Success: pyproject.toml provides consistent configuration for all quality tools | Instructions: Mark task in-progress [-], create project configuration, mark complete [x]_

- [ ] 6.3. Create pre-commit hooks configuration
  - Files: e2e/.pre-commit-config.yaml
  - Description: Set up pre-commit hooks for automated quality enforcement
  - Purpose: Automatically enforce code quality before commits
  - _Leverage: pre-commit framework and hooks ecosystem_
  - _Requirements: 2.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Git Hooks Engineer with expertise in pre-commit automation | Task: Create .pre-commit-config.yaml with automated quality enforcement following requirement 2.2 | Restrictions: Include Black, isort, Pylint, mypy, bandit hooks, ensure hooks work together, set appropriate failure thresholds | _Leverage: pre-commit framework, quality tool integrations_ | _Requirements: 2.2 - Pre-commit hook automation_ | Success: Pre-commit hooks automatically enforce all quality standards before commits | Instructions: Mark task in-progress [-], create pre-commit configuration, mark complete [x]_

- [ ] 6.4. Create quality validation script
  - Files: e2e/scripts/quality_check.py
  - Description: Implement comprehensive quality validation script
  - Purpose: Run all quality checks with detailed reporting
  - _Leverage: subprocess module, quality tools_
  - _Requirements: 2.5_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Quality Validation Engineer with expertise in automated quality checking | Task: Create quality_check.py with comprehensive quality validation following requirement 2.5 | Restrictions: Run all quality tools, provide clear pass/fail reporting, return proper exit codes, include helpful error messages | _Leverage: subprocess module, pathlib, existing quality tool configuration_ | _Requirements: 2.5 - Quality validation scripts_ | Success: Quality check script runs all tools and provides clear validation results | Instructions: Mark task in-progress [-], create quality validation script, mark complete [x]_

## Phase 7: Final Integration and Documentation

- [ ] 7.1. Create test execution configuration
  - Files: e2e/config/test_execution.py
  - Description: Define test execution settings and category configurations
  - Purpose: Centralize test execution configuration
  - _Leverage: dataclasses, pytest configuration_
  - _Requirements: 1.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Test Configuration Engineer with expertise in pytest configuration | Task: Create test_execution.py with execution settings following requirement 1.2 | Restrictions: Define clear category markers, execution timeouts, parallel execution settings | _Leverage: dataclasses, pytest marker system_ | _Requirements: 1.2 - Test categorization and selective execution_ | Success: Test execution configuration supports all categories and selective execution | Instructions: Mark task in-progress [-], create execution configuration, mark complete [x]_

- [ ] 7.2. Create comprehensive test runner script
  - Files: e2e/scripts/run_comprehensive_tests.py
  - Description: Implement enhanced test runner with category selection
  - Purpose: Provide unified test execution with category filtering
  - _Leverage: Existing run_tests.py patterns, pytest execution_
  - _Requirements: 1.2_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Test Execution Engineer with expertise in pytest automation | Task: Create run_comprehensive_tests.py with category-based execution following requirement 1.2 | Restrictions: Maintain compatibility with existing run_tests.py, add category selection, provide clear execution reporting | _Leverage: Existing e2e/run_tests.py patterns, pytest marker system_ | _Requirements: 1.2 - Selective test execution by category_ | Success: Test runner supports selective execution by category with clear reporting | Instructions: Mark task in-progress [-], create comprehensive test runner, mark complete [x]_

- [ ] 7.3. Create validation script for framework completeness
  - Files: e2e/scripts/validate_framework.py
  - Description: Implement framework validation that checks all components work together
  - Purpose: Validate complete framework functionality
  - _Leverage: All components from previous phases_
  - _Requirements: All requirements validation_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: System Validation Engineer with expertise in integration testing | Task: Create validate_framework.py that validates all framework components work together covering all requirements | Restrictions: Check directory structure, fixtures, validators, performance tools, quality enforcement - ensure everything functions correctly | _Leverage: All implemented components from Phases 1-6_ | _Requirements: All requirements - comprehensive framework validation_ | Success: Validation script confirms all components work together and requirements are met | Instructions: Mark task in-progress [-], create framework validation, mark complete [x]_

- [ ] 7.4. Create development environment setup script
  - Files: e2e/scripts/setup_dev_environment.sh
  - Description: Create automated development environment setup
  - Purpose: Automate developer onboarding and environment setup
  - _Leverage: Virtual environment, package installation_
  - _Requirements: Developer productivity_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: DevOps Engineer with expertise in development environment automation | Task: Create setup_dev_environment.sh for automated development setup | Restrictions: Create virtual environment, install dependencies, configure pre-commit hooks, provide clear success confirmation | _Leverage: Python venv, pip, pre-commit installation_ | _Requirements: Development environment automation_ | Success: Setup script creates complete development environment from scratch | Instructions: Mark task in-progress [-], create environment setup script, mark complete [x]_

- [ ] 7.5. Create framework documentation
  - Files: e2e/docs/framework_guide.md
  - Description: Document the complete testing framework with usage examples
  - Purpose: Provide comprehensive developer documentation
  - _Leverage: All framework components and best practices_
  - _Requirements: Documentation for developer adoption_
  - _Prompt: Implement the task for spec e2e-tests, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Technical Writer with expertise in testing framework documentation | Task: Create framework_guide.md with comprehensive testing framework documentation | Restrictions: Include setup instructions, usage examples, category explanations, troubleshooting guide, maintain clear structure for developers | _Leverage: All implemented framework components, Python documentation standards_ | _Requirements: Complete framework documentation_ | Success: Documentation provides clear guidance for framework usage and development | Instructions: Mark task in-progress [-], create comprehensive documentation, mark complete [x]_

# Tasks Document

## Task Breakdown

- [ ] 1. Create HTTP test helper library
  - Files: `cross-compile/onvif/tests/src/common/http_test_helpers.c`, `cross-compile/onvif/tests/src/common/http_test_helpers.h`
  - Create utilities for sending raw HTTP requests via TCP sockets
  - Implement HTTP response parsing and validation functions
  - Add assertion helpers for HTTP status codes and headers
  - Purpose: Provide reusable infrastructure for HTTP compliance testing
  - _Leverage: `src/common/test_helpers.c`, `networking/http/http_parser.h`_
  - _Requirements: All (foundation for all tests)_
  - _Prompt: Implement the task for spec http-compliance, first run spec-workflow-guide to get the workflow guide then implement the task: Role: C Developer specializing in network programming and HTTP protocol | Task: Create comprehensive HTTP test helper library in cross-compile/onvif/tests/src/common/http_test_helpers.c/h with raw TCP socket communication, HTTP request generation, response parsing, and validation functions. Use existing patterns from src/common/test_helpers.c and types from networking/http/http_parser.h. Set this task to in_progress in .spec-workflow/specs/http-compliance/tasks.md, then implement. | Restrictions: Must use POSIX sockets only, follow test helper patterns from test_helpers.c, use existing http_request_t/http_response_t types, NO magic numbers for return codes (use constants), mandatory Doxygen documentation for all functions, follow global variable naming g_http_test_<name> if needed | _Leverage: src/common/test_helpers.c for patterns, networking/http/http_parser.h for types, existing CMocka assertion patterns | Success: Library compiles without warnings, provides raw HTTP request sending via TCP, parses HTTP responses correctly, includes status code and header assertion helpers, full Doxygen documentation, follows project coding standards. Mark task as completed [x] in tasks.md when done._

- [ ] 2. Implement HTTP message format tests
  - File: `cross-compile/onvif/tests/src/integration/http_compliance/test_http_message_format.c`
  - Test RFC 9112 message format: "start-line CRLF *( field-line CRLF ) CRLF [ message-body ]"
  - Validate CRLF and LF-only line ending handling
  - Test rejection of invalid header syntax (whitespace between name and colon)
  - Test rejection of malformed requests
  - Purpose: Ensure HTTP server correctly parses RFC 9112 message format
  - _Leverage: `http_test_helpers.h`, `networking/http/http_parser.c`_
  - _Requirements: 1 (HTTP Message Format Validation)_
  - _Prompt: Implement the task for spec http-compliance, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Integration Test Engineer with expertise in HTTP protocol testing | Task: Implement integration tests for HTTP message format validation in cross-compile/onvif/tests/src/integration/http_compliance/test_http_message_format.c covering requirement 1. Use http_test_helpers for sending requests and validating responses. Test valid formats, CRLF/LF handling, invalid header whitespace, and malformed requests. Set this task to in_progress in .spec-workflow/specs/http-compliance/tasks.md, then implement. | Restrictions: Test naming MUST follow test_integration_http_message_format_<scenario> pattern, use http_test_helpers functions only, NO direct socket code in tests, test server in isolated environment, all assertions use CMocka macros, full Doxygen comments | _Leverage: http_test_helpers.h for TCP communication, src/common/test_helpers.c for patterns, networking/http/http_parser.c as test target | Success: All acceptance criteria for requirement 1 tested, tests validate RFC 9112 message format compliance, proper CRLF/LF handling verified, invalid syntax correctly rejected with 400, tests pass consistently. Run: make test-http-compliance-message-format. Mark task as completed [x] in tasks.md when done._

- [ ] 3. Implement HTTP method support tests
  - File: `cross-compile/onvif/tests/src/integration/http_compliance/test_http_methods.c`
  - Test POST and GET methods are accepted
  - Test PUT, DELETE, PATCH, HEAD, OPTIONS, CONNECT, TRACE are rejected with 405
  - Validate 405 response includes "Allow: GET, POST" header
  - Purpose: Ensure ONVIF method requirements (POST/GET only)
  - _Leverage: `http_test_helpers.h`, `networking/http/http_server.c`_
  - _Requirements: 2 (HTTP Method Support)_
  - _Prompt: Implement the task for spec http-compliance, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Compliance Test Engineer with HTTP protocol expertise | Task: Implement integration tests for HTTP method support in cross-compile/onvif/tests/src/integration/http_compliance/test_http_methods.c covering requirement 2. Test POST/GET acceptance, reject unsupported methods with 405, verify Allow header. Set this task to in_progress in .spec-workflow/specs/http-compliance/tasks.md, then implement. | Restrictions: Test naming MUST follow test_integration_http_methods_<scenario> pattern, verify exact status codes (200 for POST/GET, 405 for others), validate Allow header format exactly, use http_test_helpers only, all assertions use CMocka, mandatory Doxygen documentation | _Leverage: http_test_helpers.h for request sending, networking/http/http_server.c as test target, ONVIF spec requirements | Success: POST and GET methods work correctly, all unsupported methods return 405, Allow header present and correct in 405 responses, all requirement 2 acceptance criteria met, tests pass reliably. Run: make test-http-compliance-methods. Mark task as completed [x] in tasks.md when done._

- [ ] 4. Implement Host header validation tests
  - File: `cross-compile/onvif/tests/src/integration/http_compliance/test_http_headers.c`
  - Test missing Host header returns 400
  - Test multiple Host headers returns 400
  - Test Host header validation (valid hostname/port)
  - Test invalid Host header returns 400
  - Purpose: Ensure RFC 9112 Host header requirements
  - _Leverage: `http_test_helpers.h`, `networking/http/http_parser.c`_
  - _Requirements: 3 (Host Header Validation)_
  - _Prompt: Implement the task for spec http-compliance, first run spec-workflow-guide to get the workflow guide then implement the task: Role: HTTP Security Test Engineer specializing in header validation | Task: Implement integration tests for Host header validation in cross-compile/onvif/tests/src/integration/http_compliance/test_http_headers.c covering requirement 3. Test missing Host, multiple Host headers, validation, invalid formats - all return 400. Set this task to in_progress in .spec-workflow/specs/http-compliance/tasks.md, then implement. | Restrictions: Test naming MUST follow test_integration_http_headers_<scenario> pattern, validate exact 400 status codes, test edge cases (empty host, invalid port, malformed), use http_test_helpers for all requests, CMocka assertions only, full Doxygen docs | _Leverage: http_test_helpers.h for TCP requests, networking/http/http_parser.c header parsing as target, RFC 9112 Host requirements | Success: Missing Host header rejected with 400, multiple Host headers rejected with 400, Host validation works correctly, invalid formats rejected, all requirement 3 acceptance criteria met, tests pass consistently. Run: make test-http-compliance-headers. Mark task as completed [x] in tasks.md when done._

- [ ] 5. Implement transfer encoding tests
  - File: `cross-compile/onvif/tests/src/integration/http_compliance/test_http_transfer_encoding.c`
  - Test basic chunked transfer encoding parsing
  - Test multiple chunks with correct size parsing
  - Test chunked encoding with trailing headers
  - Test rejection of multiple transfer codings (400)
  - Test unknown transfer coding returns 501
  - Test chunked must be final encoding
  - Purpose: Ensure RFC 9112 transfer encoding compliance
  - _Leverage: `http_test_helpers.h`, `networking/http/http_parser.c`_
  - _Requirements: 4 (Transfer Encoding Support)_
  - _Prompt: Implement the task for spec http-compliance, first run spec-workflow-guide to get the workflow guide then implement the task: Role: HTTP Protocol Expert specializing in transfer encoding | Task: Implement integration tests for chunked transfer encoding in cross-compile/onvif/tests/src/integration/http_compliance/test_http_transfer_encoding.c covering requirement 4. Test chunked encoding, multiple chunks, trailers, error cases (multiple encodings, unknown coding, chunked not final). Set this task to in_progress in .spec-workflow/specs/http-compliance/tasks.md, then implement. | Restrictions: Test naming MUST follow test_integration_http_transfer_<scenario> pattern, create valid chunked requests (size CRLF data CRLF), test trailing headers support, validate 400 for multiple encodings, 501 for unknown, use http_test_helpers, CMocka assertions, Doxygen docs | _Leverage: http_test_helpers.h for chunked request generation, networking/http/http_parser.c transfer encoding parsing, RFC 9112 chunked format specification | Success: Basic chunked encoding works, multiple chunks parsed correctly, trailers supported, multiple encodings rejected with 400, unknown encoding returns 501, chunked-not-final rejected, all requirement 4 acceptance criteria met. Run: make test-http-compliance-chunked. Mark task as completed [x] in tasks.md when done._

- [ ] 6. Implement HTTP authentication tests
  - File: `cross-compile/onvif/tests/src/integration/http_compliance/test_http_auth_integration.c`
  - Test 401 response when accessing protected resource without credentials
  - Test WWW-Authenticate header format in 401 response
  - Test valid digest credentials are accepted
  - Test invalid digest credentials are rejected with 401
  - Test digest authentication parameters (nonce, realm, qop, algorithm)
  - Purpose: Ensure ONVIF digest authentication compliance
  - _Leverage: `http_test_helpers.h`, `networking/http/http_auth.c`, `tests/src/unit/networking/test_http_auth.c` patterns_
  - _Requirements: 5 (HTTP Authentication)_
  - _Prompt: Implement the task for spec http-compliance, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Security Test Engineer with expertise in HTTP digest authentication | Task: Implement integration tests for HTTP digest authentication in cross-compile/onvif/tests/src/integration/http_compliance/test_http_auth_integration.c covering requirement 5. Test 401 without credentials, WWW-Authenticate header, valid/invalid credentials, digest parameters. Extend patterns from tests/src/unit/networking/test_http_auth.c for integration level. Set this task to in_progress in .spec-workflow/specs/http-compliance/tasks.md, then implement. | Restrictions: Test naming MUST follow test_integration_http_auth_<scenario> pattern, validate WWW-Authenticate header format per RFC 2617, test digest parameter correctness, use http_test_helpers for auth header building, NO hardcoded credentials in tests, CMocka assertions, Doxygen docs | _Leverage: http_test_helpers.h for auth requests, networking/http/http_auth.c as target, tests/src/unit/networking/test_http_auth.c for patterns, RFC 2617 digest spec | Success: 401 returned without credentials, WWW-Authenticate header correct format, valid credentials accepted, invalid rejected with 401, digest parameters validated, all requirement 5 acceptance criteria met. Run: make test-http-compliance-auth. Mark task as completed [x] in tasks.md when done._

- [ ] 7. Implement HTTP status code tests
  - File: `cross-compile/onvif/tests/src/integration/http_compliance/test_http_status_codes.c`
  - Test 200 OK for successful requests
  - Test 400 Bad Request for malformed requests
  - Test 401 Unauthorized for missing authentication
  - Test 405 Method Not Allowed for unsupported methods
  - Test 415 Unsupported Media Type for wrong Content-Type
  - Test 500 Internal Server Error for server errors
  - Purpose: Ensure ONVIF status code requirements
  - _Leverage: `http_test_helpers.h`, `networking/http/http_server.c`_
  - _Requirements: 6 (HTTP Status Code Compliance)_
  - _Prompt: Implement the task for spec http-compliance, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Compliance Engineer with HTTP status code expertise | Task: Implement integration tests for HTTP status codes in cross-compile/onvif/tests/src/integration/http_compliance/test_http_status_codes.c covering requirement 6. Test all ONVIF-required status codes: 200, 400, 401, 405, 415, 500 with appropriate triggering scenarios. Set this task to in_progress in .spec-workflow/specs/http-compliance/tasks.md, then implement. | Restrictions: Test naming MUST follow test_integration_http_status_<code>_<scenario> pattern, validate exact status codes, test realistic triggering conditions, verify status text matches code, use http_test_helpers, CMocka assertions, Doxygen docs | _Leverage: http_test_helpers.h for requests, networking/http/http_server.c response generation, ONVIF Core Spec status code requirements | Success: 200 for valid requests, 400 for malformed, 401 for no auth, 405 for bad method, 415 for bad Content-Type, 500 for server errors, all requirement 6 acceptance criteria met. Run: make test-http-compliance-status. Mark task as completed [x] in tasks.md when done._

- [ ] 8. Implement request-target format tests
  - File: `cross-compile/onvif/tests/src/integration/http_compliance/test_http_request_target.c`
  - Test origin-form request-target (e.g., /onvif/device_service)
  - Test absolute-form request-target (e.g., http://camera.local/onvif/device_service)
  - Test asterisk-form for OPTIONS * requests
  - Test invalid request-target formats return 400
  - Purpose: Ensure RFC 9112 request-target compliance
  - _Leverage: `http_test_helpers.h`, `networking/http/http_parser.c`_
  - _Requirements: 7 (Request-Target Format Support)_
  - _Prompt: Implement the task for spec http-compliance, first run spec-workflow-guide to get the workflow guide then implement the task: Role: HTTP Protocol Specialist with RFC 9112 expertise | Task: Implement integration tests for request-target formats in cross-compile/onvif/tests/src/integration/http_compliance/test_http_request_target.c covering requirement 7. Test origin-form, absolute-form, asterisk-form, and invalid formats. Set this task to in_progress in .spec-workflow/specs/http-compliance/tasks.md, then implement. | Restrictions: Test naming MUST follow test_integration_http_request_target_<form> pattern, create valid examples per RFC 9112, test OPTIONS * specifically, validate 400 for invalid formats, use http_test_helpers, CMocka assertions, Doxygen docs | _Leverage: http_test_helpers.h for request generation, networking/http/http_parser.c request-target parsing, RFC 9112 request-target specification | Success: Origin-form works, absolute-form works, asterisk-form for OPTIONS works, invalid formats rejected with 400, all requirement 7 acceptance criteria met. Run: make test-http-compliance-request-target. Mark task as completed [x] in tasks.md when done._

- [ ] 9. Implement connection management tests
  - File: `cross-compile/onvif/tests/src/integration/http_compliance/test_http_connection.c`
  - Test Connection: keep-alive keeps connection open
  - Test Connection: close closes connection after response
  - Test persistent connection is default for HTTP/1.1
  - Test idle connection timeout and closure
  - Test graceful connection closure
  - Purpose: Ensure RFC 9112 connection management compliance
  - _Leverage: `http_test_helpers.h`, `networking/http/http_server.c`_
  - _Requirements: 8 (Connection Management)_
  - _Prompt: Implement the task for spec http-compliance, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Network Protocol Engineer with TCP/HTTP connection expertise | Task: Implement integration tests for HTTP connection management in cross-compile/onvif/tests/src/integration/http_compliance/test_http_connection.c covering requirement 8. Test keep-alive, close, persistent default, timeout, graceful closure. Need socket state tracking in test helpers. Set this task to in_progress in .spec-workflow/specs/http-compliance/tasks.md, then implement. | Restrictions: Test naming MUST follow test_integration_http_connection_<scenario> pattern, track socket state across requests, validate connection reuse for keep-alive, verify closure for Connection: close, test timeout behavior (may need mock time), use http_test_helpers, CMocka assertions, Doxygen docs | _Leverage: http_test_helpers.h with connection state tracking, networking/http/http_server.c connection handling, RFC 9112 connection management | Success: Keep-alive maintains connection, Connection: close closes properly, persistent is default, timeout works, graceful closure verified, all requirement 8 acceptance criteria met. Run: make test-http-compliance-connection. Mark task as completed [x] in tasks.md when done._

- [ ] 10. Implement Content-Type validation tests
  - File: `cross-compile/onvif/tests/src/integration/http_compliance/test_http_content_type.c`
  - Test Content-Type: application/soap+xml is accepted
  - Test Content-Type: text/xml is accepted
  - Test unsupported Content-Type returns 415
  - Test missing Content-Type in POST returns 400
  - Purpose: Ensure ONVIF Content-Type requirements
  - _Leverage: `http_test_helpers.h`, `networking/http/http_server.c`_
  - _Requirements: 9 (Content-Type Validation)_
  - _Prompt: Implement the task for spec http-compliance, first run spec-workflow-guide to get the workflow guide then implement the task: Role: ONVIF Web Services Test Engineer with SOAP expertise | Task: Implement integration tests for Content-Type validation in cross-compile/onvif/tests/src/integration/http_compliance/test_http_content_type.c covering requirement 9. Test SOAP content types (application/soap+xml, text/xml) acceptance, reject unsupported types with 415, missing Content-Type in POST returns 400. Set this task to in_progress in .spec-workflow/specs/http-compliance/tasks.md, then implement. | Restrictions: Test naming MUST follow test_integration_http_content_type_<scenario> pattern, test with actual SOAP-like body content, validate exact 415/400 status codes, verify Content-Type header parsing, use http_test_helpers, CMocka assertions, Doxygen docs | _Leverage: http_test_helpers.h for Content-Type headers, networking/http/http_server.c validation logic, ONVIF Core Spec Content-Type requirements | Success: application/soap+xml accepted, text/xml accepted, unsupported types return 415, missing Content-Type in POST returns 400, all requirement 9 acceptance criteria met. Run: make test-http-compliance-content-type. Mark task as completed [x] in tasks.md when done._

- [ ] 11. Implement error response security tests
  - File: `cross-compile/onvif/tests/src/integration/http_compliance/test_http_error_security.c`
  - Test error responses do not include internal implementation details
  - Test error responses do not include stack traces
  - Test error responses do not expose file paths
  - Test error responses provide minimal information
  - Purpose: Ensure secure error handling per ONVIF security requirements
  - _Leverage: `http_test_helpers.h`, `networking/http/http_server.c`_
  - _Requirements: 10 (Error Response Security)_
  - _Prompt: Implement the task for spec http-compliance, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Security Test Engineer with expertise in information disclosure prevention | Task: Implement integration tests for error response security in cross-compile/onvif/tests/src/integration/http_compliance/test_http_error_security.c covering requirement 10. Trigger various errors, validate responses do NOT contain internal details, stack traces, file paths, or excessive information. Set this task to in_progress in .spec-workflow/specs/http-compliance/tasks.md, then implement. | Restrictions: Test naming MUST follow test_integration_http_error_security_<scenario> pattern, scan error response bodies for sensitive patterns (regex for paths, stack traces), trigger diverse error conditions, verify minimal info only, use http_test_helpers, CMocka assertions, Doxygen docs | _Leverage: http_test_helpers.h for error triggering, networking/http/http_server.c error response generation, ONVIF security best practices | Success: No internal details in errors, no stack traces, no file paths, minimal information provided, all requirement 10 acceptance criteria met. Run: make test-http-compliance-error-security. Mark task as completed [x] in tasks.md when done._

- [ ] 12. Create HTTP compliance test runner
  - File: `cross-compile/onvif/tests/src/runner/test_http_compliance_runner.c`
  - Create modular test runner for HTTP compliance test suite
  - Register all HTTP compliance test functions
  - Set up HTTP server before tests, tear down after
  - Add test result reporting
  - Purpose: Execute complete HTTP compliance test suite
  - _Leverage: `src/runner/test_integration_runner.c` patterns_
  - _Requirements: All_
  - _Prompt: Implement the task for spec http-compliance, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Test Infrastructure Engineer with CMocka framework expertise | Task: Create HTTP compliance test runner in cross-compile/onvif/tests/src/runner/test_http_compliance_runner.c following patterns from src/runner/test_integration_runner.c. Register all test functions from tasks 2-11, set up/tear down HTTP server, provide result reporting. Set this task to in_progress in .spec-workflow/specs/http-compliance/tasks.md, then implement. | Restrictions: Follow existing runner patterns exactly, use CMocka group/setup/teardown functions, start HTTP server in test setup (isolated config), stop in teardown, clean resource cleanup, register tests by category, Doxygen docs for runner | _Leverage: src/runner/test_integration_runner.c for patterns, CMocka test runner API, networking/http/http_server.h for start/stop | Success: Runner compiles and links all tests, HTTP server starts/stops cleanly, all tests execute in isolated environment, proper result reporting, clean resource cleanup. Run: make test-http-compliance runs all tests. Mark task as completed [x] in tasks.md when done._

- [ ] 13. Update Makefile with HTTP compliance targets
  - File: `cross-compile/onvif/tests/Makefile`
  - Add HTTP_COMPLIANCE_TEST_SRCS variable with all test source files
  - Add test-http-compliance target to build and run all HTTP compliance tests
  - Add individual category targets (test-http-compliance-methods, test-http-compliance-auth, etc.)
  - Add test-http-compliance-coverage target for coverage analysis
  - Integrate with existing test infrastructure
  - Purpose: Enable building and running HTTP compliance tests
  - _Leverage: Existing Makefile test targets and patterns_
  - _Requirements: All (build system integration)_
  - _Prompt: Implement the task for spec http-compliance, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Build System Engineer with GNU Make expertise | Task: Update cross-compile/onvif/tests/Makefile to add HTTP compliance test targets. Add source variables, compile targets, test execution targets (test-http-compliance, test-http-compliance-<category>), coverage target. Follow existing Makefile patterns for integration tests. Set this task to in_progress in .spec-workflow/specs/http-compliance/tasks.md, then implement. | Restrictions: Follow existing Makefile structure and naming, use modular runner approach like existing tests, add to appropriate sections (test sources, runners, targets), maintain existing build flags, support coverage builds, ensure compatibility with native compilation for tests | _Leverage: Existing Makefile patterns for INTEGRATION_TEST_SRCS and runners, CMocka library setup, coverage target patterns | Success: make test-http-compliance builds and runs all tests, individual category targets work, coverage target generates reports, integrates with existing test infrastructure, no regression in existing test targets. Verify with: make test-http-compliance && make test-http-compliance-coverage. Mark task as completed [x] in tasks.md when done._

- [ ] 14. Update test documentation
  - Files: `cross-compile/onvif/tests/README.md`, `.spec-workflow/specs/http-compliance/README.md`
  - Document HTTP compliance test suite purpose and structure
  - Add instructions for running HTTP compliance tests
  - Document test coverage and what is validated
  - Add troubleshooting section
  - Purpose: Provide clear documentation for HTTP compliance testing
  - _Leverage: Existing test README structure_
  - _Requirements: All (documentation)_
  - _Prompt: Implement the task for spec http-compliance, first run spec-workflow-guide to get the workflow guide then implement the task: Role: Technical Documentation Specialist with testing expertise | Task: Update cross-compile/onvif/tests/README.md to document HTTP compliance test suite and create .spec-workflow/specs/http-compliance/README.md with comprehensive testing documentation. Include purpose, structure, running instructions, coverage details, troubleshooting. Set this task to in_progress in .spec-workflow/specs/http-compliance/tasks.md, then implement. | Restrictions: Follow existing README format and structure, use clear markdown formatting, include command examples that work, document all test categories, explain what compliance is validated, provide troubleshooting for common issues, keep concise and scannable | _Leverage: Existing tests/README.md structure, spec requirements/design docs for content | Success: README clearly documents test suite, instructions are accurate and complete, coverage is explained, troubleshooting helps debug issues, documentation is professional and useful. Validate: follow documented commands successfully. Mark task as completed [x] in tasks.md when done._

- [ ] 15. Run complete test suite and validate coverage
  - Verify all HTTP compliance tests pass
  - Generate coverage report (target >90% of HTTP server/parser code)
  - Run code quality validation (linting and formatting)
  - Fix any issues found
  - Validate against requirements document
  - Purpose: Ensure complete and correct HTTP compliance test implementation
  - _Leverage: Makefile test and coverage targets, linting/formatting scripts_
  - _Requirements: All_
  - _Prompt: Implement the task for spec http-compliance, first run spec-workflow-guide to get the workflow guide then implement the task: Role: QA Engineer with test validation and quality assurance expertise | Task: Execute complete HTTP compliance test suite validation. Run all tests, generate coverage report, run linting/formatting, fix issues, verify all requirements met. Check coverage >90% of HTTP code. Set this task to in_progress in .spec-workflow/specs/http-compliance/tasks.md, then implement. | Restrictions: ALL tests MUST pass, coverage MUST be >90% of HTTP server/parser, NO linting errors, NO formatting issues, verify each requirement has passing tests, document any gaps or issues, fix all identified problems | _Leverage: make test-http-compliance, make test-http-compliance-coverage-html, ./cross-compile/onvif/scripts/lint_code.sh --check, ./cross-compile/onvif/scripts/format_code.sh --check | Success: All tests pass consistently, coverage >90%, zero linting errors, zero formatting issues, all requirements validated with tests, test suite is complete and correct. Generate report: make test-http-compliance-coverage-html. Mark task as completed [x] in tasks.md when done._

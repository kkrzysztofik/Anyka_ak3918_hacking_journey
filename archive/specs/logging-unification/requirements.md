# Requirements Document

## Introduction

The logging unification feature addresses the critical technical debt in the ONVIF project caused by multiple fragmented logging implementations that violate the DRY (Don't Repeat Yourself) principle. Currently, the project has four distinct logging systems: platform logging, service logging, generic logging utilities, and direct stdio usage. In parallel, error handling logic is scattered across services, ad-hoc helper files, and inline HTTP/SOAP response builders. This fragmentation creates maintenance overhead, inconsistent behavior, security blind spots, and poor developer experience.

The unified logging and error handling system will provide a single, function-based API surface that consolidates diagnostic output and error propagation into a maintainable, debuggable, and performance-optimized solution. A cohesive design ensures that every failure path produces structured logs, machine-readable error objects, and standards-compliant protocol responses. This aligns with the project's principles of code quality, developer experience, security, and maintainability while supporting the reliability requirements of embedded systems.

## Alignment with Product Vision

This feature directly supports several key product principles and objectives:

**Security First**: The unified logging and error handling system will include comprehensive security features including input sanitization, controlled output destinations, protected error payloads, and safeguards against log or fault injection attacks. All diagnostics will be auditable and secure by design.

**Developer Experience**: By providing a single, intuitive API, the feature dramatically improves the developer experience by eliminating the need to learn and maintain four different logging interfaces and numerous bespoke error helpers. The function-based approach enables better debugging with IDE support, breakpoints, and clear stack traces for both logs and error flows.

**Performance**: The unified system will be optimized for embedded systems with efficient memory usage, compile-time filtering, and minimal runtime overhead, supporting the project's performance requirements for sub-second response times even along error paths.

**Open Source & Educational Value**: The clean, well-documented logging and error handling stack will serve as a reference implementation for embedded systems diagnostics, supporting the project's educational objectives.

**Standards Compliance**: The system will support structured logging formats, ONVIF SOAP fault requirements, and integration with standard monitoring systems, supporting the project's compliance and observability objectives.

## Requirements

### Requirement 1: Single Unified API

**User Story:** As a developer working on the ONVIF project, I want a single logging API that works consistently across all modules, so that I can focus on implementing features rather than learning multiple logging systems.

#### Acceptance Criteria

1. WHEN a developer includes the unified logging header THEN they SHALL have access to all logging functionality through a single API
2. WHEN a developer calls `log_error("message")` THEN the system SHALL output the message with consistent formatting regardless of which module calls it
3. WHEN a developer needs contextual logging THEN they SHALL be able to use `log_error_ctx(context, "message")` with service and operation context
4. WHEN a developer needs enhanced debugging THEN they SHALL be able to use `log_error_at("message")` to include source file, function, and line information

### Requirement 2: Clean Break from Legacy Systems

**User Story:** As a project maintainer, I want to eliminate all legacy logging implementations, so that the codebase follows DRY principles and has no duplicated functionality.

#### Acceptance Criteria

1. WHEN the unified logging system is implemented THEN all platform_log_* calls SHALL be replaced with unified log_* calls
2. WHEN the unified logging system is implemented THEN all service_log_* calls SHALL be replaced with unified log_*_ctx calls
3. WHEN the unified logging system is implemented THEN all direct stdio logging calls SHALL be replaced with appropriate log_* calls
4. WHEN the implementation is complete THEN no legacy logging files SHALL exist in the codebase
5. WHEN the implementation is complete THEN the build system SHALL only include the unified logging implementation

### Requirement 3: Function-Based Interface for Debugging

**User Story:** As a developer debugging issues, I want to be able to set breakpoints on logging functions and step through the code, so that I can effectively troubleshoot problems.

#### Acceptance Criteria

1. WHEN a developer sets a breakpoint on `log_error()` THEN the debugger SHALL stop at the function entry point
2. WHEN a developer steps through logging code THEN they SHALL see clear function call stacks without macro expansions
3. WHEN the system encounters an error THEN the IDE SHALL provide proper autocomplete and type checking for logging function parameters
4. WHEN a developer uses the logging functions THEN they SHALL receive compile-time type safety and parameter validation

### Requirement 4: Contextual Logging Support

**User Story:** As a system administrator monitoring the ONVIF camera, I want log messages to include service and operation context, so that I can quickly identify which component is generating messages and trace request flows.

#### Acceptance Criteria

1. WHEN an ONVIF service processes a request THEN log messages SHALL include the service name (e.g., "device_service", "media_service")
2. WHEN an ONVIF operation is executed THEN log messages SHALL include the operation name (e.g., "get_capabilities", "get_profiles")
3. WHEN a multi-step operation is performed THEN log messages SHALL include session identifiers for request tracing
4. WHEN viewing logs THEN service and operation context SHALL be clearly formatted and easily parseable

### Requirement 5: Performance Optimization

**User Story:** As an embedded systems developer, I want the logging system to have minimal performance impact, so that it doesn't affect the camera's real-time video streaming and response times.

#### Acceptance Criteria

1. WHEN a log level is disabled THEN the logging call SHALL return immediately with minimal CPU overhead (<10 cycles)
2. WHEN logging is active THEN the system SHALL use less than 1% of available CPU time for logging operations
3. WHEN the system is under load THEN logging SHALL not block video streaming or ONVIF operations
4. WHEN memory is constrained THEN the logging system SHALL use fixed memory buffers without dynamic allocation

### Requirement 6: Simple Configuration Management

**User Story:** As a system administrator, I want predictable logging configuration that stays stable at runtime, so that operational workflows remain simple.

#### Acceptance Criteria

1. WHEN the system starts THEN logging level and hostname settings SHALL be loaded from the application configuration and cached for the duration of the process.
2. WHEN administrators need to adjust log verbosity THEN they SHALL restart the service; no runtime API SHALL exist to modify log level.
3. WHEN stdout becomes unavailable THEN the logger SHALL fall back to stderr and emit a single error describing the degraded mode.
4. WHEN the configuration specifies an unsupported log level THEN the system SHALL clamp it to the nearest supported level and emit a warning once per boot.

### Requirement 7: Standardized Log Format

**User Story:** As a system administrator analyzing logs, I want all log messages to follow a consistent, parseable format, so that automated tooling can rely on the structure.

#### Acceptance Criteria

1. WHEN any log message is output THEN it SHALL follow the exact format `YYYY-MM-DD HH:MM:SS,mmm LEVEL [HOSTNAME] component.path.identifier Message text`.
2. WHEN timestamps are generated THEN they SHALL use ISO 8601 date and time with millisecond precision (e.g., `2018-10-25 11:56:35,008`).
3. WHEN level names are rendered THEN they SHALL use one of `TRACE`, `DEBUG`, `INFO`, `NOTICE`, `WARNING`, `ERROR`, or `FATAL`.
4. WHEN hostnames are emitted THEN they SHALL be enclosed in square brackets and derived from the sanitized hostname cache.
5. WHEN component paths are emitted THEN they SHALL use dot notation and originate from the logging context or fallback default.
6. WHEN a formatted log line is produced THEN it SHALL contain no raw control characters other than space or tab.

## Non-Functional Requirements

### Code Architecture and Modularity
- **Single Responsibility Principle**: The unified logging system SHALL be contained in dedicated files (`unified_logging.h/.c`) with clear separation of concerns
- **Modular Design**: Logging functionality SHALL be independent of other systems and usable by any component
- **Dependency Management**: The logging system SHALL have minimal dependencies (only standard C libraries and platform utilities)
- **Clear Interfaces**: The API SHALL provide clean, documented interfaces for basic logging, contextual logging, and configuration management

### Performance
- **Response Time**: Logging calls SHALL complete in less than 100 microseconds for active log levels
- **Memory Usage**: The logging system SHALL use no more than 32KB of memory for simple buffering
- **CPU Overhead**: Disabled log levels SHALL have zero runtime cost through early return optimization
- **Concurrency**: The logging system SHALL be thread-safe with minimal locking overhead

### Security
- **Input Validation**: All log message formatting SHALL prevent buffer overflows and format string attacks
- **Information Disclosure**: The logging system SHALL not log sensitive information (passwords, tokens) by default
- **Injection Prevention**: Log messages SHALL be sanitized to prevent log injection attacks

### Reliability
- **Fault Tolerance**: The logging system SHALL never cause application crashes, even with invalid inputs
- **Error Handling**: Internal logging errors SHALL be handled gracefully with fallback to stderr output
- **Resource Management**: All allocated resources SHALL be properly cleaned up during shutdown
- **Consistency**: Log message formatting SHALL be consistent across all components and contexts

### Usability
- **API Simplicity**: Developers SHALL be able to use basic logging with a single function call
- **Clear Documentation**: All logging functions SHALL have comprehensive Doxygen documentation with examples
- **Migration Support**: Automated scripts SHALL be provided to convert existing logging calls to the unified API
- **IDE Integration**: The function-based API SHALL provide excellent IDE support with autocomplete and parameter hints

### Requirement 8: Security and Sanitization

**User Story:** As a security engineer, I want every log message to be sanitized before it is emitted, so that malicious input cannot corrupt log streams or leak sensitive content.

#### Acceptance Criteria

1. WHEN a log message exceeds 1024 bytes THEN it SHALL be truncated with a `...` suffix and a warning SHALL be emitted once per component.
2. WHEN a log message contains control characters below ASCII 0x20 (excluding tab) or 0x7F THEN they SHALL be replaced with spaces before output.
3. WHEN a log message contains invalid UTF-8 sequences THEN they SHALL be replaced with `?` characters prior to emission.
4. WHEN sensitive keys such as `password`, `token`, `secret`, or `apikey` appear in log context THEN their values SHALL be redacted to `***` automatically.
5. WHEN a logging function formats a message THEN the implementation SHALL reject `%n` specifiers and enforce bounded `vsnprintf` usage.

### Requirement 9: Configuration Behavior

**User Story:** As an operator, I want predictable logging behaviour tied to startup configuration, so that runtime changes do not introduce instability.

#### Acceptance Criteria

1. WHEN the application starts THEN logging level and hostname settings SHALL be loaded once from `logging_settings` and cached for runtime use.
2. WHEN administrators need to change log level THEN they SHALL restart the service; no runtime API SHALL exist to mutate levels.
3. WHEN stdout becomes unavailable THEN logging SHALL fail over to stderr and emit a single error describing the degraded mode.
4. WHEN configuration specifies an unsupported log level THEN the system SHALL clamp it to the nearest supported value and log a warning once per boot.

### Requirement 10: Migration Tooling

**User Story:** As a maintainer, I want automated assistance migrating legacy logging calls to the unified API, so that the transition is fast and free of human error.

#### Acceptance Criteria

1. WHEN the migration script runs THEN it SHALL locate and rewrite `platform_log_*`, `service_log_*`, and `printf`-style logging calls to the new API.
2. WHEN the migration script completes THEN it SHALL generate a report listing any calls it could not convert automatically.
3. WHEN the migration script executes THEN it SHALL preserve existing include ordering and coding standards.
4. WHEN developers run unit tests after migration THEN no legacy logging references SHALL remain.

### Requirement 11: Unified Error Handling API

**User Story:** As a developer working on ONVIF services, I want a single error handling API that produces structured error objects, so that I can consistently propagate failures without duplicating boilerplate.

#### Acceptance Criteria

1. WHEN a developer includes the unified error handling header THEN they SHALL gain access to context creation, error result builders, and protocol translation helpers through a single API surface.
2. WHEN a service constructs an error via the API THEN it SHALL return a typed `error_result_t` object containing error category, message, remediation hint, and protocol metadata.
3. WHEN the API exposes functions THEN all return codes SHALL use predefined `ONVIF_*` constants—no magic numbers are permitted.
4. WHEN developers review the public header THEN every function SHALL include complete Doxygen documentation with usage examples and threading notes.

### Requirement 12: Consistent Protocol Responses

**User Story:** As a maintainer of SOAP and HTTP integrations, I want all error responses to follow the same mappings and sanitization rules, so that clients receive predictable faults and status codes.

#### Acceptance Criteria

1. WHEN a service uses the unified error API to generate an HTTP response THEN the resulting status code SHALL be selected from a centralized mapping table tied to error patterns.
2. WHEN the API produces a SOAP fault via gSOAP THEN the fault code, string, actor, and detail fields SHALL conform to ONVIF 2.5 requirements and match the mapping table definitions.
3. WHEN a fault is generated THEN the response payload SHALL include a correlation identifier that matches the associated log entry.
4. WHEN error detail strings originate from user input THEN they SHALL pass through the same sanitization rules used by the logging subsystem before emission.

### Requirement 13: Error and Logging Correlation

**User Story:** As an SRE responsible for monitoring the camera, I want the logging and error handling systems to be tightly integrated, so that I can correlate faults, metrics, and remediation steps in seconds.

#### Acceptance Criteria

1. WHEN the unified error API handles a failure THEN it SHALL emit a structured log entry at the appropriate severity level using the unified logging system.
2. WHEN an error is logged THEN the log entry SHALL contain a correlation identifier, service context, and error category that match the generated error result.
3. WHEN the system aggregates diagnostics THEN counters for each error category SHALL be exposed via the existing metrics interface or an extensible hook for future telemetry collectors.
4. WHEN the same error pattern occurs repeatedly THEN throttling rules SHALL prevent duplicate log spam while preserving at least one entry per minute per pattern.

### Requirement 14: Error Handling Migration and Validation

**User Story:** As a project maintainer, I want all existing ad-hoc error handling logic replaced with the unified system, so that the codebase remains consistent and maintainable.

#### Acceptance Criteria

1. WHEN the implementation completes THEN all calls to legacy helpers (e.g., `error_handle_*`, inline SOAP builders, direct HTTP status assignments) SHALL use the unified API instead.
2. WHEN static analysis or unit tests run THEN no files SHALL include legacy error headers or define duplicate error enums.
3. WHEN the migration script executes THEN it SHALL flag any residual direct SOAP fault construction or HTTP response manipulation for manual cleanup.
4. WHEN regression tests execute THEN they SHALL assert that every error path emits both a structured log and a standardized protocol response.

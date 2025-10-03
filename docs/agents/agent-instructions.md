# Agent Instructions - Anyka AK3918 Project

## When Working on This Project

### Mandatory Requirements

1. **ALWAYS use bash** for all terminal commands and file operations
2. **ALWAYS use native cross-compilation** for development tasks and building
3. **ALWAYS test compilation** using the native make command before suggesting changes
4. **ALWAYS follow the include ordering** standards strictly
5. **MANDATORY: Update Doxygen documentation** for ALL code changes
6. **MANDATORY: Regenerate HTML documentation** after any code changes
7. **MANDATORY: Perform comprehensive code review** before approving changes (see [Review Prompt](review-prompt.md))
8. **MANDATORY: Use existing utilities** and avoid code duplication
9. **Reference the `cross-compile/anyka_reference/akipc/` tree** when reverse-engineering or implementing new features
10. **Use SD-card testing** as the primary development workflow

## Documentation Workflow (MANDATORY)

1. Make code changes
2. Update Doxygen comments for all modified functions
3. Update file headers if needed
4. Run documentation generation: `make -C cross-compile/onvif docs`
5. Verify documentation in `cross-compile/onvif/docs/html/index.html`
6. Test compilation to ensure changes work
7. **Perform comprehensive code review**
8. Commit changes

## Agent Workflow & Best Practices (MANDATORY)

### Pre-Development Checklist

- [ ] **Understand the task** - Read requirements carefully and ask clarifying questions
- [ ] **Check existing code** - Review related files and utilities before implementing
- [ ] **Plan the approach** - Break down complex tasks into smaller, manageable steps
- [ ] **Identify dependencies** - Understand what other components might be affected
- [ ] **Review coding standards** - Ensure compliance with project guidelines

### Development Process

#### 1. Code Implementation

- Follow include ordering standards strictly
- Use existing utilities to avoid code duplication
- Implement proper error handling and validation
- Add comprehensive Doxygen documentation
- Use consistent naming conventions and formatting

#### 2. Code Quality Validation (MANDATORY)

- **Run linting script**: `./cross-compile/onvif/scripts/lint_code.sh --check`
- **Run formatting script**: `./cross-compile/onvif/scripts/format_code.sh --check`
- **Fix all linting issues** before proceeding
- **Fix all formatting issues** before proceeding
- **Verify function ordering** compliance with project guidelines
- **Check global variable placement** at top of files after includes

#### 3. Mock Testing Validation (MANDATORY) - **MANDATORY** for all unit test changes

- **Mock Framework Compliance**: Verify only CMocka mocks are used (no other mocking frameworks)
- **Mock Implementation Pattern**: Check all mocks use `__wrap_` prefix and `--wrap` linker mechanism
- **Mock File Structure**: Verify mock files follow `*_mock.c` and `*_mock.h` pattern
- **Mock Function Implementation**: Validate all mock functions use standard CMocka patterns:
  - `function_called()` for call tracking
  - `check_expected_*()` for parameter validation
  - `mock()` for return value configuration
- **Mock Configuration**: Verify tests use `will_return()` and `expect_*()` functions correctly
- **Mock State Management**: Check proper mock initialization and cleanup in test setup/teardown
- **Mock Parameter Validation**: Ensure all mock functions validate parameters using CMocka functions
- **Mock Call Verification**: Verify tests check mock function calls and parameters
- **Mock Error Simulation**: Confirm error conditions are properly tested using mocks
- **Mock Documentation**: Validate all mock functions have proper Doxygen documentation
- **Mock Isolation**: Ensure each test is independent and doesn't rely on mock state from other tests
- **Mock Coverage**: Verify all external dependencies are properly mocked
- **Mock Naming**: Check mock functions follow naming conventions (`__wrap_<function_name>`)
- **Mock File Organization**: Verify mocks are properly organized in `tests/src/mocks/` directory
- **Mock Testing Requirements**: Confirm no real hardware, network, or file system operations in unit tests

#### 4. Testing and Validation

- Test compilation with native make
- Run static analysis tools
- Perform memory leak testing
- Test with various input conditions
- Verify ONVIF compliance

#### 5. Documentation and Review

- Update Doxygen documentation
- Regenerate HTML documentation
- Perform comprehensive code review
- Update any affected documentation
- Test documentation generation

#### 6. Integration and Deployment

- Test with SD-card payload
- Verify functionality on target device
- Check for regression issues
- Update version control
- Document any breaking changes

### Quality Assurance Checklist

- [ ] **Code Quality**: Follows all coding standards and best practices
- [ ] **Security**: Input validation, buffer management, authentication
- [ ] **Performance**: Efficient algorithms, memory usage, I/O operations
- [ ] **Maintainability**: Clear code structure, documentation, error handling
- [ ] **ONVIF Compliance**: Proper service implementation and protocol handling
- [ ] **Testing**: Comprehensive testing and validation
- [ ] **Documentation**: Complete and up-to-date documentation

### Common Pitfalls to Avoid

- **Code Duplication**: Always check for existing utilities before implementing new code
- **Memory Leaks**: Ensure all allocated resources are properly freed
- **Buffer Overflows**: Use safe string functions and bounds checking
- **Missing Error Handling**: Handle all error conditions gracefully
- **Incomplete Documentation**: Update documentation for all code changes
- **Security Vulnerabilities**: Validate all inputs and use secure coding practices
- **Performance Issues**: Profile critical code paths and optimize bottlenecks

## Memory & Context

- This project focuses on reverse-engineering Anyka AK3918 camera firmware and developing custom implementations
- **Reference implementation**: `cross-compile/anyka_reference/akipc/` contains the authoritative vendor code for understanding camera behavior
- **Component libraries**: `cross-compile/anyka_reference/component/` and `platform/` provide reusable drivers and hardware abstraction
- Native cross-compilation tools are used for consistent builds in WSL Ubuntu
- SD-card payload testing is the primary iteration method for safe device testing
- ONVIF 2.5 implementation is the current focus area with full service compliance
- Platform abstraction and logging integration are key architectural concerns
- **Bash is mandatory** for all terminal operations in WSL Ubuntu development environment
- **Documentation updates are mandatory** for all code changes with Doxygen compliance
- **Code review is mandatory** for all code changes with comprehensive security and performance analysis
- **Utility usage is mandatory** - no code duplication allowed, always use shared utilities
- **Security is paramount** - all code must be secure and robust with proper input validation
- **ONVIF compliance is critical** - must follow ONVIF 2.5 specification for all services
- **Performance matters** - optimize for embedded camera systems with efficient resource usage
- **Testing is essential** - comprehensive testing before deployment including integration tests

# Error Handling & Debugging - Anyka AK3918 Project

## Error Handling Guidelines (MANDATORY)

### Error Code Strategy

Use consistent error handling throughout:

- **Define clear error codes** for all failure conditions
- **Use ONVIF-compliant error codes** where applicable
- **Provide meaningful error messages** for debugging
- **Log errors with appropriate severity levels**
- **Handle errors gracefully** without crashing

### Error Code Constants (MANDATORY)

- **NEVER use magic numbers** for return codes (`return -1`, `return 0`)
- **ALWAYS use predefined constants** for all function return values
- **Module-specific constants** defined in module headers (e.g., `HTTP_AUTH_SUCCESS`, `HTTP_AUTH_ERROR_NULL`)
- **Global constants** defined in `utils/error/error_handling.h` (e.g., `ONVIF_SUCCESS`, `ONVIF_ERROR_INVALID`)
- **Enforcement**: All code reviews must verify constant usage instead of magic numbers

#### Examples

```c
// ✅ CORRECT - Using predefined constants
return HTTP_AUTH_SUCCESS;
return HTTP_AUTH_ERROR_NULL;
return ONVIF_SUCCESS;
return ONVIF_ERROR_INVALID;

// ❌ INCORRECT - Using magic numbers
return 0;
return -1;
return -2;
return 1;
```

### Error Handling Patterns

#### Function Return Values

```c
/**
 * @brief Process input data safely with bounds checking
 * @param input Input string to process (must not be NULL)
 * @return Length of processed data on success, -1 on error
 * @note Input is validated and buffer overflow is prevented
 */
int process_data(const char* input) {
    if (!input) {
        return ONVIF_ERROR_INVALID_PARAMETER;  // Input validation
    }

    char buffer[100];
    if (strlen(input) >= sizeof(buffer)) {
        return ONVIF_ERROR_BUFFER_OVERFLOW;  // Buffer overflow prevention
    }

    strncpy(buffer, input, sizeof(buffer) - 1);
    buffer[sizeof(buffer) - 1] = '\0';  // Null termination

    return (int)strlen(buffer);
}
```

#### Memory Management Error Handling

```c
/**
 * @brief Create a new media profile with proper memory management
 * @return Pointer to created profile on success, NULL on failure
 * @note Caller must free the returned profile using free()
 */
struct media_profile* create_profile() {
    struct media_profile* profile = malloc(sizeof(struct media_profile));
    if (!profile) {
        return NULL;  // Error handling
    }

    memset(profile, 0, sizeof(struct media_profile));
    strncpy(profile->token, "MainProfile", sizeof(profile->token) - 1);
    profile->token[sizeof(profile->token) - 1] = '\0';

    return profile;
}
```

## Debugging Tools & Techniques

### Debugging Tools

Use appropriate debugging techniques:

- **Enable debug logging** in development builds
- **Use `gdb` or similar debugger** for complex issues (see [Coredump Analysis](coredump-analysis-prompt.md))
- **Add debug prints** for critical code paths
- **Use memory debugging tools** (valgrind, AddressSanitizer)
- **Test with various input conditions** and edge cases

### Logging Strategy

Implement comprehensive logging:

- **Use different log levels** (DEBUG, INFO, WARN, ERROR, FATAL)
- **Log all significant events** and state changes
- **Include context information** in log messages
- **Use structured logging** for better analysis
- **Rotate log files** to prevent disk space issues

### Memory Debugging

#### Common Memory Issues

- **Memory Leaks**: Use valgrind to detect leaks
- **Buffer Overflows**: Use AddressSanitizer for detection
- **Use-After-Free**: Enable debugging tools to catch these
- **Double Free**: Check for multiple free() calls on same pointer
- **Uninitialized Memory**: Use memory debugging tools

#### Memory Debugging Commands

```bash
# Run with valgrind for memory leak detection
valgrind --leak-check=full --show-leak-kinds=all ./onvifd

# Run with AddressSanitizer for buffer overflow detection
gcc -fsanitize=address -g -o onvifd onvifd.c
./onvifd

# Run with ThreadSanitizer for race condition detection
gcc -fsanitize=thread -g -o onvifd onvifd.c
./onvifd
```

### Thread Debugging

#### Common Threading Issues

- **Race Conditions**: Use ThreadSanitizer
- **Deadlocks**: Use gdb with thread analysis
- **Mutex Issues**: Check proper locking/unlocking
- **Thread Safety**: Verify shared resource access

#### Thread Debugging Commands

```bash
# Debug with gdb for thread analysis
gdb ./onvifd
(gdb) set follow-fork-mode child
(gdb) run
(gdb) info threads
(gdb) thread apply all bt

# Run with ThreadSanitizer
gdb -ex "set environment TSAN_OPTIONS=halt_on_error=1" ./onvifd
```

## Testing Strategy

### Comprehensive Testing Approach

- **Unit tests** for individual functions
- **Integration tests** for service interactions
- **Performance tests** for critical paths
- **Security tests** for input validation
- **Regression tests** for bug fixes

### Test Case Design

#### Unit Test Examples

```c
/**
 * @brief Test memory manager initialization
 */
static void test_unit_memory_manager_init(void **state) {
    (void)state;  // Unused parameter

    // Test successful initialization
    assert_int_equal(memory_manager_init(), MEMORY_SUCCESS);

    // Test double initialization
    assert_int_equal(memory_manager_init(), MEMORY_ERROR_ALREADY_INITIALIZED);

    // Test cleanup
    memory_manager_cleanup();
    assert_int_equal(memory_manager_init(), MEMORY_SUCCESS);
}
```

#### Integration Test Examples

```c
/**
 * @brief Test PTZ absolute move functionality
 */
static void test_integration_ptz_absolute_move_functionality(void **state) {
    (void)state;  // Unused parameter

    // Setup
    onvif_ptz_init();

    // Test valid move
    assert_int_equal(onvif_ptz_absolute_move("Profile1", 90.0f, 0.0f, 1.0f), ONVIF_SUCCESS);

    // Test invalid profile
    assert_int_equal(onvif_ptz_absolute_move("InvalidProfile", 90.0f, 0.0f, 1.0f), ONVIF_ERROR_INVALID_PARAMETER);

    // Cleanup
    onvif_ptz_cleanup();
}
```

## Common Debugging Scenarios

### ONVIF Service Issues

#### Service Not Starting

1. **Check configuration files** for syntax errors
2. **Verify port availability** (no conflicts)
3. **Check log files** for error messages
4. **Test with minimal configuration**
5. **Verify dependencies** are installed

#### SOAP Request Failures

1. **Validate XML structure** of requests
2. **Check ONVIF compliance** of responses
3. **Verify authentication** mechanisms
4. **Test with known working clients**
5. **Enable debug logging** for SOAP processing

#### RTSP Streaming Issues

1. **Check video encoder** configuration
2. **Verify network connectivity**
3. **Test with different clients** (VLC, ffplay)
4. **Check bandwidth** and performance
5. **Validate H.264 encoding** parameters

### Platform Integration Issues

#### Hardware Communication Failures

1. **Check device file permissions**
2. **Verify hardware initialization** sequence
3. **Test with reference implementation**
4. **Check Anyka SDK** integration
5. **Validate platform abstraction** layer

#### PTZ Control Problems

1. **Verify motor connections**
2. **Check calibration** settings
3. **Test individual axes** separately
4. **Validate coordinate systems**
5. **Check for mechanical issues**

### Performance Issues

#### Memory Usage

1. **Profile memory allocation** patterns
2. **Check for memory leaks** with valgrind
3. **Optimize buffer sizes** for embedded system
4. **Use memory pools** for frequent allocations
5. **Monitor memory usage** over time

#### CPU Performance

1. **Profile critical code paths**
2. **Optimize algorithms** for embedded constraints
3. **Check for unnecessary loops**
4. **Use appropriate data structures**
5. **Consider threading** for I/O operations

## Debugging Best Practices

### Development Environment

- **Use debug builds** for development
- **Enable all debug logging** during development
- **Use static analysis tools** regularly
- **Test on target hardware** when possible
- **Keep reference implementation** available

### Error Reporting

- **Include context information** in error messages
- **Use consistent error codes** across modules
- **Log errors with appropriate severity**
- **Provide recovery suggestions** when possible
- **Document error conditions** in code

### Testing Strategy

- **Test error conditions** explicitly
- **Use boundary value testing**
- **Test with invalid inputs**
- **Verify error recovery** mechanisms
- **Test under stress conditions**

## SD Card Testing Workflow

### Safe Testing Process

```bash
# 1. Build the ONVIF binary
cd cross-compile/onvif && make

# 2. Verify binary exists
test -f out/onvifd && echo "Build successful"

# 3. Copy to SD card payload
cp out/onvifd ../../SD_card_contents/anyka_hack/usr/bin/

# 4. Copy configuration if changed
cp configs/anyka_cfg.ini ../../SD_card_contents/anyka_hack/onvif/

# 5. Deploy to SD card and test on actual device
# (Safe testing - leaves original firmware intact)
```

### Validation Checklist

- [ ] Binary builds successfully without errors
- [ ] File size is reasonable for embedded system
- [ ] Dependencies are properly linked
- [ ] Configuration files are up-to-date
- [ ] SD card payload structure is maintained
- [ ] Device boots with SD card
- [ ] ONVIF services start correctly
- [ ] Web interface is accessible
- [ ] RTSP streaming works
- [ ] PTZ control functions properly

## Emergency Recovery

### When Things Go Wrong

1. **Remove SD card** to restore original firmware
2. **Check UART logs** for error messages
3. **Verify hardware connections**
4. **Test with known working configuration**
5. **Use reference implementation** for comparison
6. **Check for hardware failures**
7. **Restore from backup** if available

### Recovery Commands

```bash
# Check device status via UART
echo "status" > /dev/ttyUSB0

# Restart services
killall onvifd
/usr/bin/onvifd &

# Check log files
tail -f /var/log/onvif.log

# Verify configuration
cat /onvif/anyka_cfg.ini
```

# Security Guidelines (MANDATORY)

## Input Validation

**ALL user inputs must be properly validated and sanitized**:

- Check for NULL pointers before dereferencing
- Validate string lengths and bounds
- Sanitize XML/SOAP input to prevent injection attacks
- Validate numeric ranges and types
- Check for buffer overflows in all string operations

## Buffer Management

**Prevent buffer overflows and underflows**:

- Use `strncpy()` instead of `strcpy()`
- Always null-terminate strings
- Check array bounds before access
- Use fixed-size buffers with proper bounds checking
- Avoid `scanf()` with unbounded format strings

## Memory Security

**Prevent memory-related vulnerabilities**:

- Initialize all allocated memory with `memset()` or `calloc()`
- Free all allocated resources in error paths
- Check for use-after-free vulnerabilities
- Validate pointer arithmetic safety
- Use stack variables when possible, heap when necessary

## Network Security

**Secure network communications**:

- Validate all network input before processing
- Use proper error message handling (no information leakage)
- Implement proper authentication and authorization
- Validate SOAP/XML input to prevent injection

## Authentication & Authorization

- Implement proper credential handling
- Use secure password storage mechanisms
- Validate session management
- Ensure proper access controls for all endpoints
- Implement rate limiting for authentication attempts

## ONVIF Compliance Requirements (MANDATORY)

### Service Implementation

Complete implementation of all required services:

- **Device Service**: Device information, capabilities, system date/time
- **Media Service**: Profile management, stream URIs, video/audio configurations
- **PTZ Service**: Pan/tilt/zoom control, preset management, continuous move
- **Imaging Service**: Image settings, brightness, contrast, saturation controls

### SOAP Protocol

Proper XML/SOAP request/response handling:

- Correct XML namespace usage
- Proper SOAP envelope structure
- ONVIF-compliant error codes and messages
- Proper content-type headers

### WS-Discovery

Correct multicast discovery implementation:

- Proper UDP multicast socket handling
- Correct discovery message format
- Proper device announcement and probe responses

### RTSP Streaming

Proper H.264 stream generation and delivery:

- Correct SDP format for stream descriptions
- Proper RTP packetization
- Correct stream URI generation
- Proper authentication for RTSP streams

### Error Codes

ONVIF-compliant error reporting:

- Use standard ONVIF error codes
- Provide meaningful error messages
- Handle all error conditions gracefully

## Security Testing

**Validate security measures**:

- Test input validation with malicious inputs
- Verify authentication and authorization mechanisms
- Check for information leakage in error messages
- Test network security with various attack vectors

## Common Security Patterns

### Safe String Operations

```c
// ❌ BAD - Unsafe string operations
char buffer[100];
strcpy(buffer, user_input);  // Buffer overflow risk
sprintf(buffer, "%s", user_input);  // Format string vulnerability

// ✅ GOOD - Safe string operations
char buffer[100];
strncpy(buffer, user_input, sizeof(buffer) - 1);
buffer[sizeof(buffer) - 1] = '\0';  // Ensure null termination
snprintf(buffer, sizeof(buffer), "%s", user_input);  // Bounded format
```

### Input Validation

```c
// ❌ BAD - No input validation
int process_data(char* input) {
    return strlen(input);  // NULL pointer dereference risk
}

// ✅ GOOD - Proper input validation
int process_data(const char* input) {
    if (!input) {
        return -1;  // Input validation
    }

    size_t len = strlen(input);
    if (len > MAX_INPUT_LENGTH) {
        return -1;  // Length validation
    }

    return (int)len;
}
```

### Memory Management

```c
// ❌ BAD - Memory leak
void create_profile() {
    struct media_profile* profile = malloc(sizeof(struct media_profile));
    profile->token = "MainProfile";
    // Missing free() - memory leak
}

// ✅ GOOD - Proper memory management
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

## Security Checklist

**Before deploying any code, verify**:

- [ ] **Input Validation**: All inputs are validated and sanitized
- [ ] **Buffer Management**: No buffer overflows or underflows
- [ ] **Memory Security**: Proper memory allocation and cleanup
- [ ] **Network Security**: Secure communication protocols
- [ ] **Authentication**: Proper credential handling
- [ ] **Authorization**: Access controls for all endpoints
- [ ] **Error Handling**: No information leakage in error messages
- [ ] **ONVIF Compliance**: Proper service implementation
- [ ] **Security Testing**: Malicious input testing completed

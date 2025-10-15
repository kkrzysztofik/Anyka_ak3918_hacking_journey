# Code Quality Patterns for Unified Configuration System

This document outlines new code quality patterns and best practices introduced by the unified configuration system refactoring.

**Last Updated:** 2025-10-16
**Scope:** Cross-compile/onvif ONVIF daemon project

---

## Table of Contents

1. [Overview](#overview)
2. [Configuration Access Patterns](#configuration-access-patterns)
3. [Error Handling](#error-handling)
4. [Thread Safety](#thread-safety)
5. [Testing Patterns](#testing-patterns)
6. [Migration Guide](#migration-guide)
7. [Best Practices](#best-practices)

---

## Overview

The unified configuration system introduces a new, modern approach to configuration management that improves code quality by:

- **Centralizing configuration access** - Single source of truth eliminates duplicated config code
- **Type safety** - Schema-driven validation catches errors at compile and runtime
- **Thread safety** - Built-in locking and generation counters prevent race conditions
- **Testability** - INI-based configuration enables easy test isolation
- **Maintainability** - Clear separation of concerns between runtime and storage layers

---

## Configuration Access Patterns

### Pattern 1: Simple Configuration Reading

**Before (Old Pattern):**
```c
// Scattered throughout codebase
const char* http_port_str = platform_config_get_string("onvif", "http_port", "8080");
int http_port = atoi(http_port_str);  // Manual conversion, error-prone
if (http_port <= 0 || http_port > 65535) {
    // Inconsistent validation in multiple places
    http_port = 8080;
}
```

**After (New Pattern):**
```c
// Unified approach
int http_port = 8080;  // Default value
config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &http_port);
// Validation handled by unified system - no manual checks needed
```

**Why This Is Better:**
- Type safety - No manual string conversions
- Centralized validation - All bounds checking in one place
- Consistent error handling - All sections use same approach
- Easier testing - Configuration can be overridden per test

### Pattern 2: Configuration Updates with Persistence

**Before (Old Pattern):**
```c
// Scattered manual persistence code
void onvif_service_update_setting(const char* key, const char* value) {
    // Store in memory
    struct onvif_settings* settings = get_settings();
    strncpy(settings->custom_field, value, sizeof(settings->custom_field) - 1);

    // Try to save to disk (inconsistently implemented)
    if (platform_config_save("onvif", key, value) != 0) {
        // Error handling varies wildly
        platform_log_warning("Save failed");
    }
}
```

**After (New Pattern):**
```c
// Unified approach
void onvif_service_update_setting(const char* key, int value) {
    // Single function call handles both memory and persistence
    int result = config_runtime_set_int(CONFIG_SECTION_ONVIF, key, value);

    if (result != ONVIF_SUCCESS) {
        // Consistent error handling
        platform_log_error("Configuration update failed: %d", result);
        return;
    }

    // Configuration is automatically queued for async persistence
    // No manual save() calls needed
}
```

**Why This Is Better:**
- Atomic operations - Memory and disk always in sync
- Async persistence - Non-blocking I/O improves responsiveness
- Consistent error codes - All sections use same error handling
- Automatic validation - Type checking before persistence

### Pattern 3: Service Initialization with Configuration

**Before (Old Pattern):**
```c
int onvif_media_init(void) {
    // Manual configuration loading
    const char* width_str = platform_config_get_string("media", "width", "1920");
    const char* height_str = platform_config_get_string("media", "height", "1080");
    int width = atoi(width_str);
    int height = atoi(height_str);

    // Manual validation
    if (width < 160 || width > 2048) width = 1920;
    if (height < 120 || height > 2048) height = 1080;

    // Inconsistent initialization logic
    return platform_media_init(width, height);
}
```

**After (New Pattern):**
```c
int onvif_media_init(void) {
    // Get configuration from unified system
    const struct application_config* config = config_runtime_snapshot();

    // All validation already done
    int width = config->media ? config->media->width : 1920;
    int height = config->media ? config->media->height : 1080;

    return platform_media_init(width, height);
}
```

**Why This Is Better:**
- Simpler code - No manual parsing or validation
- Type-safe access - Direct structure access, no string conversions
- Consistent defaults - All defaults in schema, not scattered in code
- Thread-safe snapshot - Generation counter prevents stale reads

---

## Error Handling

### Unified Error Codes

The unified configuration system uses standard ONVIF error codes:

```c
// Success case
if (config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 8080) == ONVIF_SUCCESS) {
    platform_log_info("Port updated successfully");
}

// Error cases
int result = config_runtime_get_int(CONFIG_SECTION_NETWORK, "timeout", &timeout);
switch (result) {
    case ONVIF_SUCCESS:
        // Configuration read successfully
        break;
    case ONVIF_ERROR_NOT_FOUND:
        // Configuration parameter not in schema
        platform_log_error("Unknown parameter");
        break;
    case ONVIF_VALIDATION_FAILED:
        // Value out of bounds or invalid format
        platform_log_error("Invalid configuration value");
        break;
    case ONVIF_ERROR_NOT_INITIALIZED:
        // Configuration system not initialized
        platform_log_error("Configuration system not ready");
        break;
    default:
        platform_log_error("Unexpected error: %d", result);
        break;
}
```

### Logging Best Practices

```c
// ✅ Good: Use structured logging with context
platform_log_info("Loading configuration from %s", config_file);
if (config_runtime_init(app_config) != ONVIF_SUCCESS) {
    platform_log_error("Failed to initialize configuration system");
    return -1;
}

// ✅ Good: Log configuration changes
int old_port = http_port;
if (config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", new_port) == ONVIF_SUCCESS) {
    platform_log_info("HTTP port changed: %d -> %d", old_port, new_port);
}

// ❌ Bad: Logging with insufficient context
platform_log_warning("Config error");

// ❌ Bad: Logging credential information
platform_log_info("User '%s' password: %s", username, password);  // SECURITY RISK!

// ✅ Good: Sanitized logging
platform_log_info("User '%s' authentication attempt", username);
```

---

## Thread Safety

### Generation Counters for Change Detection

```c
// Get current generation
uint32_t gen_before = config_runtime_get_generation();

// Perform configuration operations
config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 9000);

// Check if configuration changed
uint32_t gen_after = config_runtime_get_generation();
if (gen_after != gen_before) {
    platform_log_info("Configuration was updated");
    // Might need to restart services if significant settings changed
}
```

### Thread-Safe Configuration Snapshot

```c
// Thread 1: Update configuration
config_runtime_set_int(CONFIG_SECTION_PTZ, "max_pan_speed", 90);

// Thread 2: Read configuration (safe)
const struct application_config* config = config_runtime_snapshot();
if (config && config->ptz) {
    int max_speed = config->ptz->max_pan_speed;  // Always consistent
}

// Thread 3: Continue reading even during updates (safe)
// The snapshot pointer remains valid until next update
```

### Pattern: Retry Loop for Configuration-Based Initialization

```c
int onvif_service_init_with_config_timeout(int max_retries) {
    for (int attempt = 0; attempt < max_retries; attempt++) {
        // Get latest configuration (thread-safe snapshot)
        const struct application_config* config = config_runtime_snapshot();
        if (!config) {
            platform_log_warning("Configuration not ready, attempt %d/%d", attempt + 1, max_retries);
            platform_sleep_ms(100);
            continue;
        }

        // Try to initialize with current configuration
        int result = platform_service_init(
            config->server->worker_threads,
            config->server->max_connections
        );

        if (result == PLATFORM_SUCCESS) {
            platform_log_info("Service initialized successfully");
            return ONVIF_SUCCESS;
        }

        platform_log_warning("Initialization failed, attempt %d/%d", attempt + 1, max_retries);
        platform_sleep_ms(100);
    }

    platform_log_error("Service initialization failed after %d attempts", max_retries);
    return ONVIF_ERROR;
}
```

---

## Testing Patterns

### Pattern 1: Test Configuration Isolation

**Setup/Teardown:**
```c
int setup_test_with_config(void** state) {
    // Initialize memory manager
    memory_manager_init();

    // Allocate test state
    test_state_t* test = calloc(1, sizeof(test_state_t));

    // Allocate configuration structures
    test->app_config = calloc(1, sizeof(struct application_config));
    test->app_config->onvif = calloc(1, sizeof(struct onvif_settings));

    // Enable real functions (for integration tests)
    config_mock_use_real_function(true);

    // Initialize configuration system
    config_runtime_init(test->app_config);

    // Load test configuration from INI file
    config_storage_load("configs/test_config.ini", &test->config);

    *state = test;
    return 0;
}

int teardown_test_with_config(void** state) {
    test_state_t* test = (test_state_t*)*state;

    // Cleanup configuration system
    config_runtime_cleanup();

    // Free allocated memory
    if (test->app_config) {
        if (test->app_config->onvif) free(test->app_config->onvif);
        free(test->app_config);
    }

    free(test);
    memory_manager_cleanup();

    return 0;
}
```

### Pattern 2: Testing Configuration Validation

```c
void test_onvif_http_port_validation(void** state) {
    test_state_t* test = (test_state_t*)*state;

    // Test minimum valid value
    assert_int_equal(ONVIF_SUCCESS,
        config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 1));

    // Test maximum valid value
    assert_int_equal(ONVIF_SUCCESS,
        config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 65535));

    // Test out-of-bounds: too high
    assert_int_equal(ONVIF_VALIDATION_FAILED,
        config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 65536));

    // Test out-of-bounds: too low
    assert_int_equal(ONVIF_VALIDATION_FAILED,
        config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 0));

    // Verify value didn't change after invalid update
    int current_port;
    config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &current_port);
    assert_int_equal(65535, current_port);  // Last valid value
}
```

### Pattern 3: Testing Configuration Persistence

```c
void test_config_persistence_async(void** state) {
    test_state_t* test = (test_state_t*)*state;

    int new_port = 9000;

    // Update configuration (queued for persistence)
    assert_int_equal(ONVIF_SUCCESS,
        config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", new_port));

    // Check persistence queue has pending operation
    int pending = config_runtime_get_persistence_status();
    assert_true(pending > 0);

    // Process persistence queue
    assert_int_equal(ONVIF_SUCCESS, config_runtime_process_persistence_queue());

    // Reload configuration to verify persistence
    config_storage_reload("configs/test_config.ini");

    int reloaded_port;
    config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &reloaded_port);
    assert_int_equal(new_port, reloaded_port);
}
```

---

## Migration Guide

### Step 1: Update Service Initialization

**Before:**
```c
int onvif_device_init(void) {
    g_device_config.manufacturer = "Anyka";
    g_device_config.model = platform_config_get_string("device", "model", "AK3918");
    // ... more manual initialization
}
```

**After:**
```c
int onvif_device_init(void) {
    const struct application_config* config = config_runtime_snapshot();
    if (!config || !config->device) {
        platform_log_error("Configuration not available");
        return ONVIF_ERROR_NOT_INITIALIZED;
    }

    // Use configuration directly
    const char* manufacturer = config->device->manufacturer;
    const char* model = config->device->model;
    // ... initialization uses config values
}
```

### Step 2: Replace Manual Configuration Updates

**Before:**
```c
void http_server_set_max_connections(int max_conn) {
    g_http_server_config.max_connections = max_conn;
    platform_config_save("server", "max_connections", itoa(max_conn));
    // ... more manual logic
}
```

**After:**
```c
void http_server_set_max_connections(int max_conn) {
    // Single unified call handles everything
    if (config_runtime_set_int(CONFIG_SECTION_SERVER, "max_connections", max_conn) != ONVIF_SUCCESS) {
        platform_log_error("Failed to update max_connections");
        return;
    }
    // Persistence happens automatically
}
```

### Step 3: Update Error Handling

**Before:**
```c
if (result != 0) {
    printf("Error code: %d\n", result);  // Unclear error
}
```

**After:**
```c
if (result != ONVIF_SUCCESS) {
    switch (result) {
        case ONVIF_ERROR_NOT_FOUND:
            platform_log_error("Configuration parameter not found");
            break;
        case ONVIF_VALIDATION_FAILED:
            platform_log_error("Configuration validation failed");
            break;
        default:
            platform_log_error("Configuration error: %d", result);
    }
}
```

---

## Best Practices

### 1. Always Check Configuration Availability

```c
// ✅ Good: Check for NULL before access
const struct application_config* config = config_runtime_snapshot();
if (config && config->network) {
    use_device_ip(config->network->device_ip);
} else {
    use_default_ip("192.168.1.1");
}

// ❌ Bad: Assume configuration exists
const struct application_config* config = config_runtime_snapshot();
use_device_ip(config->network->device_ip);  // Crash if NULL
```

### 2. Use Generation Counters for Change Detection

```c
// ✅ Good: Detect configuration changes
static uint32_t last_gen = 0;
uint32_t current_gen = config_runtime_get_generation();

if (current_gen != last_gen) {
    platform_log_info("Configuration changed, reloading");
    reload_all_settings();
    last_gen = current_gen;
}

// ❌ Bad: Re-read configuration on every access
// Performance issue - constant disk reads
const struct application_config* config = config_runtime_snapshot();
```

### 3. Batch Configuration Updates

```c
// ✅ Good: Batch updates to reduce persistence queue entries
config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 8080);
config_runtime_set_string(CONFIG_SECTION_DEVICE, "manufacturer", "Anyka");
config_runtime_set_int(CONFIG_SECTION_SERVER, "worker_threads", 4);
// Single process_persistence_queue() call persists all changes

// ❌ Bad: Process persistence queue after each update
config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 8080);
config_runtime_process_persistence_queue();  // Unnecessary disk write
config_runtime_set_string(CONFIG_SECTION_DEVICE, "manufacturer", "Anyka");
config_runtime_process_persistence_queue();  // Unnecessary disk write
```

### 4. Validate Configuration During Initialization

```c
// ✅ Good: Validate critical settings at startup
int onvif_daemon_init(void) {
    config_runtime_init(app_config);

    // Validate critical configuration values
    int http_port;
    if (config_runtime_get_int(CONFIG_SECTION_ONVIF, "http_port", &http_port) != ONVIF_SUCCESS) {
        platform_log_error("Invalid HTTP port configuration");
        return ONVIF_ERROR;
    }

    if (http_port <= 0 || http_port > 65535) {
        platform_log_error("HTTP port out of valid range");
        return ONVIF_ERROR;
    }

    // All services can proceed with confidence
    return ONVIF_SUCCESS;
}
```

### 5. Document Configuration Dependencies

```c
/**
 * @brief Initialize networking layer
 *
 * @requires CONFIG_SECTION_NETWORK.device_ip - Must be configured before calling
 * @requires CONFIG_SECTION_SERVER.worker_threads - Optional, defaults to 4
 *
 * @return ONVIF_SUCCESS on success, error code on failure
 */
int network_layer_init(void) {
    const struct application_config* config = config_runtime_snapshot();

    // Validate required configuration
    if (!config || !config->network || !config->network->device_ip[0]) {
        platform_log_error("Required configuration not available: network.device_ip");
        return ONVIF_ERROR_NOT_INITIALIZED;
    }

    // Use optional configuration with defaults
    int worker_threads = config->server ? config->server->worker_threads : 4;

    return platform_network_init(config->network->device_ip, worker_threads);
}
```

---

## Performance Considerations

### Configuration Snapshot Efficiency

The `config_runtime_snapshot()` returns a read-only pointer to the current configuration. This is very efficient:

```c
// Efficient: No copying, just pointer to internal structure
const struct application_config* config = config_runtime_snapshot();

// Safe: Can be called repeatedly without performance cost
for (int i = 0; i < 1000; i++) {
    config = config_runtime_snapshot();
    process_with_config(config);
}
```

### Async Persistence Benefits

Configuration updates are queued and persisted asynchronously:

```c
// Returns immediately without blocking
config_runtime_set_int(CONFIG_SECTION_ONVIF, "http_port", 8080);

// Other operations continue unblocked
do_other_work();

// Persistence happens in background or when explicitly requested
config_runtime_process_persistence_queue();
```

---

## Summary

The unified configuration system introduces modern code quality patterns that:

- **Eliminate duplication** - Single source of truth for configuration
- **Improve type safety** - Schema-driven validation at compile and runtime
- **Enhance testability** - INI-based configuration enables easy test isolation
- **Increase maintainability** - Clear separation between runtime and storage
- **Enable scalability** - Support for 4 stream profiles and 8 user accounts

Follow these patterns consistently across all ONVIF services to maintain code quality and system reliability.

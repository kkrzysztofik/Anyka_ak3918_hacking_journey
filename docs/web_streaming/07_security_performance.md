# Security and Performance Considerations

## Overview

This module covers security hardening, performance optimization, and operational considerations for browser-compatible video streaming implementation.

## Security Framework

### 1. Authentication and Access Control

#### Stream Access Authentication
```c
// Authentication for streaming endpoints
typedef struct {
    char username[64];
    char token[128];
    time_t expires;
    uint32_t permissions;
} stream_auth_token_t;

int authenticate_streaming_request(const http_request_t* request,
                                 stream_auth_token_t* auth_token) {
    // Extract authentication from headers or query parameters
    const char* auth_header = get_header_value(request->headers, "Authorization");
    const char* token_param = get_query_parameter(request->uri, "token");

    if (auth_header) {
        return validate_bearer_token(auth_header, auth_token);
    } else if (token_param) {
        return validate_query_token(token_param, auth_token);
    }

    return SECURITY_ERROR_NO_AUTH;
}

// Token-based authentication for streaming
static int validate_bearer_token(const char* auth_header, stream_auth_token_t* token) {
    if (strncmp(auth_header, "Bearer ", 7) != 0) {
        return SECURITY_ERROR_INVALID_TOKEN;
    }

    const char* token_value = auth_header + 7;

    // Validate token against stored sessions
    return lookup_auth_token(token_value, token);
}
```

#### Rate Limiting Implementation
```c
// Rate limiting per client IP
typedef struct {
    char client_ip[INET_ADDRSTRLEN];
    uint32_t request_count;
    time_t window_start;
    time_t last_request;
} rate_limit_entry_t;

static rate_limit_entry_t g_rate_limits[MAX_RATE_LIMIT_ENTRIES];
static pthread_mutex_t g_rate_limit_mutex = PTHREAD_MUTEX_INITIALIZER;

int check_rate_limit(const char* client_ip) {
    pthread_mutex_lock(&g_rate_limit_mutex);

    rate_limit_entry_t* entry = find_or_create_rate_limit_entry(client_ip);
    time_t now = time(NULL);

    // Reset window if necessary
    if (now - entry->window_start >= RATE_LIMIT_WINDOW_SECONDS) {
        entry->request_count = 0;
        entry->window_start = now;
    }

    entry->request_count++;
    entry->last_request = now;

    int result = (entry->request_count <= MAX_REQUESTS_PER_WINDOW) ?
                 RATE_LIMIT_OK : RATE_LIMIT_EXCEEDED;

    pthread_mutex_unlock(&g_rate_limit_mutex);
    return result;
}
```

### 2. Secure Transport

#### HTTPS/WSS Support
```c
// TLS configuration for secure streaming
typedef struct {
    char cert_file[256];
    char key_file[256];
    char ca_file[256];
    bool require_client_cert;
    char cipher_suite[512];
} tls_config_t;

int configure_secure_streaming(const tls_config_t* tls_config) {
    // Initialize OpenSSL context
    SSL_CTX* ssl_ctx = SSL_CTX_new(TLS_server_method());
    if (!ssl_ctx) {
        return SECURITY_ERROR_SSL_INIT;
    }

    // Load certificate and key
    if (SSL_CTX_use_certificate_file(ssl_ctx, tls_config->cert_file,
                                    SSL_FILETYPE_PEM) <= 0) {
        SSL_CTX_free(ssl_ctx);
        return SECURITY_ERROR_CERT_LOAD;
    }

    if (SSL_CTX_use_PrivateKey_file(ssl_ctx, tls_config->key_file,
                                   SSL_FILETYPE_PEM) <= 0) {
        SSL_CTX_free(ssl_ctx);
        return SECURITY_ERROR_KEY_LOAD;
    }

    // Set cipher preferences
    if (strlen(tls_config->cipher_suite) > 0) {
        SSL_CTX_set_cipher_list(ssl_ctx, tls_config->cipher_suite);
    }

    g_ssl_context = ssl_ctx;
    return SECURITY_SUCCESS;
}
```

### 3. Input Validation and Security

#### Request Validation
```c
// Comprehensive input validation for streaming requests
int validate_streaming_request(const http_request_t* request) {
    // Validate URI length
    if (strlen(request->uri) > MAX_URI_LENGTH) {
        return SECURITY_ERROR_URI_TOO_LONG;
    }

    // Validate for path traversal attempts
    if (strstr(request->uri, "..") || strstr(request->uri, "//")) {
        return SECURITY_ERROR_PATH_TRAVERSAL;
    }

    // Validate profile token format
    const char* profile = get_query_parameter(request->uri, "profile");
    if (profile && !is_valid_profile_token(profile)) {
        return SECURITY_ERROR_INVALID_PROFILE;
    }

    // Validate streaming parameters
    return validate_streaming_parameters(request);
}

static bool is_valid_profile_token(const char* token) {
    if (strlen(token) > MAX_PROFILE_TOKEN_LENGTH) {
        return false;
    }

    // Allow only alphanumeric and underscore
    for (const char* p = token; *p; p++) {
        if (!isalnum(*p) && *p != '_') {
            return false;
        }
    }

    return true;
}
```

#### WebSocket Security
```c
// WebSocket-specific security measures
int secure_websocket_upgrade(const http_request_t* request) {
    // Validate Origin header to prevent CSRF
    const char* origin = get_header_value(request->headers, "Origin");
    if (origin && !is_allowed_origin(origin)) {
        return SECURITY_ERROR_INVALID_ORIGIN;
    }

    // Validate WebSocket key
    const char* ws_key = get_header_value(request->headers, "Sec-WebSocket-Key");
    if (!ws_key || strlen(ws_key) != 24) {
        return SECURITY_ERROR_INVALID_WS_KEY;
    }

    // Additional security checks
    return validate_websocket_headers(request);
}

static bool is_allowed_origin(const char* origin) {
    // Check against allowed origins list
    const char* allowed_origins[] = {
        "https://camera-viewer.example.com",
        "https://localhost:3000",
        NULL
    };

    for (int i = 0; allowed_origins[i]; i++) {
        if (strcmp(origin, allowed_origins[i]) == 0) {
            return true;
        }
    }

    return false;
}
```

### 4. Security Monitoring and Logging

#### Security Event Logging
```c
// Security event logging system
typedef enum {
    SECURITY_EVENT_AUTH_FAILURE,
    SECURITY_EVENT_RATE_LIMIT_EXCEEDED,
    SECURITY_EVENT_INVALID_REQUEST,
    SECURITY_EVENT_SUSPICIOUS_ACTIVITY,
    SECURITY_EVENT_ACCESS_DENIED
} security_event_type_t;

void log_security_event(security_event_type_t event_type,
                       const char* client_ip,
                       const char* details) {
    time_t now = time(NULL);
    char timestamp[32];
    strftime(timestamp, sizeof(timestamp), "%Y-%m-%d %H:%M:%S",
             localtime(&now));

    // Log to security log file
    FILE* security_log = fopen("/var/log/onvif_security.log", "a");
    if (security_log) {
        fprintf(security_log, "[%s] %s from %s: %s\n",
               timestamp, get_security_event_name(event_type),
               client_ip, details);
        fclose(security_log);
    }

    // Check for attack patterns
    detect_security_patterns(client_ip, event_type);
}
```

## Performance Optimization

### 1. Resource Management

#### Memory Pool Management
```c
// Memory pool for streaming buffers
typedef struct {
    uint8_t* buffer;
    size_t size;
    bool in_use;
    pthread_mutex_t mutex;
} buffer_pool_entry_t;

static buffer_pool_entry_t g_buffer_pool[BUFFER_POOL_SIZE];
static pthread_mutex_t g_pool_mutex = PTHREAD_MUTEX_INITIALIZER;

uint8_t* acquire_streaming_buffer(size_t required_size) {
    pthread_mutex_lock(&g_pool_mutex);

    for (int i = 0; i < BUFFER_POOL_SIZE; i++) {
        buffer_pool_entry_t* entry = &g_buffer_pool[i];

        if (!entry->in_use && entry->size >= required_size) {
            pthread_mutex_lock(&entry->mutex);
            entry->in_use = true;
            pthread_mutex_unlock(&g_pool_mutex);
            return entry->buffer;
        }
    }

    pthread_mutex_unlock(&g_pool_mutex);

    // Allocate new buffer if pool exhausted
    return malloc(required_size);
}

void release_streaming_buffer(uint8_t* buffer) {
    pthread_mutex_lock(&g_pool_mutex);

    for (int i = 0; i < BUFFER_POOL_SIZE; i++) {
        buffer_pool_entry_t* entry = &g_buffer_pool[i];

        if (entry->buffer == buffer) {
            entry->in_use = false;
            pthread_mutex_unlock(&entry->mutex);
            pthread_mutex_unlock(&g_pool_mutex);
            return;
        }
    }

    pthread_mutex_unlock(&g_pool_mutex);

    // Buffer not from pool, free it
    free(buffer);
}
```

#### CPU Load Balancing
```c
// Adaptive quality control based on system load
typedef struct {
    double cpu_threshold_high;
    double cpu_threshold_low;
    int quality_step;
    int framerate_step;
    time_t last_adjustment;
} adaptive_quality_config_t;

void monitor_and_adjust_quality(void) {
    static adaptive_quality_config_t config = {
        .cpu_threshold_high = 80.0,
        .cpu_threshold_low = 60.0,
        .quality_step = 5,
        .framerate_step = 2,
        .last_adjustment = 0
    };

    time_t now = time(NULL);
    if (now - config.last_adjustment < QUALITY_ADJUSTMENT_INTERVAL) {
        return;
    }

    double cpu_usage = get_cpu_usage_percentage();

    if (cpu_usage > config.cpu_threshold_high) {
        // Reduce quality to save CPU
        reduce_streaming_quality(&config);
        config.last_adjustment = now;
    } else if (cpu_usage < config.cpu_threshold_low) {
        // Increase quality if CPU allows
        increase_streaming_quality(&config);
        config.last_adjustment = now;
    }
}

static void reduce_streaming_quality(adaptive_quality_config_t* config) {
    // Reduce MJPEG quality
    if (g_mjpeg_config.quality > MIN_JPEG_QUALITY) {
        g_mjpeg_config.quality = max(g_mjpeg_config.quality - config->quality_step,
                                   MIN_JPEG_QUALITY);
    }

    // Reduce frame rate
    if (g_streaming_config.mjpeg_frame_rate > MIN_FRAME_RATE) {
        g_streaming_config.mjpeg_frame_rate = max(
            g_streaming_config.mjpeg_frame_rate - config->framerate_step,
            MIN_FRAME_RATE);
    }
}
```

### 2. Network Optimization

#### Bandwidth Management
```c
// Per-client bandwidth monitoring and control
typedef struct {
    char client_id[64];
    uint64_t bytes_sent;
    time_t window_start;
    double bandwidth_limit; // bytes per second
    bool throttled;
} bandwidth_monitor_t;

int check_bandwidth_limit(const char* client_id, size_t data_size) {
    bandwidth_monitor_t* monitor = get_client_bandwidth_monitor(client_id);
    time_t now = time(NULL);

    // Reset window if necessary
    if (now - monitor->window_start >= BANDWIDTH_WINDOW_SECONDS) {
        monitor->bytes_sent = 0;
        monitor->window_start = now;
        monitor->throttled = false;
    }

    // Check if sending this data would exceed limit
    double elapsed = difftime(now, monitor->window_start);
    if (elapsed > 0) {
        double current_rate = monitor->bytes_sent / elapsed;
        double projected_rate = (monitor->bytes_sent + data_size) / elapsed;

        if (projected_rate > monitor->bandwidth_limit) {
            monitor->throttled = true;
            return BANDWIDTH_LIMIT_EXCEEDED;
        }
    }

    monitor->bytes_sent += data_size;
    return BANDWIDTH_OK;
}
```

#### Connection Optimization
```c
// TCP optimization for streaming
void optimize_streaming_socket(int sockfd) {
    // Set TCP_NODELAY to reduce latency
    int nodelay = 1;
    setsockopt(sockfd, IPPROTO_TCP, TCP_NODELAY, &nodelay, sizeof(nodelay));

    // Increase send buffer size
    int send_buffer = STREAMING_SEND_BUFFER_SIZE;
    setsockopt(sockfd, SOL_SOCKET, SO_SNDBUF, &send_buffer, sizeof(send_buffer));

    // Set keep-alive to detect disconnected clients
    int keepalive = 1;
    setsockopt(sockfd, SOL_SOCKET, SO_KEEPALIVE, &keepalive, sizeof(keepalive));

    // Configure keep-alive parameters
    int keepidle = 30;   // Start after 30 seconds of inactivity
    int keepintvl = 10;  // Probe every 10 seconds
    int keepcnt = 3;     // Give up after 3 failed probes

    setsockopt(sockfd, IPPROTO_TCP, TCP_KEEPIDLE, &keepidle, sizeof(keepidle));
    setsockopt(sockfd, IPPROTO_TCP, TCP_KEEPINTVL, &keepintvl, sizeof(keepintvl));
    setsockopt(sockfd, IPPROTO_TCP, TCP_KEEPCNT, &keepcnt, sizeof(keepcnt));
}
```

### 3. Caching and Storage Management

#### HLS Segment Caching
```c
// Intelligent HLS segment caching
typedef struct {
    char segment_id[64];
    uint8_t* data;
    size_t size;
    time_t created;
    time_t last_accessed;
    uint32_t access_count;
} segment_cache_entry_t;

static segment_cache_entry_t g_segment_cache[MAX_CACHED_SEGMENTS];
static pthread_rwlock_t g_cache_lock = PTHREAD_RWLOCK_INITIALIZER;

uint8_t* get_cached_segment(const char* segment_id, size_t* size) {
    pthread_rwlock_rdlock(&g_cache_lock);

    for (int i = 0; i < MAX_CACHED_SEGMENTS; i++) {
        segment_cache_entry_t* entry = &g_segment_cache[i];

        if (entry->data && strcmp(entry->segment_id, segment_id) == 0) {
            entry->last_accessed = time(NULL);
            entry->access_count++;
            *size = entry->size;

            uint8_t* cached_data = malloc(entry->size);
            memcpy(cached_data, entry->data, entry->size);

            pthread_rwlock_unlock(&g_cache_lock);
            return cached_data;
        }
    }

    pthread_rwlock_unlock(&g_cache_lock);
    return NULL;
}

void cache_segment(const char* segment_id, const uint8_t* data, size_t size) {
    pthread_rwlock_wrlock(&g_cache_lock);

    // Find LRU slot or empty slot
    int target_slot = find_cache_slot_lru();

    segment_cache_entry_t* entry = &g_segment_cache[target_slot];

    // Free existing data
    if (entry->data) {
        free(entry->data);
    }

    // Store new segment
    strcpy(entry->segment_id, segment_id);
    entry->data = malloc(size);
    memcpy(entry->data, data, size);
    entry->size = size;
    entry->created = time(NULL);
    entry->last_accessed = entry->created;
    entry->access_count = 1;

    pthread_rwlock_unlock(&g_cache_lock);
}
```

## Performance Monitoring

### 1. Real-time Metrics Collection

```c
// Comprehensive performance metrics
typedef struct {
    // Throughput metrics
    double frames_per_second;
    double megabits_per_second;
    uint64_t total_bytes_sent;
    uint64_t total_frames_sent;

    // Latency metrics
    double avg_response_time_ms;
    double p95_response_time_ms;
    double p99_response_time_ms;

    // Resource utilization
    double cpu_usage_percent;
    double memory_usage_percent;
    double disk_usage_percent;

    // Client metrics
    int active_mjpeg_clients;
    int active_websocket_clients;
    int active_hls_clients;
    int active_webrtc_clients;

    // Error metrics
    uint64_t connection_errors;
    uint64_t encoding_errors;
    uint64_t network_errors;
} performance_metrics_t;

void collect_performance_metrics(performance_metrics_t* metrics) {
    memset(metrics, 0, sizeof(performance_metrics_t));

    // Collect throughput data
    collect_throughput_metrics(metrics);

    // Collect latency data
    collect_latency_metrics(metrics);

    // Collect resource utilization
    metrics->cpu_usage_percent = get_cpu_usage_percentage();
    metrics->memory_usage_percent = get_memory_usage_percentage();
    metrics->disk_usage_percent = get_disk_usage_percentage();

    // Collect client counts
    collect_client_metrics(metrics);

    // Collect error counts
    collect_error_metrics(metrics);
}
```

### 2. Performance Alerting

```c
// Performance threshold monitoring
typedef struct {
    double cpu_warning_threshold;
    double cpu_critical_threshold;
    double memory_warning_threshold;
    double memory_critical_threshold;
    double response_time_threshold;
    double error_rate_threshold;
} performance_thresholds_t;

void check_performance_thresholds(const performance_metrics_t* metrics,
                                const performance_thresholds_t* thresholds) {
    // CPU usage alerts
    if (metrics->cpu_usage_percent > thresholds->cpu_critical_threshold) {
        send_performance_alert(ALERT_CPU_CRITICAL, metrics->cpu_usage_percent);
        trigger_emergency_load_reduction();
    } else if (metrics->cpu_usage_percent > thresholds->cpu_warning_threshold) {
        send_performance_alert(ALERT_CPU_WARNING, metrics->cpu_usage_percent);
    }

    // Memory usage alerts
    if (metrics->memory_usage_percent > thresholds->memory_critical_threshold) {
        send_performance_alert(ALERT_MEMORY_CRITICAL, metrics->memory_usage_percent);
        trigger_memory_cleanup();
    }

    // Response time alerts
    if (metrics->p95_response_time_ms > thresholds->response_time_threshold) {
        send_performance_alert(ALERT_LATENCY_HIGH, metrics->p95_response_time_ms);
    }
}
```

## Deployment and Operational Considerations

### 1. Service Health Checks

```c
// Health check endpoint for monitoring systems
int handle_health_check_request(const http_request_t* request,
                               char* response, size_t response_size) {
    health_status_t status;
    collect_health_status(&status);

    char health_json[1024];
    format_health_status_json(&status, health_json, sizeof(health_json));

    int http_status = status.overall_healthy ? 200 : 503;
    return create_json_response(response, response_size, http_status, health_json);
}

void collect_health_status(health_status_t* status) {
    status->overall_healthy = true;

    // Check streaming services
    status->mjpeg_healthy = is_mjpeg_service_healthy();
    status->websocket_healthy = is_websocket_service_healthy();
    status->hls_healthy = is_hls_service_healthy();

    if (!status->mjpeg_healthy || !status->websocket_healthy || !status->hls_healthy) {
        status->overall_healthy = false;
    }

    // Check platform integration
    status->platform_healthy = test_platform_integration();
    if (!status->platform_healthy) {
        status->overall_healthy = false;
    }

    // Check resource availability
    status->resources_healthy = check_resource_availability();
    if (!status->resources_healthy) {
        status->overall_healthy = false;
    }
}
```

### 2. Configuration Management

```c
// Dynamic configuration updates
int update_streaming_configuration(const browser_streaming_config_t* new_config) {
    // Validate configuration
    if (validate_streaming_config(new_config) != 0) {
        return CONFIG_ERROR_INVALID;
    }

    // Apply changes safely
    pthread_mutex_lock(&g_config_mutex);

    browser_streaming_config_t old_config = g_streaming_config;
    g_streaming_config = *new_config;

    // Restart services if necessary
    if (config_requires_restart(&old_config, new_config)) {
        restart_affected_services(&old_config, new_config);
    }

    // Save to persistent storage
    int result = save_streaming_config(CONFIG_FILE_PATH);

    pthread_mutex_unlock(&g_config_mutex);
    return result;
}
```

## Implementation Checklist

- [ ] Implement authentication and authorization for streaming endpoints
- [ ] Add rate limiting and CSRF protection
- [ ] Configure HTTPS/WSS support for secure transport
- [ ] Add comprehensive input validation and sanitization
- [ ] Implement security event logging and monitoring
- [ ] Add memory pool management for streaming buffers
- [ ] Implement adaptive quality control based on system load
- [ ] Add bandwidth monitoring and throttling
- [ ] Create segment caching system for HLS
- [ ] Add comprehensive performance metrics collection
- [ ] Implement performance alerting and threshold monitoring
- [ ] Create health check endpoints
- [ ] Add dynamic configuration management

## Performance Targets

### Security Requirements
- **Authentication**: All streaming endpoints protected
- **Rate Limiting**: < 1% false positives
- **Input Validation**: 100% request validation coverage
- **Logging**: All security events logged with <10ms overhead

### Performance Requirements
- **CPU Overhead**: < 20% additional CPU usage for all streaming
- **Memory Usage**: < 100MB additional memory for all streaming services
- **Response Time**: < 100ms average response time for control operations
- **Bandwidth**: Efficient utilization with <5% protocol overhead

### Reliability Requirements
- **Uptime**: 99.9% streaming service availability
- **Recovery**: < 30 seconds recovery from service failures
- **Monitoring**: < 1 minute detection of performance issues
- **Health Checks**: < 5 second health check response time

## Next Steps

This completes the security and performance module. All web streaming implementation modules are now complete. Return to [00_overview.md](00_overview.md) for the complete implementation roadmap.

## Related Documentation

- **Previous**: [06_integration_platform.md](06_integration_platform.md) - Platform integration details
- **Overview**: [00_overview.md](00_overview.md) - Complete implementation plan
- **Templates**: [templates/](templates/) - Security and performance code patterns
- **All Phases**: Complete implementation across all 7 modules
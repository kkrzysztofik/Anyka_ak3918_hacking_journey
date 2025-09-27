# Memory Optimization

## 🎯 Memory Optimization Strategy

### Current Memory Problems
- **128KB allocated per request** regardless of actual size (2-8KB typical)
- **Buffer pool unused** - 1.6MB pre-allocated but 0% utilization
- **90%+ memory waste** on typical SOAP responses
- **Memory fragmentation** from frequent large allocations

### Optimization Approach
1. **Use existing infrastructure** - No new utilities needed
2. **Dynamic sizing** - Allocate exact size needed
3. **Buffer pooling** - Use pre-allocated buffers for medium responses
4. **Memory tracking** - Ensure all allocations use `ONVIF_MALLOC/FREE`

## 🔧 Implementation Patterns

### Pattern 1: Small Responses (<4KB) - Dynamic Buffer

**BEFORE (Current - Wasteful):**
```c
response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);  // 128KB!
strncpy(response->body, soap_response, ONVIF_RESPONSE_BUFFER_SIZE - 1);
```

**AFTER (Using Existing Infrastructure):**
```c
/**
 * @brief Build SOAP response using existing dynamic buffer infrastructure
 * @param response Response structure to populate (must not be NULL)
 * @param soap_content SOAP content to include
 * @return 0 on success, -1 on error
 * @note Uses existing memory_manager utilities per AGENTS.md standards
 */
static int build_response_with_dynamic_buffer(onvif_response_t *response, const char *soap_content) {
    if (!response || !soap_content) {
        return -1;  // Input validation per AGENTS.md
    }

    dynamic_buffer_t response_buffer;
    dynamic_buffer_init(&response_buffer, 0);  // Default size

    // Build SOAP response safely
    dynamic_buffer_append_string(&response_buffer, "<?xml version=\"1.0\"?>");
    dynamic_buffer_appendf(&response_buffer, "<soap:Envelope>%s</soap:Envelope>", soap_content);

    // Use exact-size allocation with proper error handling
    size_t response_length = dynamic_buffer_length(&response_buffer);
    response->body = ONVIF_MALLOC(response_length + 1);
    if (!response->body) {
        dynamic_buffer_cleanup(&response_buffer);
        return -1;
    }

    memcpy(response->body, dynamic_buffer_data(&response_buffer), response_length);
    response->body[response_length] = '\0';
    response->body_length = response_length;

    dynamic_buffer_cleanup(&response_buffer);

    platform_log_debug("Response allocated: %zu bytes (saved %zu bytes)",
                       response_length, ONVIF_RESPONSE_BUFFER_SIZE - response_length);
    return 0;
}
```

### Pattern 2: Medium Responses (4-32KB) - Buffer Pool

**BEFORE (Current - Wasteful):**
```c
response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);  // 128KB!
strncpy(response->body, soap_response, ONVIF_RESPONSE_BUFFER_SIZE - 1);
```

**AFTER (Using Existing Buffer Pool):**
```c
/**
 * @brief Build SOAP response using existing buffer pool infrastructure
 * @param response Response structure to populate (must not be NULL)
 * @param soap_content SOAP content to include
 * @return 0 on success, -1 on error
 * @note Uses existing buffer_pool utilities per AGENTS.md standards
 */
static int build_response_with_buffer_pool(onvif_response_t *response, const char *soap_content) {
    if (!response || !soap_content) {
        return -1;  // Input validation per AGENTS.md
    }

    char* pool_buffer = buffer_pool_get(&g_networking_response_buffer_pool);
    if (pool_buffer) {
        // Build response in pool buffer with safe string functions
        int result = snprintf(pool_buffer, BUFFER_SIZE,
                             "<?xml version=\"1.0\"?><soap:Envelope>%s</soap:Envelope>",
                             soap_content);

        if (result < 0 || result >= BUFFER_SIZE) {
            buffer_pool_return(&g_networking_response_buffer_pool, pool_buffer);
            return -1;  // Buffer too small or encoding error
        }

        size_t actual_length = (size_t)result;
        response->body = ONVIF_MALLOC(actual_length + 1);
        if (!response->body) {
            buffer_pool_return(&g_networking_response_buffer_pool, pool_buffer);
            return -1;
        }

        // Use safe string copy
        strncpy(response->body, pool_buffer, actual_length + 1);
        response->body[actual_length] = '\0';  // Ensure null termination
        response->body_length = actual_length;

        // Return buffer to pool
        buffer_pool_return(&g_networking_response_buffer_pool, pool_buffer);

        platform_log_debug("Pool response: %zu bytes (saved %zu bytes)",
                           actual_length, ONVIF_RESPONSE_BUFFER_SIZE - actual_length);
        return 0;
    } else {
        // Pool exhausted - fall back to exact allocation
        size_t needed = strlen(soap_content) + 64;  // XML wrapper overhead
        response->body = ONVIF_MALLOC(needed);
        if (!response->body) {
            return -1;
        }

        int result = snprintf(response->body, needed,
                             "<?xml version=\"1.0\"?><soap:Envelope>%s</soap:Envelope>",
                             soap_content);
        if (result < 0 || result >= (int)needed) {
            ONVIF_FREE(response->body);
            response->body = NULL;
            return -1;
        }

        response->body_length = (size_t)result;
        platform_log_debug("Fallback response: %zu bytes", response->body_length);
        return 0;
    }
}
```

## 🎯 Service-Specific Implementation

### Device Service Integration

**Target File**: `cross-compile/onvif/src/services/device/onvif_device.c`

**Step 1: Add Required Includes**
```c
// Add to includes section (after onvif_types.h)
#include "utils/memory/memory_manager.h"
#include "networking/common/buffer_pool.h"
```

**Step 2: Replace Allocation Patterns**
```c
// Find and replace patterns like:
// response->body = ONVIF_MALLOC(ONVIF_RESPONSE_BUFFER_SIZE);

// Replace with:
if (estimated_size < 4096) {
    result = build_response_with_dynamic_buffer(response, soap_content);
} else if (estimated_size < 32768) {
    result = build_response_with_buffer_pool(response, soap_content);
} else {
    // Large response - use direct allocation with tracking
    response->body = ONVIF_MALLOC(estimated_size + 1);
    // ... build response ...
}
```

### Media Service Integration

**Target File**: `cross-compile/onvif/src/services/media/onvif_media.c`

**Similar pattern** - Replace 128KB allocations with smart allocation based on content size.

### PTZ Service Integration

**Target File**: `cross-compile/onvif/src/services/ptz/onvif_ptz.c`

**Similar pattern** - Apply same optimization approach.

## 🔍 Memory Usage Analysis

### Current State Measurement
```bash
# Count current 128KB allocations
echo "=== Memory Usage Analysis $(date) ===" >> refactoring_progress.txt
echo "Current 128KB allocations:" >> refactoring_progress.txt
grep -r "ONVIF_RESPONSE_BUFFER_SIZE" cross-compile/onvif/src/services/ --include="*.c" | wc -l >> refactoring_progress.txt

# Count direct malloc usage
echo "Direct malloc usage:" >> refactoring_progress.txt
grep -r "malloc(" cross-compile/onvif/src/services/ --include="*.c" | wc -l >> refactoring_progress.txt

# Count tracked allocation usage
echo "Tracked allocation usage:" >> refactoring_progress.txt
grep -r "ONVIF_MALLOC" cross-compile/onvif/src/services/ --include="*.c" | wc -l >> refactoring_progress.txt
```

### Target State Measurement
```bash
# After optimization - count smart allocation usage
echo "Smart allocation usage:" >> refactoring_progress.txt
grep -r "build_response_with_dynamic_buffer\|build_response_with_buffer_pool" cross-compile/onvif/src/services/ --include="*.c" | wc -l >> refactoring_progress.txt

# Count remaining 128KB allocations (should be 0)
echo "Remaining 128KB allocations:" >> refactoring_progress.txt
grep -r "ONVIF_RESPONSE_BUFFER_SIZE" cross-compile/onvif/src/services/ --include="*.c" | wc -l >> refactoring_progress.txt
```

## 🧪 Memory Testing

### Runtime Memory Testing
```bash
# Test with memory monitoring (if valgrind available)
cd cross-compile/onvif
valgrind --tool=massif --massif-out-file=memory_before.out ./onvifd --test-mode &
sleep 5
kill %1

# Check memory allocation patterns
ms_print memory_before.out | head -20
```

### Memory Leak Testing
```bash
# Test for memory leaks using existing tracking
valgrind --leak-check=full ./onvifd --test-mode 2>&1 | grep -E "(definitely lost|indirectly lost)"
```

## 📊 Expected Memory Improvements

### Memory Usage Transformation
- **Before**: 128KB × 10 requests = **1.28MB waste**
- **After**: <50KB average per request = **<500KB total**
- **Savings**: **65% reduction** in peak memory usage

### Buffer Pool Utilization
- **Before**: 0% utilization (1.6MB unused)
- **After**: >50% utilization for medium responses
- **Efficiency**: Zero allocation overhead for pooled responses

### Memory Tracking Coverage
- **Before**: ~60% coverage (mixed malloc/ONVIF_MALLOC)
- **After**: 100% coverage (all allocations tracked)
- **Benefit**: Complete leak detection and monitoring

## ✅ Success Criteria

- [ ] **90% reduction** in response buffer waste using existing infrastructure
- [ ] **Zero 128KB allocations** for small responses (<4KB)
- [ ] **Buffer pool utilization** >50% (from 0%) using existing `buffer_pool.c`
- [ ] **Memory leaks**: Zero detected with existing memory tracking
- [ ] **MANDATORY**: All allocations use `ONVIF_MALLOC/FREE` for tracking
- [ ] **MANDATORY**: All changes documented with updated Doxygen
- [ ] **MANDATORY**: All coding standards from AGENTS.md followed

## 🔧 Troubleshooting

### Common Issues

**Buffer Pool Not Available**
```bash
# Check if buffer pool is initialized
grep -rn "buffer_pool_init" cross-compile/onvif/src/ --include="*.c"
```

**Dynamic Buffer Not Working**
```bash
# Check if memory manager is included
grep -rn "memory_manager.h" cross-compile/onvif/src/services/ --include="*.c"
```

**Memory Leaks After Changes**
```bash
# Use existing memory tracking to identify leaks
valgrind --leak-check=full ./onvifd --test-mode
```

### Rollback Procedure
```bash
# Restore original files if needed
find cross-compile/onvif/src/services -name "*.backup" -exec bash -c 'mv "$1" "${1%.backup}"' _ {} \;

# Regenerate documentation after rollback
cd cross-compile/onvif && make docs
```

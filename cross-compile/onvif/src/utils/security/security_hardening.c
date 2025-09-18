/**
 * @file security_hardening.c
 * @brief Comprehensive security hardening measures implementation
 */

#include "security_hardening.h"
#include "platform/platform.h"
#include "utils/error/error_handling.h"
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>
#include <ctype.h>
#include <sys/socket.h>
#include <netinet/in.h>
#include <arpa/inet.h>

/* Global security configuration */
static security_level_t g_security_level = SECURITY_LEVEL_BASIC;
static int g_max_requests_per_minute = MAX_REQUESTS_PER_MINUTE;
static int g_rate_limit_window = 60; /* seconds */

/* Rate limiting storage */
#define MAX_RATE_LIMIT_ENTRIES 1000
static rate_limit_entry_t g_rate_limits[MAX_RATE_LIMIT_ENTRIES];
static int g_rate_limit_count = 0;

/* Security initialization */
int security_init(security_level_t level) {
    g_security_level = level;
    
    // Initialize rate limiting
    memset(g_rate_limits, 0, sizeof(g_rate_limits));
    g_rate_limit_count = 0;
    
    ONVIF_LOG_INFO("Security system initialized with level %d\n", level);
    return ONVIF_SUCCESS;
}

void security_cleanup(void) {
    // Clean up rate limiting data
    memset(g_rate_limits, 0, sizeof(g_rate_limits));
    g_rate_limit_count = 0;
    
    ONVIF_LOG_INFO("Security system cleaned up\n");
}




/* Input validation functions */
int security_validate_http_headers(const http_request_t *request, security_context_t *context) {
    if (!request || !context) {
        return ONVIF_ERROR;
    }
    
    // Check for suspicious headers
    if (request->headers) {
        // Check for XSS attempts in headers
        if (strstr(request->headers, "<script") != NULL ||
            strstr(request->headers, "javascript:") != NULL ||
            strstr(request->headers, "vbscript:") != NULL) {
            ONVIF_LOG_ERROR("XSS attempt detected in headers\n");
            return ONVIF_ERROR;
        }
        
        // Check for SQL injection attempts
        if (strstr(request->headers, "'; DROP") != NULL ||
            strstr(request->headers, "UNION SELECT") != NULL ||
            strstr(request->headers, "OR 1=1") != NULL) {
            ONVIF_LOG_ERROR("SQL injection attempt detected in headers\n");
            return ONVIF_ERROR;
        }
    }
    
    return ONVIF_SUCCESS;
}

int security_validate_xml_structure(const char *xml, size_t length, security_context_t *context) {
    ONVIF_VALIDATE_NULL(xml, "xml");
    
    if (length == 0 || length > MAX_INPUT_LENGTH) {
        ONVIF_LOG_ERROR("Invalid XML length: %zu\n", length);
        return ONVIF_ERROR;
    }
    
    // Check for XML bomb attacks
    if (security_detect_xml_bomb(xml, length) != ONVIF_SUCCESS) {
        ONVIF_LOG_ERROR("XML bomb attack detected\n");
        return ONVIF_ERROR;
    }
    
    // Check for XXE attacks
    if (security_detect_xxe_attack(xml, length) != ONVIF_SUCCESS) {
        ONVIF_LOG_ERROR("XXE attack detected\n");
        return ONVIF_ERROR;
    }
    
    // Basic XML structure validation
    int depth = 0;
    int attribute_count = 0;
    
    for (size_t i = 0; i < length; i++) {
        if (xml[i] == '<') {
            depth++;
            if (depth > MAX_XML_DEPTH) {
                ONVIF_LOG_ERROR("XML depth too deep: %d\n", depth);
                return ONVIF_ERROR;
            }
        } else if (xml[i] == '>') {
            depth--;
        } else if (xml[i] == '=') {
            attribute_count++;
            if (attribute_count > MAX_XML_ATTRIBUTES) {
                ONVIF_LOG_ERROR("Too many XML attributes: %d\n", attribute_count);
                return ONVIF_ERROR;
            }
        }
    }
    
    return ONVIF_SUCCESS;
}

int security_sanitize_input(const char *input, char *output, size_t output_size, security_context_t *context) {
    ONVIF_VALIDATE_NULL(input, "input");
    ONVIF_VALIDATE_NULL(output, "output");
    
    if (output_size == 0) {
        return ONVIF_ERROR;
    }
    
    size_t input_len = strlen(input);
    if (input_len >= output_size) {
        ONVIF_LOG_ERROR("Input too long for sanitization: %zu >= %zu\n", input_len, output_size);
        return ONVIF_ERROR;
    }
    
    size_t out_pos = 0;
    for (size_t i = 0; i < input_len && out_pos < output_size - 1; i++) {
        char c = input[i];
        
        // Remove or escape dangerous characters
        switch (c) {
            case '<':
                if (out_pos + 4 < output_size - 1) {
                    strcpy(output + out_pos, "&lt;");
                    out_pos += 4;
                }
                break;
            case '>':
                if (out_pos + 4 < output_size - 1) {
                    strcpy(output + out_pos, "&gt;");
                    out_pos += 4;
                }
                break;
            case '&':
                if (out_pos + 5 < output_size - 1) {
                    strcpy(output + out_pos, "&amp;");
                    out_pos += 5;
                }
                break;
            case '"':
                if (out_pos + 6 < output_size - 1) {
                    strcpy(output + out_pos, "&quot;");
                    out_pos += 6;
                }
                break;
            case '\'':
                if (out_pos + 6 < output_size - 1) {
                    strcpy(output + out_pos, "&apos;");
                    out_pos += 6;
                }
                break;
            case '\0':
                // Skip null bytes
                break;
            default:
                if (isprint((unsigned char)c)) {
                    output[out_pos++] = c;
                }
                break;
        }
    }
    
    output[out_pos] = '\0';
    return ONVIF_SUCCESS;
}

/* Rate limiting functions */
int security_check_rate_limit(const char *client_ip, security_context_t *context) {
    ONVIF_VALIDATE_NULL(client_ip, "client_ip");
    
    time_t now = time(NULL);
    
    // Find existing entry
    for (int i = 0; i < g_rate_limit_count; i++) {
        if (strcmp(g_rate_limits[i].client_ip, client_ip) == 0) {
            // Check if window has expired
            if (now - g_rate_limits[i].window_start >= g_rate_limit_window) {
                // Reset window
                g_rate_limits[i].window_start = now;
                g_rate_limits[i].request_count = 0;
                g_rate_limits[i].is_blocked = 0;
            }
            
            // Check if client is blocked
            if (g_rate_limits[i].is_blocked) {
                return ONVIF_ERROR;
            }
            
            // Check rate limit
            if (g_rate_limits[i].request_count >= g_max_requests_per_minute) {
                g_rate_limits[i].is_blocked = 1;
                ONVIF_LOG_ERROR("Rate limit exceeded for client: %s\n", client_ip);
                return ONVIF_ERROR;
            }
            
            return ONVIF_SUCCESS;
        }
    }
    
    // Add new entry if not found
    if (g_rate_limit_count < MAX_RATE_LIMIT_ENTRIES) {
        strncpy(g_rate_limits[g_rate_limit_count].client_ip, client_ip, sizeof(g_rate_limits[g_rate_limit_count].client_ip) - 1);
        g_rate_limits[g_rate_limit_count].client_ip[sizeof(g_rate_limits[g_rate_limit_count].client_ip) - 1] = '\0';
        g_rate_limits[g_rate_limit_count].window_start = now;
        g_rate_limits[g_rate_limit_count].request_count = 0;
        g_rate_limits[g_rate_limit_count].is_blocked = 0;
        g_rate_limit_count++;
    }
    
    return ONVIF_SUCCESS;
}

int security_update_rate_limit(const char *client_ip, security_context_t *context) {
    ONVIF_VALIDATE_NULL(client_ip, "client_ip");
    
    for (int i = 0; i < g_rate_limit_count; i++) {
        if (strcmp(g_rate_limits[i].client_ip, client_ip) == 0) {
            g_rate_limits[i].request_count++;
            return ONVIF_SUCCESS;
        }
    }
    
    return ONVIF_ERROR;
}

int security_is_client_blocked(const char *client_ip, security_context_t *context) {
    ONVIF_VALIDATE_NULL(client_ip, "client_ip");
    
    for (int i = 0; i < g_rate_limit_count; i++) {
        if (strcmp(g_rate_limits[i].client_ip, client_ip) == 0) {
            return g_rate_limits[i].is_blocked;
        }
    }
    
    return 0; // Not blocked if not found
}

/* Attack detection functions */
int security_detect_sql_injection(const char *input) {
    ONVIF_VALIDATE_NULL(input, "input");
    
    const char *patterns[] = {
        "'; DROP",
        "UNION SELECT",
        "OR 1=1",
        "AND 1=1",
        "EXEC(",
        "EXECUTE(",
        "sp_",
        "xp_",
        NULL
    };
    
    for (int i = 0; patterns[i] != NULL; i++) {
        if (strstr(input, patterns[i]) != NULL) {
            ONVIF_LOG_ERROR("SQL injection pattern detected: %s\n", patterns[i]);
            return ONVIF_ERROR;
        }
    }
    
    return ONVIF_SUCCESS;
}

int security_detect_xss_attack(const char *input) {
    ONVIF_VALIDATE_NULL(input, "input");
    
    const char *patterns[] = {
        "<script",
        "javascript:",
        "vbscript:",
        "onload=",
        "onerror=",
        "onclick=",
        "eval(",
        "document.cookie",
        NULL
    };
    
    for (int i = 0; patterns[i] != NULL; i++) {
        if (strstr(input, patterns[i]) != NULL) {
            ONVIF_LOG_ERROR("XSS pattern detected: %s\n", patterns[i]);
            return ONVIF_ERROR;
        }
    }
    
    return ONVIF_SUCCESS;
}

int security_detect_path_traversal(const char *input) {
    ONVIF_VALIDATE_NULL(input, "input");
    
    const char *patterns[] = {
        "../",
        "..\\",
        "/etc/passwd",
        "/etc/shadow",
        "C:\\",
        "..%2f",
        "..%5c",
        NULL
    };
    
    for (int i = 0; patterns[i] != NULL; i++) {
        if (strstr(input, patterns[i]) != NULL) {
            ONVIF_LOG_ERROR("Path traversal pattern detected: %s\n", patterns[i]);
            return ONVIF_ERROR;
        }
    }
    
    return ONVIF_SUCCESS;
}

int security_detect_xml_bomb(const char *xml, size_t length) {
    ONVIF_VALIDATE_NULL(xml, "xml");
    
    // Check for exponential entity expansion
    const char *bomb_patterns[] = {
        "&lol9;",
        "&lol8;",
        "&lol7;",
        "&lol6;",
        "&lol5;",
        "&lol4;",
        "&lol3;",
        "&lol2;",
        "&lol1;",
        "&lol0;",
        NULL
    };
    
    for (int i = 0; bomb_patterns[i] != NULL; i++) {
        if (strstr(xml, bomb_patterns[i]) != NULL) {
            ONVIF_LOG_ERROR("XML bomb pattern detected: %s\n", bomb_patterns[i]);
            return ONVIF_ERROR;
        }
    }
    
    return ONVIF_SUCCESS;
}

int security_detect_xxe_attack(const char *xml, size_t length) {
    ONVIF_VALIDATE_NULL(xml, "xml");
    
    const char *xxe_patterns[] = {
        "DOCTYPE",
        "SYSTEM",
        "PUBLIC",
        "file://",
        "ftp://",
        "gopher://",
        NULL
    };
    
    for (int i = 0; xxe_patterns[i] != NULL; i++) {
        if (strstr(xml, xxe_patterns[i]) != NULL) {
            ONVIF_LOG_ERROR("XXE pattern detected: %s\n", xxe_patterns[i]);
            return ONVIF_ERROR;
        }
    }
    
    return ONVIF_SUCCESS;
}

/* Logging functions */
void security_log_attack(const char *attack_type, const char *client_ip, const char *details) {
    ONVIF_LOG_ERROR("SECURITY ALERT: %s attack from %s - %s\n", 
                   attack_type ? attack_type : "Unknown", 
                   client_ip ? client_ip : "Unknown", 
                   details ? details : "No details");
}


void security_log_security_event(const char *event_type, const char *client_ip, int severity) {
    const char *severity_str = (severity >= 3) ? "HIGH" : (severity >= 2) ? "MEDIUM" : "LOW";
    ONVIF_LOG_ERROR("SECURITY EVENT [%s]: %s from %s\n", 
                   severity_str, 
                   event_type ? event_type : "Unknown", 
                   client_ip ? client_ip : "Unknown");
}

/* Utility functions */
const char *security_get_client_ip(const http_request_t *request) {
    // This would extract the real client IP from the request
    // For now, return a placeholder
    return "127.0.0.1";
}

time_t security_get_current_time(void) {
    return time(NULL);
}

int security_is_valid_ip(const char *ip) {
    if (!ip) return 0;
    
    // Basic IP validation (IPv4)
    int dots = 0;
    int digits = 0;
    
    for (int i = 0; ip[i] != '\0'; i++) {
        if (ip[i] == '.') {
            dots++;
            if (digits == 0 || digits > 3) return 0;
            digits = 0;
        } else if (isdigit(ip[i])) {
            digits++;
            if (digits > 3) return 0;
        } else {
            return 0;
        }
    }
    
    return (dots == 3 && digits > 0 && digits <= 3);
}

int security_is_private_ip(const char *ip) {
    if (!ip) return 0;
    
    // Check for private IP ranges
    if (strncmp(ip, "192.168.", 8) == 0) return 1;
    if (strncmp(ip, "10.", 3) == 0) return 1;
    if (strncmp(ip, "172.", 4) == 0) {
        // Check for 172.16.0.0 to 172.31.255.255
        int second_octet = atoi(ip + 4);
        if (second_octet >= 16 && second_octet <= 31) return 1;
    }
    
    return 0;
}

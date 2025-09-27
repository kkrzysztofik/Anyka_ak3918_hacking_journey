/**
 * @file security_hardening.c
 * @brief Comprehensive security hardening measures implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "security_hardening.h"

#include "networking/common/connection_manager.h"
#include "networking/http/http_parser.h"
#include "utils/error/error_handling.h"
#include "utils/logging/platform_logging.h"

#include <arpa/inet.h>
#include <ctype.h>
#include <netinet/in.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <time.h>

/* Global security configuration */
static security_level_t g_security_level = SECURITY_LEVEL_BASIC; // NOLINT
static int g_max_requests_per_minute = MAX_REQUESTS_PER_MINUTE;  // NOLINT
static int g_rate_limit_window = 60; /* seconds */               // NOLINT

/* Rate limiting storage */
#define MAX_RATE_LIMIT_ENTRIES 1000
static rate_limit_entry_t g_rate_limits[MAX_RATE_LIMIT_ENTRIES]; // NOLINT
static int g_rate_limit_count = 0;                               // NOLINT

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
int security_validate_http_headers(const http_request_t* request, security_context_t* context) {
  if (!request || !context) {
    return ONVIF_ERROR;
  }

  // Check for suspicious headers by iterating through header array
  if (request->headers && request->header_count > 0) {
    for (size_t i = 0; i < request->header_count; i++) {
      const char* header_value = request->headers[i].value;
      if (!header_value) {
        continue;
      }

      // Check for XSS attempts in header values
      if (strstr(header_value, "<script") != NULL ||
          strstr(header_value, "javascript:") != NULL ||
          strstr(header_value, "vbscript:") != NULL) {
        ONVIF_LOG_ERROR("XSS attempt detected in header '%s': %s\n",
                        request->headers[i].name ? request->headers[i].name : "unknown",
                        header_value);
        return ONVIF_ERROR;
      }

      // Check for SQL injection attempts in header values
      if (strstr(header_value, "'; DROP") != NULL ||
          strstr(header_value, "UNION SELECT") != NULL ||
          strstr(header_value, "OR 1=1") != NULL) {
        ONVIF_LOG_ERROR("SQL injection attempt detected in header '%s': %s\n",
                        request->headers[i].name ? request->headers[i].name : "unknown",
                        header_value);
        return ONVIF_ERROR;
      }
    }
  }

  return ONVIF_SUCCESS;
}

int security_validate_xml_structure(const char* xml, size_t length, security_context_t* context) {
  ONVIF_VALIDATE_NULL(xml, "xml");

  if (length == 0 || length > MAX_INPUT_LENGTH) {
    ONVIF_LOG_ERROR("Invalid XML length: %zu\n", length);
    return ONVIF_ERROR;
  }

  // Log XML content for debugging when validation fails
  ONVIF_LOG_DEBUG("Validating XML structure (length=%zu): %.*s\n", length, (int)length, xml);

  // Check for XML bomb attacks
  if (security_detect_xml_bomb(xml, length) != ONVIF_SUCCESS) {
    ONVIF_LOG_ERROR("XML bomb attack detected\n");
    ONVIF_LOG_DEBUG("XML bomb content: %.*s\n", (int)length, xml);
    return ONVIF_ERROR;
  }

  // Check for XXE attacks
  if (security_detect_xxe_attack(xml, length) != ONVIF_SUCCESS) {
    ONVIF_LOG_ERROR("XXE attack detected\n");
    ONVIF_LOG_DEBUG("XXE attack content: %.*s\n", (int)length, xml);
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
        ONVIF_LOG_DEBUG("XML depth error content: %.*s\n", (int)length, xml);
        return ONVIF_ERROR;
      }
    } else if (xml[i] == '>') {
      depth--;
    } else if (xml[i] == '=') {
      attribute_count++;
      if (attribute_count > MAX_XML_ATTRIBUTES) {
        ONVIF_LOG_ERROR("Too many XML attributes: %d\n", attribute_count);
        ONVIF_LOG_DEBUG("XML attributes error content: %.*s\n", (int)length, xml);
        return ONVIF_ERROR;
      }
    }
  }

  return ONVIF_SUCCESS;
}

/**
 * @brief Escape a single character to HTML entity
 * @param character Character to escape
 * @param output Output buffer
 * @param out_pos Current position in output buffer
 * @param output_size Size of output buffer
 * @return Number of characters written, or 0 if no space
 */
static size_t escape_character(char character, char* output, size_t out_pos, size_t output_size) {
  const char* entity = NULL;
  size_t entity_len = 0;

  switch (character) {
  case '<':
    entity = "&lt;";
    entity_len = 4;
    break;
  case '>':
    entity = "&gt;";
    entity_len = 4;
    break;
  case '&':
    entity = "&amp;";
    entity_len = 5;
    break;
  case '"':
    entity = "&quot;";
    entity_len = 6;
    break;
  case '\'':
    entity = "&apos;";
    entity_len = 6;
    break;
  case '\0':
    return 0; // Skip null bytes
  default:
    if (isprint((unsigned char)character)) {
      if (out_pos < output_size - 1) {
        output[out_pos] = character;
        return 1;
      }
    }
    return 0;
  }

  // Check if we have space for the entity
  if (entity && out_pos + entity_len < output_size) {
    strcpy(output + out_pos, entity);
    return entity_len;
  }

  return 0;
}

int security_sanitize_input(const char* input, char* output, size_t output_size,
                            security_context_t* context) {
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
    char character = input[i];
    size_t written = escape_character(character, output, out_pos, output_size);
    out_pos += written;
  }

  output[out_pos] = '\0';
  return ONVIF_SUCCESS;
}

/* Rate limiting functions */
int security_check_rate_limit(const char* client_ip, security_context_t* context) {
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
    strncpy(g_rate_limits[g_rate_limit_count].client_ip, client_ip,
            sizeof(g_rate_limits[g_rate_limit_count].client_ip) - 1);
    g_rate_limits[g_rate_limit_count]
      .client_ip[sizeof(g_rate_limits[g_rate_limit_count].client_ip) - 1] = '\0';
    g_rate_limits[g_rate_limit_count].window_start = now;
    g_rate_limits[g_rate_limit_count].request_count = 0;
    g_rate_limits[g_rate_limit_count].is_blocked = 0;
    g_rate_limit_count++;
  }

  return ONVIF_SUCCESS;
}

int security_update_rate_limit(const char* client_ip, security_context_t* context) {
  ONVIF_VALIDATE_NULL(client_ip, "client_ip");

  for (int i = 0; i < g_rate_limit_count; i++) {
    if (strcmp(g_rate_limits[i].client_ip, client_ip) == 0) {
      g_rate_limits[i].request_count++;
      return ONVIF_SUCCESS;
    }
  }

  return ONVIF_ERROR;
}

int security_is_client_blocked(const char* client_ip, security_context_t* context) {
  ONVIF_VALIDATE_NULL(client_ip, "client_ip");

  for (int i = 0; i < g_rate_limit_count; i++) {
    if (strcmp(g_rate_limits[i].client_ip, client_ip) == 0) {
      return g_rate_limits[i].is_blocked;
    }
  }

  return 0; // Not blocked if not found
}

/* Attack detection functions */
int security_detect_sql_injection(const char* input) {
  ONVIF_VALIDATE_NULL(input, "input");

  const char* patterns[] = {"'; DROP",  "UNION SELECT", "OR 1=1", "AND 1=1", "EXEC(",
                            "EXECUTE(", "sp_",          "xp_",    NULL};

  for (int i = 0; patterns[i] != NULL; i++) {
    if (strstr(input, patterns[i]) != NULL) {
      ONVIF_LOG_ERROR("SQL injection pattern detected: %s\n", patterns[i]);
      return ONVIF_ERROR;
    }
  }

  return ONVIF_SUCCESS;
}

int security_detect_xss_attack(const char* input) {
  ONVIF_VALIDATE_NULL(input, "input");

  const char* patterns[] = {"<script",  "javascript:", "vbscript:",       "onload=", "onerror=",
                            "onclick=", "eval(",       "document.cookie", NULL};

  for (int i = 0; patterns[i] != NULL; i++) {
    if (strstr(input, patterns[i]) != NULL) {
      ONVIF_LOG_ERROR("XSS pattern detected: %s\n", patterns[i]);
      return ONVIF_ERROR;
    }
  }

  return ONVIF_SUCCESS;
}

int security_detect_path_traversal(const char* input) {
  ONVIF_VALIDATE_NULL(input, "input");

  const char* patterns[] = {"../",  "..\\",  "/etc/passwd", "/etc/shadow",
                            "C:\\", "..%2f", "..%5c",       NULL};

  for (int i = 0; patterns[i] != NULL; i++) {
    if (strstr(input, patterns[i]) != NULL) {
      ONVIF_LOG_ERROR("Path traversal pattern detected: %s\n", patterns[i]);
      return ONVIF_ERROR;
    }
  }

  return ONVIF_SUCCESS;
}

int security_detect_xml_bomb(const char* xml, size_t length) {
  ONVIF_VALIDATE_NULL(xml, "xml");

  // Check for exponential entity expansion
  const char* bomb_patterns[] = {"&lol9;", "&lol8;", "&lol7;", "&lol6;", "&lol5;", "&lol4;",
                                 "&lol3;", "&lol2;", "&lol1;", "&lol0;", NULL};

  for (int i = 0; bomb_patterns[i] != NULL; i++) {
    if (strstr(xml, bomb_patterns[i]) != NULL) {
      ONVIF_LOG_ERROR("XML bomb pattern detected: %s\n", bomb_patterns[i]);
      return ONVIF_ERROR;
    }
  }

  return ONVIF_SUCCESS;
}

int security_detect_xxe_attack(const char* xml, size_t length) {
  ONVIF_VALIDATE_NULL(xml, "xml");

  const char* xxe_patterns[] = {"DOCTYPE", "SYSTEM",    "PUBLIC", "file://",
                                "ftp://",  "gopher://", NULL};

  for (int i = 0; xxe_patterns[i] != NULL; i++) {
    if (strstr(xml, xxe_patterns[i]) != NULL) {
      ONVIF_LOG_ERROR("XXE pattern detected: %s\n", xxe_patterns[i]);
      return ONVIF_ERROR;
    }
  }

  return ONVIF_SUCCESS;
}

/* Logging functions */
void security_log_attack(const char* attack_type, const char* client_ip, const char* details) {
  ONVIF_LOG_ERROR("SECURITY ALERT: %s attack from %s - %s\n", attack_type ? attack_type : "Unknown",
                  client_ip ? client_ip : "Unknown", details ? details : "No details");
}

void security_log_security_event(const char* event_type, const char* client_ip, int severity) {
  const char* severity_str = (severity >= 3) ? "HIGH" : (severity >= 2) ? "MEDIUM" : "LOW";
  ONVIF_LOG_ERROR("SECURITY EVENT [%s]: %s from %s\n", severity_str,
                  event_type ? event_type : "Unknown", client_ip ? client_ip : "Unknown");
}

/* Utility functions */
const char* security_get_client_ip(const connection_t* conn) {
  if (!conn) {
    return "unknown";
  }

  // Return the client IP stored in the connection
  return conn->client_ip;
}

time_t security_get_current_time(void) {
  return time(NULL);
}

int security_is_valid_ip(const char* ip_address) {
  if (!ip_address) {
    return 0;
  }

  // Basic IP validation (IPv4)
  int dots = 0;
  int digits = 0;

  for (int i = 0; ip_address[i] != '\0'; i++) {
    if (ip_address[i] == '.') {
      dots++;
      if (digits == 0 || digits > 3) {
        return 0;
      }
      digits = 0;
    } else if (isdigit(ip_address[i])) {
      digits++;
      if (digits > 3) {
        return 0;
      }
    } else {
      return 0;
    }
  }

  return (dots == 3 && digits > 0 && digits <= 3);
}

int security_is_private_ip(const char* ip_address) {
  if (!ip_address) {
    return 0;
  }

  // Check for private IP ranges
  if (strncmp(ip_address, "192.168.", 8) == 0) {
    return 1;
  }
  if (strncmp(ip_address, "10.", 3) == 0) {
    return 1;
  }
  if (strncmp(ip_address, "172.", 4) == 0) {
    // Check for 172.16.0.0 to 172.31.255.255
    char* end_ptr = NULL;
    long second_octet = strtol(ip_address + 4, &end_ptr, 10);
    if (end_ptr != ip_address + 4 && second_octet >= 16 && second_octet <= 31) {
      return 1;
    }
  }

  return 0;
}

/* Security headers functions */
int security_add_security_headers(http_response_t* response, security_context_t* context) {
  ONVIF_VALIDATE_NULL(response, "response");

  // Add security headers to prevent common attacks
  // X-Content-Type-Options: nosniff
  if (http_response_add_header(response, "X-Content-Type-Options", "nosniff") != 0) {
    ONVIF_LOG_ERROR("Failed to add X-Content-Type-Options header\n");
    return ONVIF_ERROR;
  }

  // X-Frame-Options: DENY
  if (http_response_add_header(response, "X-Frame-Options", "DENY") != 0) {
    ONVIF_LOG_ERROR("Failed to add X-Frame-Options header\n");
    return ONVIF_ERROR;
  }

  // X-XSS-Protection: 1; mode=block
  if (http_response_add_header(response, "X-XSS-Protection", "1; mode=block") != 0) {
    ONVIF_LOG_ERROR("Failed to add X-XSS-Protection header\n");
    return ONVIF_ERROR;
  }

  // Strict-Transport-Security for HTTPS (if applicable)
  if (http_response_add_header(response, "Strict-Transport-Security", "max-age=31536000; includeSubDomains") != 0) {
    ONVIF_LOG_ERROR("Failed to add Strict-Transport-Security header\n");
    return ONVIF_ERROR;
  }

  // Content-Security-Policy: default-src 'none'
  if (http_response_add_header(response, "Content-Security-Policy", "default-src 'none'") != 0) {
    ONVIF_LOG_ERROR("Failed to add Content-Security-Policy header\n");
    return ONVIF_ERROR;
  }

  // Referrer-Policy: no-referrer
  if (http_response_add_header(response, "Referrer-Policy", "no-referrer") != 0) {
    ONVIF_LOG_ERROR("Failed to add Referrer-Policy header\n");
    return ONVIF_ERROR;
  }

  ONVIF_LOG_DEBUG("Security headers added successfully\n");
  return ONVIF_SUCCESS;
}

/* Comprehensive request validation functions */
int security_validate_request(const http_request_t* request, security_context_t* context) {
  ONVIF_VALIDATE_NULL(request, "request");
  ONVIF_VALIDATE_NULL(context, "context");

  // 1. Rate limiting check - first line of defense
  if (security_check_rate_limit(context->client_ip, context) != ONVIF_SUCCESS) {
    ONVIF_LOG_ERROR("Rate limit exceeded for client %s\n", context->client_ip);
    security_log_attack("RATE_LIMIT_EXCEEDED", context->client_ip, "Too many requests");
    return ONVIF_ERROR;
  }

  // 2. Check if client is blocked due to previous attacks
  if (security_is_client_blocked(context->client_ip, context)) {
    ONVIF_LOG_ERROR("Blocked client %s attempted request\n", context->client_ip);
    security_log_attack("BLOCKED_CLIENT_ACCESS", context->client_ip, "Client is blocked");
    return ONVIF_ERROR;
  }

  // 3. HTTP headers validation for attack detection
  if (security_validate_http_headers(request, context) != ONVIF_SUCCESS) {
    ONVIF_LOG_ERROR("HTTP headers validation failed for client %s\n", context->client_ip);
    security_log_attack("MALICIOUS_HEADERS", context->client_ip, "Suspicious HTTP headers detected");
    return ONVIF_ERROR;
  }

  // 4. Update rate limiting counters after successful validation
  security_update_rate_limit(context->client_ip, context);

  return ONVIF_SUCCESS;
}

int security_validate_request_body(const http_request_t* request, security_context_t* context) {
  ONVIF_VALIDATE_NULL(request, "request");
  ONVIF_VALIDATE_NULL(context, "context");

  // Only validate if body is present
  if (!request->body || request->body_length == 0) {
    return ONVIF_SUCCESS;
  }

  // 1. Validate XML/SOAP body for security threats
  if (security_validate_xml_structure(request->body, request->body_length, context) != ONVIF_SUCCESS) {
    ONVIF_LOG_ERROR("XML security validation failed for client %s\n", context->client_ip);
    security_log_attack("MALICIOUS_XML", context->client_ip, "XML bomb or XXE attack detected");
    return ONVIF_ERROR;
  }

  // 2. Check for injection attacks in XML content
  if (security_detect_sql_injection(request->body) != ONVIF_SUCCESS) {
    ONVIF_LOG_ERROR("SQL injection detected in XML body from client %s\n", context->client_ip);
    security_log_attack("SQL_INJECTION", context->client_ip, "SQL injection in XML body");
    return ONVIF_ERROR;
  }

  if (security_detect_xss_attack(request->body) != ONVIF_SUCCESS) {
    ONVIF_LOG_ERROR("XSS attack detected in XML body from client %s\n", context->client_ip);
    security_log_attack("XSS_ATTACK", context->client_ip, "XSS attack in XML body");
    return ONVIF_ERROR;
  }

  return ONVIF_SUCCESS;
}

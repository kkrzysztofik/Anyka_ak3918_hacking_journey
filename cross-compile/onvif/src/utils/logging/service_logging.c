/**
 * @file service_logging.c
 * @brief Service-specific logging utilities implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "service_logging.h"

#include <stdarg.h>
#include <stdio.h>
#include <string.h>
#include <strings.h>

#include "platform/platform.h"
#include "utils/memory/memory_manager.h"

void service_log_init_context(service_log_context_t* context, // NOLINT
                              const char* service_name,       // NOLINT
                              const char* action_name,        // NOLINT
                              service_log_level_t level) {
  if (!context || !service_name) {
    return;
  }

  context->service_name = service_name;
  context->action_name = action_name;
  context->level = level;
}

/* Internal constants for formatting buffers and redaction */
#define SERVICE_LOG_MESSAGE_BUF_SIZE 512
#define SERVICE_LOG_FORMAT_BUF_SIZE  256
#define SERVICE_LOG_AUTH_NAME_LEN    13
#define SERVICE_LOG_REDACT_LEN       10
#define SERVICE_LOG_WSSE_REDACT_LEN  16

void service_log_operation_success(const service_log_context_t* context, const char* operation) {
  if (!context || !operation) {
    return;
  }

  char message[SERVICE_LOG_MESSAGE_BUF_SIZE];
  if (context->action_name) {
    (void)snprintf(message, sizeof(message), "[%s:%s] %s completed successfully", context->service_name, context->action_name, operation);
  } else {
    (void)snprintf(message, sizeof(message), "[%s] %s completed successfully", context->service_name, operation);
  }

  platform_log_info("%s\n", message);
}

void service_log_operation_failure(const service_log_context_t* context, const char* operation, int error_code, const char* error_message) {
  if (!context || !operation) {
    return;
  }

  char message[SERVICE_LOG_MESSAGE_BUF_SIZE];
  if (context->action_name) {
    (void)snprintf(message, sizeof(message), "[%s:%s] %s failed (code: %d): %s", context->service_name, context->action_name, operation, error_code,
                   error_message ? error_message : "Unknown error");
  } else {
    (void)snprintf(message, sizeof(message), "[%s] %s failed (code: %d): %s", context->service_name, operation, error_code,
                   error_message ? error_message : "Unknown error");
  }

  platform_log_error("%s\n", message);
}

void service_log_validation_error(const service_log_context_t* context, const char* field_name, const char* value) {
  if (!context || !field_name) {
    return;
  }

  char message[SERVICE_LOG_MESSAGE_BUF_SIZE];
  if (context->action_name) {
    (void)snprintf(message, sizeof(message), "[%s:%s] Validation failed for field '%s' (value: %s)", context->service_name, context->action_name,
                   field_name, value ? value : "NULL");
  } else {
    (void)snprintf(message, sizeof(message), "[%s] Validation failed for field '%s' (value: %s)", context->service_name, field_name,
                   value ? value : "NULL");
  }

  platform_log_error("%s\n", message);
}

void service_log_config_error(const service_log_context_t* context, const char* config_key, const char* error_message) {
  if (!context || !config_key) {
    return;
  }

  char message[SERVICE_LOG_MESSAGE_BUF_SIZE];
  if (context->action_name) {
    (void)snprintf(message, sizeof(message), "[%s:%s] Configuration error for key '%s': %s", context->service_name, context->action_name, config_key,
                   error_message ? error_message : "Unknown error");
  } else {
    (void)snprintf(message, sizeof(message), "[%s] Configuration error for key '%s': %s", context->service_name, config_key,
                   error_message ? error_message : "Unknown error");
  }

  platform_log_error("%s\n", message);
}

void service_log_platform_error(const service_log_context_t* context, const char* platform_operation, int error_code) {
  if (!context || !platform_operation) {
    return;
  }

  char message[SERVICE_LOG_MESSAGE_BUF_SIZE];
  if (context->action_name) {
    (void)snprintf(message, sizeof(message), "[%s:%s] Platform operation '%s' failed (code: %d)", context->service_name, context->action_name,
                   platform_operation, error_code);
  } else {
    (void)snprintf(message, sizeof(message), "[%s] Platform operation '%s' failed (code: %d)", context->service_name, platform_operation, error_code);
  }

  platform_log_error("%s\n", message);
}

void service_log_not_implemented(const service_log_context_t* context, const char* feature) {
  if (!context || !feature) {
    return;
  }

  char message[SERVICE_LOG_MESSAGE_BUF_SIZE];
  if (context->action_name) {
    (void)snprintf(message, sizeof(message), "[%s:%s] Feature '%s' not implemented", context->service_name, context->action_name, feature);
  } else {
    (void)snprintf(message, sizeof(message), "[%s] Feature '%s' not implemented", context->service_name, feature);
  }

  platform_log_info("%s\n", message);
}

void service_log_timeout(const service_log_context_t* context, const char* operation, int timeout_ms) {
  if (!context || !operation) {
    return;
  }

  char message[SERVICE_LOG_MESSAGE_BUF_SIZE];
  if (context->action_name) {
    memory_safe_snprintf(message, sizeof(message), "[%s:%s] %s timed out after %dms", context->service_name, context->action_name, operation,
                         timeout_ms);
  } else {
    memory_safe_snprintf(message, sizeof(message), "[%s] %s timed out after %dms", context->service_name, operation, timeout_ms);
  }

  platform_log_error("%s\n", message);
}

void service_log_warning(const service_log_context_t* context, const char* format, ...) {
  if (!context || !format) {
    return;
  }

  char message[SERVICE_LOG_MESSAGE_BUF_SIZE];
  char formatted[SERVICE_LOG_FORMAT_BUF_SIZE];

  va_list args;
  va_start(args, format);
  (void)vsnprintf(formatted, sizeof(formatted), format, args);
  va_end(args);

  if (context->action_name) {
    memory_safe_snprintf(message, sizeof(message), "[%s:%s] %s", context->service_name, context->action_name, formatted);
  } else {
    memory_safe_snprintf(message, sizeof(message), "[%s] %s", context->service_name, formatted);
  }

  platform_log_warning("%s\n", message);
}

void service_log_info(const service_log_context_t* context, const char* format, ...) {
  if (!context || !format) {
    return;
  }

  char message[SERVICE_LOG_MESSAGE_BUF_SIZE];
  char formatted[SERVICE_LOG_FORMAT_BUF_SIZE];

  va_list args;
  va_start(args, format);
  (void)vsnprintf(formatted, sizeof(formatted), format, args);
  va_end(args);

  if (context->action_name) {
    memory_safe_snprintf(message, sizeof(message), "[%s:%s] %s", context->service_name, context->action_name, formatted);
  } else {
    memory_safe_snprintf(message, sizeof(message), "[%s] %s", context->service_name, formatted);
  }

  platform_log_info("%s\n", message);
}

void service_log_debug(const service_log_context_t* context, const char* format, ...) {
  if (!context || !format) {
    return;
  }

  char message[SERVICE_LOG_MESSAGE_BUF_SIZE];
  char formatted[SERVICE_LOG_FORMAT_BUF_SIZE];

  va_list args;
  va_start(args, format);
  (void)vsnprintf(formatted, sizeof(formatted), format, args);
  va_end(args);

  if (context->action_name) {
    memory_safe_snprintf(message, sizeof(message), "[%s:%s] %s", context->service_name, context->action_name, formatted);
  } else {
    memory_safe_snprintf(message, sizeof(message), "[%s] %s", context->service_name, formatted);
  }

  platform_log_info("%s\n", message); // Use info level for debug in this implementation
}

void service_log_redact_header_value(const char* header_name, char* header_value) {
  if (!header_name || !header_value) {
    return;
  }
  if (strncasecmp(header_name, "Authorization", (size_t)SERVICE_LOG_AUTH_NAME_LEN) == 0) {
    header_value[0] = '\0';
    (void)strncat(header_value, "<REDACTED>", (size_t)SERVICE_LOG_REDACT_LEN);
  }
}

static char to_lower_char(char input_char) {
  if (input_char >= 'A' && input_char <= 'Z') {
    return (char)(input_char - 'A' + 'a');
  }
  return input_char;
}

static char* ci_strstr_local(char* haystack, const char* needle) {
  if (!haystack || !needle || !*needle) {
    return NULL;
  }
  size_t nlen = strlen(needle);
  for (char* hay = haystack; *hay; ++hay) {
    size_t index = 0;
    while (index < nlen && hay[index]) {
      char hay_char = to_lower_char(hay[index]);
      char needle_char = to_lower_char(needle[index]);
      if (hay_char != needle_char) {
        break;
      }
      index++;
    }
    if (index == nlen) {
      return hay;
    }
  }
  return NULL;
}

void service_log_redact_wsse_password(char* body) {
  if (!body) {
    return;
  }
  const char* open_tag = "<wsse:Password";
  const char* close_tag = "</wsse:Password>";
  char* scan_cursor = body;
  while (1) {
    char* open_pos = ci_strstr_local(scan_cursor, open_tag);
    if (!open_pos) {
      break;
    }
    char* end_open = strchr(open_pos, '>');
    if (!end_open) {
      break;
    }
    char* close_pos = ci_strstr_local(end_open + 1, close_tag);
    if (!close_pos) {
      break;
    }
    char* content_begin = end_open + 1;
    if (content_begin < close_pos) {
      *content_begin = '\0';
      (void)strncat(open_pos, ">***REDACTED***", (size_t)SERVICE_LOG_WSSE_REDACT_LEN);
      (void)strncat(open_pos, close_tag, strlen(close_tag));
    }
    scan_cursor = close_pos + strlen(close_tag);
  }
}

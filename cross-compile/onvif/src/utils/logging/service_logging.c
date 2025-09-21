/**
 * @file service_logging.c
 * @brief Service-specific logging utilities implementation
 * @author kkrzysztofik
 * @date 2025
 */

#include "service_logging.h"

#include <stdarg.h>
#include <stdio.h>

#include "platform/platform.h"
#include "utils/memory/memory_manager.h"

void service_log_init_context(service_log_context_t *context,  // NOLINT
                              const char *service_name,        // NOLINT
                              const char *action_name,         // NOLINT
                              service_log_level_t level) {
  if (!context || !service_name) {
    return;
  }

  context->service_name = service_name;
  context->action_name = action_name;
  context->level = level;
}

void service_log_operation_success(const service_log_context_t *context,
                                   const char *operation) {
  if (!context || !operation) {
    return;
  }

  char message[512];
  if (context->action_name) {
    (void)snprintf(message, sizeof(message),
                   "[%s:%s] %s completed successfully", context->service_name,
                   context->action_name, operation);
  } else {
    (void)snprintf(message, sizeof(message), "[%s] %s completed successfully",
                   context->service_name, operation);
  }

  platform_log_info("%s\n", message);
}

void service_log_operation_failure(const service_log_context_t *context,
                                   const char *operation, int error_code,
                                   const char *error_message) {
  if (!context || !operation) {
    return;
  }

  char message[512];
  if (context->action_name) {
    (void)snprintf(message, sizeof(message), "[%s:%s] %s failed (code: %d): %s",
                   context->service_name, context->action_name, operation,
                   error_code, error_message ? error_message : "Unknown error");
  } else {
    (void)snprintf(message, sizeof(message), "[%s] %s failed (code: %d): %s",
                   context->service_name, operation, error_code,
                   error_message ? error_message : "Unknown error");
  }

  platform_log_error("%s\n", message);
}

void service_log_validation_error(const service_log_context_t *context,
                                  const char *field_name, const char *value) {
  if (!context || !field_name) {
    return;
  }

  char message[512];
  if (context->action_name) {
    (void)snprintf(message, sizeof(message),
                   "[%s:%s] Validation failed for field '%s' (value: %s)",
                   context->service_name, context->action_name, field_name,
                   value ? value : "NULL");
  } else {
    (void)snprintf(message, sizeof(message),
                   "[%s] Validation failed for field '%s' (value: %s)",
                   context->service_name, field_name, value ? value : "NULL");
  }

  platform_log_error("%s\n", message);
}

void service_log_config_error(const service_log_context_t *context,
                              const char *config_key,
                              const char *error_message) {
  if (!context || !config_key) {
    return;
  }

  char message[512];
  if (context->action_name) {
    (void)snprintf(message, sizeof(message),
                   "[%s:%s] Configuration error for key '%s': %s",
                   context->service_name, context->action_name, config_key,
                   error_message ? error_message : "Unknown error");
  } else {
    (void)snprintf(message, sizeof(message),
                   "[%s] Configuration error for key '%s': %s",
                   context->service_name, config_key,
                   error_message ? error_message : "Unknown error");
  }

  platform_log_error("%s\n", message);
}

void service_log_platform_error(const service_log_context_t *context,
                                const char *platform_operation,
                                int error_code) {
  if (!context || !platform_operation) {
    return;
  }

  char message[512];
  if (context->action_name) {
    (void)snprintf(message, sizeof(message),
                   "[%s:%s] Platform operation '%s' failed (code: %d)",
                   context->service_name, context->action_name,
                   platform_operation, error_code);
  } else {
    (void)snprintf(message, sizeof(message),
                   "[%s] Platform operation '%s' failed (code: %d)",
                   context->service_name, platform_operation, error_code);
  }

  platform_log_error("%s\n", message);
}

void service_log_not_implemented(const service_log_context_t *context,
                                 const char *feature) {
  if (!context || !feature) {
    return;
  }

  char message[512];
  if (context->action_name) {
    (void)snprintf(message, sizeof(message),
                   "[%s:%s] Feature '%s' not implemented",
                   context->service_name, context->action_name, feature);
  } else {
    (void)snprintf(message, sizeof(message),
                   "[%s] Feature '%s' not implemented", context->service_name,
                   feature);
  }

  platform_log_info("%s\n", message);
}

void service_log_timeout(const service_log_context_t *context,
                         const char *operation, int timeout_ms) {
  if (!context || !operation) {
    return;
  }

  char message[512];
  if (context->action_name) {
    memory_safe_snprintf(
        message, sizeof(message), "[%s:%s] %s timed out after %dms",
        context->service_name, context->action_name, operation, timeout_ms);
  } else {
    memory_safe_snprintf(message, sizeof(message),
                         "[%s] %s timed out after %dms", context->service_name,
                         operation, timeout_ms);
  }

  platform_log_error("%s\n", message);
}

void service_log_warning(const service_log_context_t *context,
                         const char *format, ...) {
  if (!context || !format) {
    return;
  }

  char message[512];
  char formatted[256];

  va_list args;
  va_start(args, format);
  (void)vsnprintf(formatted, sizeof(formatted), format, args);
  va_end(args);

  if (context->action_name) {
    memory_safe_snprintf(message, sizeof(message), "[%s:%s] %s",
                         context->service_name, context->action_name,
                         formatted);
  } else {
    memory_safe_snprintf(message, sizeof(message), "[%s] %s",
                         context->service_name, formatted);
  }

  platform_log_warning("%s\n", message);
}

void service_log_info(const service_log_context_t *context, const char *format,
                      ...) {
  if (!context || !format) {
    return;
  }

  char message[512];
  char formatted[256];

  va_list args;
  va_start(args, format);
  (void)vsnprintf(formatted, sizeof(formatted), format, args);
  va_end(args);

  if (context->action_name) {
    memory_safe_snprintf(message, sizeof(message), "[%s:%s] %s",
                         context->service_name, context->action_name,
                         formatted);
  } else {
    memory_safe_snprintf(message, sizeof(message), "[%s] %s",
                         context->service_name, formatted);
  }

  platform_log_info("%s\n", message);
}

void service_log_debug(const service_log_context_t *context, const char *format,
                       ...) {
  if (!context || !format) {
    return;
  }

  char message[512];
  char formatted[256];

  va_list args;
  va_start(args, format);
  (void)vsnprintf(formatted, sizeof(formatted), format, args);
  va_end(args);

  if (context->action_name) {
    memory_safe_snprintf(message, sizeof(message), "[%s:%s] %s",
                         context->service_name, context->action_name,
                         formatted);
  } else {
    memory_safe_snprintf(message, sizeof(message), "[%s] %s",
                         context->service_name, formatted);
  }

  platform_log_info(
      "%s\n", message);  // Use info level for debug in this implementation
}

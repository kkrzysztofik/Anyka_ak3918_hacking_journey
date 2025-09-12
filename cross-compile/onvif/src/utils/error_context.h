/**
 * @file error_context.h
 * @brief Enhanced error handling with context and debugging information.
 */

#ifndef ONVIF_ERROR_CONTEXT_H
#define ONVIF_ERROR_CONTEXT_H

#include "error_handling.h"
#include <stdarg.h>

/* Error context structure */
typedef struct {
  int error_code;
  const char *function;
  const char *file;
  int line;
  char message[256];
  char context[512];
} error_context_t;

/* Error context macros */
#define ERROR_CONTEXT_INIT(ctx, code, func, file, line) \
  do { \
    (ctx)->error_code = (code); \
    (ctx)->function = (func); \
    (ctx)->file = (file); \
    (ctx)->line = (line); \
    (ctx)->message[0] = '\0'; \
    (ctx)->context[0] = '\0'; \
  } while(0)

#define ERROR_CONTEXT_SET_MESSAGE(ctx, fmt, ...) \
  do { \
    snprintf((ctx)->message, sizeof((ctx)->message), (fmt), ##__VA_ARGS__); \
  } while(0)

#define ERROR_CONTEXT_SET_CONTEXT(ctx, fmt, ...) \
  do { \
    snprintf((ctx)->context, sizeof((ctx)->context), (fmt), ##__VA_ARGS__); \
  } while(0)

/* Enhanced error handling macros */
#define ONVIF_ERROR_WITH_CONTEXT(code, message, ...) \
  do { \
    error_context_t ctx; \
    ERROR_CONTEXT_INIT(&ctx, (code), __FUNCTION__, __FILE__, __LINE__); \
    ERROR_CONTEXT_SET_MESSAGE(&ctx, (message), ##__VA_ARGS__); \
    onvif_log_error_context(&ctx); \
    return (code); \
  } while(0)

#define ONVIF_ERROR_IF_NULL(ptr, message, ...) \
  do { \
    if ((ptr) == NULL) { \
      ONVIF_ERROR_WITH_CONTEXT(ONVIF_ERROR_NULL, (message), ##__VA_ARGS__); \
    } \
  } while(0)

#define ONVIF_ERROR_IF_INVALID(expr, message, ...) \
  do { \
    if (!(expr)) { \
      ONVIF_ERROR_WITH_CONTEXT(ONVIF_ERROR_INVALID, (message), ##__VA_ARGS__); \
    } \
  } while(0)

#define ONVIF_ERROR_IF_FAIL(expr, message, ...) \
  do { \
    int _ret = (expr); \
    if (_ret != 0) { \
      ONVIF_ERROR_WITH_CONTEXT(_ret, (message), ##__VA_ARGS__); \
    } \
  } while(0)

/* Memory management with error context */
#define ONVIF_MALLOC_WITH_CONTEXT(size, message, ...) \
  ({ \
    void *_ptr = malloc(size); \
    if (!_ptr) { \
      ONVIF_ERROR_WITH_CONTEXT(ONVIF_ERROR_MEMORY, (message), ##__VA_ARGS__); \
    } \
    _ptr; \
  })

#define ONVIF_CALLOC_WITH_CONTEXT(count, size, message, ...) \
  ({ \
    void *_ptr = calloc(count, size); \
    if (!_ptr) { \
      ONVIF_ERROR_WITH_CONTEXT(ONVIF_ERROR_MEMORY, (message), ##__VA_ARGS__); \
    } \
    _ptr; \
  })

/* Function declarations */
void onvif_log_error_context(const error_context_t *ctx);
void onvif_log_error_with_context(int error_code, const char *function, 
                                 const char *file, int line, 
                                 const char *message, ...);
int onvif_get_error_context_string(const error_context_t *ctx, char *buffer, size_t buffer_size);

#endif /* ONVIF_ERROR_CONTEXT_H */

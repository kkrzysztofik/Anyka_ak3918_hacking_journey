/**
 * @file error_context.c
 * @brief Enhanced error handling implementation.
 */

#include "error_context.h"
#include "platform/platform.h"
#include <stdio.h>
#include <stdarg.h>

void onvif_log_error_context(const error_context_t *ctx) {
  if (!ctx) {
    return;
  }
  
  platform_log_error("ERROR [%d] in %s() at %s:%d\n", 
                     ctx->error_code, ctx->function, ctx->file, ctx->line);
  
  if (ctx->message[0] != '\0') {
    platform_log_error("  Message: %s\n", ctx->message);
  }
  
  if (ctx->context[0] != '\0') {
    platform_log_error("  Context: %s\n", ctx->context);
  }
}

void onvif_log_error_with_context(int error_code, const char *function, 
                                 const char *file, int line, 
                                 const char *message, ...) {
  error_context_t ctx;
  ERROR_CONTEXT_INIT(&ctx, error_code, function, file, line);
  
  if (message) {
    va_list args;
    va_start(args, message);
    vsnprintf(ctx.message, sizeof(ctx.message), message, args);
    va_end(args);
  }
  
  onvif_log_error_context(&ctx);
}

int onvif_get_error_context_string(const error_context_t *ctx, char *buffer, size_t buffer_size) {
  if (!ctx || !buffer || buffer_size == 0) {
    return -1;
  }
  
  int len = snprintf(buffer, buffer_size, 
                    "ERROR [%d] in %s() at %s:%d", 
                    ctx->error_code, ctx->function, ctx->file, ctx->line);
  
  if (len < 0 || (size_t)len >= buffer_size) {
    return -1;
  }
  
  if (ctx->message[0] != '\0') {
    int msg_len = snprintf(buffer + len, buffer_size - len, 
                          " - %s", ctx->message);
    if (msg_len < 0 || (size_t)(len + msg_len) >= buffer_size) {
      return -1;
    }
    len += msg_len;
  }
  
  if (ctx->context[0] != '\0') {
    int ctx_len = snprintf(buffer + len, buffer_size - len, 
                          " [%s]", ctx->context);
    if (ctx_len < 0 || (size_t)(len + ctx_len) >= buffer_size) {
      return -1;
    }
    len += ctx_len;
  }
  
  return len;
}

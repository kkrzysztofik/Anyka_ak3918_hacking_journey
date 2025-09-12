/**
 * @file xml_builder.c
 * @brief XML builder utility implementation
 */

#include "xml_builder.h"
#include "utils/error_handling.h"
#include "utils/safe_string.h"
#include <stdio.h>
#include <string.h>
#include <stdarg.h>

#define XML_BUILDER_INDENT_SIZE 2
#define XML_BUILDER_MAX_ATTRIBUTES 16

typedef struct {
  char name[64];
  char value[256];
} xml_attribute_t;

static int xml_builder_append_string(xml_builder_t *builder, const char *str) {
  if (!builder || !str || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  size_t len = strlen(str);
  if (builder->current_pos + len >= builder->buffer_size) {
    builder->error = 1;
    return ONVIF_ERROR;
  }
  
  strcpy(builder->buffer + builder->current_pos, str);
  builder->current_pos += len;
  return ONVIF_SUCCESS;
}

static int xml_builder_append_formatted(xml_builder_t *builder, const char *format, va_list args) {
  if (!builder || !format || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  int result = vsnprintf(builder->buffer + builder->current_pos, 
                        builder->buffer_size - builder->current_pos, format, args);
  
  if (result < 0 || (size_t)result >= builder->buffer_size - builder->current_pos) {
    builder->error = 1;
    return ONVIF_ERROR;
  }
  
  builder->current_pos += result;
  return ONVIF_SUCCESS;
}

/* Wrapper functions for common formatting patterns */
static int xml_builder_append_formatted_1(xml_builder_t *builder, const char *format, const char *arg1) {
  if (!builder || !format || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  int result = snprintf(builder->buffer + builder->current_pos, 
                        builder->buffer_size - builder->current_pos, format, arg1);
  
  if (result < 0 || (size_t)result >= builder->buffer_size - builder->current_pos) {
    builder->error = 1;
    return ONVIF_ERROR;
  }
  
  builder->current_pos += result;
  return ONVIF_SUCCESS;
}

static int xml_builder_append_formatted_2(xml_builder_t *builder, const char *format, const char *arg1, const char *arg2) {
  if (!builder || !format || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  int result = snprintf(builder->buffer + builder->current_pos, 
                        builder->buffer_size - builder->current_pos, format, arg1, arg2);
  
  if (result < 0 || (size_t)result >= builder->buffer_size - builder->current_pos) {
    builder->error = 1;
    return ONVIF_ERROR;
  }
  
  builder->current_pos += result;
  return ONVIF_SUCCESS;
}

static int xml_builder_append_formatted_4(xml_builder_t *builder, const char *format, const char *arg1, const char *arg2, const char *arg3, const char *arg4) {
  if (!builder || !format || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  int result = snprintf(builder->buffer + builder->current_pos, 
                        builder->buffer_size - builder->current_pos, format, arg1, arg2, arg3, arg4);
  
  if (result < 0 || (size_t)result >= builder->buffer_size - builder->current_pos) {
    builder->error = 1;
    return ONVIF_ERROR;
  }
  
  builder->current_pos += result;
  return ONVIF_SUCCESS;
}

static int xml_builder_append_indent(xml_builder_t *builder) {
  if (!builder || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  for (int i = 0; i < builder->indent_level * XML_BUILDER_INDENT_SIZE; i++) {
    if (xml_builder_append_string(builder, " ") != ONVIF_SUCCESS) {
      return ONVIF_ERROR;
    }
  }
  return ONVIF_SUCCESS;
}

int xml_builder_init(xml_builder_t *builder, char *buffer, size_t buffer_size) {
  if (!builder || !buffer || buffer_size == 0) {
    return ONVIF_ERROR_INVALID;
  }
  
  memset(builder, 0, sizeof(xml_builder_t));
  builder->buffer = buffer;
  builder->buffer_size = buffer_size;
  builder->current_pos = 0;
  builder->indent_level = 0;
  builder->error = 0;
  
  // Ensure buffer is null-terminated
  builder->buffer[0] = '\0';
  
  return ONVIF_SUCCESS;
}

void xml_builder_cleanup(xml_builder_t *builder) {
  if (builder) {
    memset(builder, 0, sizeof(xml_builder_t));
  }
}

int xml_builder_has_error(const xml_builder_t *builder) {
  return builder ? builder->error : 1;
}

size_t xml_builder_get_position(const xml_builder_t *builder) {
  return builder ? builder->current_pos : 0;
}

size_t xml_builder_get_remaining(const xml_builder_t *builder) {
  return builder ? (builder->buffer_size - builder->current_pos) : 0;
}

int xml_builder_start_document(xml_builder_t *builder, const char *encoding) {
  if (!builder || !encoding) {
    return ONVIF_ERROR_INVALID;
  }
  
  if (xml_builder_append_formatted_1(builder, "<?xml version=\"1.0\" encoding=\"%s\"?>\n", 
                                  encoding) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  return ONVIF_SUCCESS;
}

int xml_builder_start_element(xml_builder_t *builder, const char *name, ...) {
  if (!builder || !name || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Add indentation
  if (xml_builder_indent(builder) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Start element
  if (xml_builder_append_formatted_1(builder, "<%s", name) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Add attributes
  va_list args;
  va_start(args, name);
  
  const char *attr_name = va_arg(args, const char *);
  while (attr_name != NULL) {
    const char *attr_value = va_arg(args, const char *);
    if (attr_value) {
      if (xml_builder_append_formatted_2(builder, " %s=\"%s\"", attr_name, attr_value) != ONVIF_SUCCESS) {
        va_end(args);
        return ONVIF_ERROR;
      }
    }
    attr_name = va_arg(args, const char *);
  }
  
  va_end(args);
  
  // Close opening tag
  if (xml_builder_append_string(builder, ">") != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Add newline
  if (xml_builder_newline(builder) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Increase indent for children
  xml_builder_increase_indent(builder);
  
  return ONVIF_SUCCESS;
}

int xml_builder_start_element_with_namespace(xml_builder_t *builder, const char *prefix,
                                           const char *name, const char *namespace_uri) {
  if (!builder || !prefix || !name || !namespace_uri) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Add indentation
  if (xml_builder_indent(builder) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Start element with namespace
  if (xml_builder_append_formatted_4(builder, "<%s:%s xmlns:%s=\"%s\">", 
                                  prefix, name, prefix, namespace_uri) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Add newline
  if (xml_builder_newline(builder) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Increase indent for children
  xml_builder_increase_indent(builder);
  
  return ONVIF_SUCCESS;
}

int xml_builder_end_element(xml_builder_t *builder, const char *name) {
  if (!builder || !name || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Decrease indent first
  xml_builder_decrease_indent(builder);
  
  // Add indentation
  if (xml_builder_indent(builder) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // End element
  if (xml_builder_append_formatted_1(builder, "</%s>", name) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Add newline
  if (xml_builder_newline(builder) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  return ONVIF_SUCCESS;
}

int xml_builder_self_closing_element(xml_builder_t *builder, const char *name, ...) {
  if (!builder || !name || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Add indentation
  if (xml_builder_indent(builder) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Start element
  if (xml_builder_append_formatted_1(builder, "<%s", name) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Add attributes
  va_list args;
  va_start(args, name);
  
  const char *attr_name = va_arg(args, const char *);
  while (attr_name != NULL) {
    const char *attr_value = va_arg(args, const char *);
    if (attr_value) {
      if (xml_builder_append_formatted_2(builder, " %s=\"%s\"", attr_name, attr_value) != ONVIF_SUCCESS) {
        va_end(args);
        return ONVIF_ERROR;
      }
    }
    attr_name = va_arg(args, const char *);
  }
  
  va_end(args);
  
  // Close as self-closing element
  if (xml_builder_append_string(builder, " />") != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Add newline
  if (xml_builder_newline(builder) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  return ONVIF_SUCCESS;
}

int xml_builder_element_with_text(xml_builder_t *builder, const char *name, const char *content, ...) {
  if (!builder || !name || !content || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Add indentation
  if (xml_builder_indent(builder) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Start element
  if (xml_builder_append_formatted_1(builder, "<%s", name) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Add attributes
  va_list args;
  va_start(args, content);
  
  const char *attr_name = va_arg(args, const char *);
  while (attr_name != NULL) {
    const char *attr_value = va_arg(args, const char *);
    if (attr_value) {
      if (xml_builder_append_formatted_2(builder, " %s=\"%s\"", attr_name, attr_value) != ONVIF_ERROR) {
        va_end(args);
        return ONVIF_ERROR;
      }
    }
    attr_name = va_arg(args, const char *);
  }
  
  va_end(args);
  
  // Add content and close
  if (xml_builder_append_formatted_2(builder, ">%s</%s>", content, name) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Add newline
  if (xml_builder_newline(builder) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  return ONVIF_SUCCESS;
}

int xml_builder_element_with_formatted_text(xml_builder_t *builder, const char *name,
                                          const char *format, ...) {
  if (!builder || !name || !format || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Add indentation
  if (xml_builder_indent(builder) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Start element
  if (xml_builder_append_formatted_1(builder, "<%s>", name) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Add formatted content
  va_list args;
  va_start(args, format);
  int result = xml_builder_append_formatted(builder, format, args);
  va_end(args);
  
  if (result != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Close element
  if (xml_builder_append_formatted_1(builder, "</%s>", name) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Add newline
  if (xml_builder_newline(builder) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  return ONVIF_SUCCESS;
}

int xml_builder_raw_content(xml_builder_t *builder, const char *content) {
  if (!builder || !content || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  return xml_builder_append_string(builder, content);
}

int xml_builder_formatted_content(xml_builder_t *builder, const char *format, ...) {
  if (!builder || !format || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  va_list args;
  va_start(args, format);
  int result = xml_builder_append_formatted(builder, format, args);
  va_end(args);
  
  return result;
}

int xml_builder_indent(xml_builder_t *builder) {
  return xml_builder_append_indent(builder);
}

int xml_builder_newline(xml_builder_t *builder) {
  return xml_builder_append_string(builder, "\n");
}

void xml_builder_increase_indent(xml_builder_t *builder) {
  if (builder) {
    builder->indent_level++;
  }
}

void xml_builder_decrease_indent(xml_builder_t *builder) {
  if (builder && builder->indent_level > 0) {
    builder->indent_level--;
  }
}

int xml_builder_escape_string(const char *input, char *output, size_t output_size) {
  if (!input || !output || output_size == 0) {
    return ONVIF_ERROR_INVALID;
  }
  
  size_t input_len = strlen(input);
  size_t output_pos = 0;
  
  for (size_t i = 0; i < input_len && output_pos < output_size - 1; i++) {
    switch (input[i]) {
      case '<':
        if (output_pos + 4 < output_size) {
          strcpy(output + output_pos, "&lt;");
          output_pos += 4;
        } else {
          return ONVIF_ERROR;
        }
        break;
      case '>':
        if (output_pos + 4 < output_size) {
          strcpy(output + output_pos, "&gt;");
          output_pos += 4;
        } else {
          return ONVIF_ERROR;
        }
        break;
      case '&':
        if (output_pos + 5 < output_size) {
          strcpy(output + output_pos, "&amp;");
          output_pos += 5;
        } else {
          return ONVIF_ERROR;
        }
        break;
      case '"':
        if (output_pos + 6 < output_size) {
          strcpy(output + output_pos, "&quot;");
          output_pos += 6;
        } else {
          return ONVIF_ERROR;
        }
        break;
      case '\'':
        if (output_pos + 6 < output_size) {
          strcpy(output + output_pos, "&apos;");
          output_pos += 6;
        } else {
          return ONVIF_ERROR;
        }
        break;
      default:
        output[output_pos++] = input[i];
        break;
    }
  }
  
  output[output_pos] = '\0';
  return ONVIF_SUCCESS;
}

const char *xml_builder_get_string(const xml_builder_t *builder) {
  return builder ? builder->buffer : NULL;
}

size_t xml_builder_get_length(const xml_builder_t *builder) {
  return builder ? builder->current_pos : 0;
}

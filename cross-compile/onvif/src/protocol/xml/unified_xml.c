/**
 * @file unified_xml.c
 * @brief Unified XML building, parsing, and validation utilities implementation
 * 
 * This module consolidates all XML functionality to eliminate duplication and provide
 * a single, consistent API for XML operations across all ONVIF services.
 * 
 * @author ONVIF Project
 * @date 2024
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdarg.h>
#include <time.h>
#include <ctype.h>
#include <strings.h>

#include "unified_xml.h"
#include "services/ptz/onvif_ptz.h"
#include "services/media/onvif_media.h"
#include "utils/error/error_handling.h"
#include "utils/memory/memory_manager.h"
#include "utils/string/string_shims.h"

static const onvif_xml_builder_config_t default_builder_config = {
  .enable_indentation = 1,
  .enable_validation = 1,
  .max_depth = 32,
  .default_encoding = "UTF-8"
};

static const onvif_xml_parser_config_t default_parser_config = {
  .max_depth = 32,
  .max_attributes = 64,
  .enable_security_checks = 1,
  .strict_mode = 0
};

static const onvif_xml_validation_config_t default_validation_config = {
  .max_depth = 32,
  .max_attributes = 64,
  .max_length = 4096,
  .enable_security_checks = 1,
  .enable_basic_validation = 1,
  .enable_structure_validation = 1
};

/* ============================================================================
 * Common XML Helper Functions
 * ============================================================================ */

/**
 * @brief Extract string value from XML between start and end tags
 * @param xml XML content to parse
 * @param start_tag The opening tag (e.g., "<tt:ProfileToken>")
 * @param end_tag The closing tag (e.g., "</tt:ProfileToken>")
 * @param value Buffer to store the extracted value
 * @param value_size Size of the value buffer
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_extract_string_value(const char *xml, const char *start_tag, const char *end_tag,
                                   char *value, size_t value_size) {
  if (!xml || !start_tag || !end_tag || !value || value_size == 0) {
    return ONVIF_ERROR_INVALID;
  }
  
  const char *start = strstr(xml, start_tag);
  if (!start) {
    return ONVIF_ERROR_NOT_FOUND;
  }
  
  start += strlen(start_tag);
  
  const char *end = strstr(start, end_tag);
  if (!end) {
    return ONVIF_ERROR_NOT_FOUND;
  }
  
  size_t value_len = end - start;
  if (value_len >= value_size) {
    return ONVIF_ERROR;
  }
  
  memory_safe_strncpy(value, value_size, start, value_len);
  value[value_len] = '\0';
  
  return ONVIF_SUCCESS;
}

/* ============================================================================
 * XML Builder Implementation
 * ============================================================================ */

int onvif_xml_builder_init(onvif_xml_builder_t *builder, char *buffer, size_t buffer_size, 
                          const onvif_xml_builder_config_t *config) {
  if (!builder || !buffer || buffer_size == 0) {
    return ONVIF_ERROR_INVALID;
  }
  
  builder->buffer = buffer;
  builder->buffer_size = buffer_size;
  builder->current_pos = 0;
  builder->indent_level = 0;
  builder->error = 0;
  
  // Apply configuration
  if (config) {
    // Configuration can be applied here if needed
  } else {
    // Use default configuration
  }
  
  // Null-terminate the buffer
  buffer[0] = '\0';
  
  return ONVIF_SUCCESS;
}

void onvif_xml_builder_cleanup(onvif_xml_builder_t *builder) {
  if (builder) {
    builder->buffer = NULL;
    builder->buffer_size = 0;
    builder->current_pos = 0;
    builder->indent_level = 0;
    builder->error = 0;
  }
}

int onvif_xml_builder_has_error(const onvif_xml_builder_t *builder) {
  return builder ? builder->error : 1;
}

size_t onvif_xml_builder_get_position(const onvif_xml_builder_t *builder) {
  return builder ? builder->current_pos : 0;
}

size_t onvif_xml_builder_get_remaining(const onvif_xml_builder_t *builder) {
  if (!builder || builder->error) {
    return 0;
  }
  return builder->buffer_size - builder->current_pos;
}

int onvif_xml_builder_start_document(onvif_xml_builder_t *builder, const char *encoding) {
  if (!builder || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  const char *enc = encoding ? encoding : "UTF-8";
  int result = memory_safe_snprintf(builder->buffer + builder->current_pos, 
                                   builder->buffer_size - builder->current_pos,
                                   "<?xml version=\"1.0\" encoding=\"%s\"?>\n", enc);
  
  if (result < 0 || (size_t)result >= (builder->buffer_size - builder->current_pos)) {
    builder->error = 1;
    return ONVIF_ERROR;
  }
  
  builder->current_pos += result;
  return ONVIF_SUCCESS;
}

int onvif_xml_builder_indent(onvif_xml_builder_t *builder) {
  if (!builder || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  for (int i = 0; i < builder->indent_level; i++) {
    int result = memory_safe_snprintf(builder->buffer + builder->current_pos,
                                     builder->buffer_size - builder->current_pos,
                                     "  ");
    
    if (result < 0 || (size_t)result >= (builder->buffer_size - builder->current_pos)) {
      builder->error = 1;
      return ONVIF_ERROR;
    }
    
    builder->current_pos += result;
  }
  
  return ONVIF_SUCCESS;
}

int onvif_xml_builder_start_element(onvif_xml_builder_t *builder, const char *name, ...) {
  if (!builder || !name || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  onvif_xml_builder_indent(builder);
  
  int result = memory_safe_snprintf(builder->buffer + builder->current_pos,
                                   builder->buffer_size - builder->current_pos,
                                   "<%s", name);
  
  if (result < 0 || (size_t)result >= (builder->buffer_size - builder->current_pos)) {
    builder->error = 1;
    return ONVIF_ERROR;
  }
  
  builder->current_pos += result;
  
  va_list args;
  va_start(args, name);
  
  const char *attr_name = va_arg(args, const char *);
  while (attr_name != NULL) {
    const char *attr_value = va_arg(args, const char *);
    if (attr_value != NULL) {
      result = memory_safe_snprintf(builder->buffer + builder->current_pos,
                                   builder->buffer_size - builder->current_pos,
                                   " %s=\"%s\"", attr_name, attr_value);
      
      if (result < 0 || (size_t)result >= (builder->buffer_size - builder->current_pos)) {
        builder->error = 1;
        va_end(args);
        return ONVIF_ERROR;
      }
      
      builder->current_pos += result;
    }
    attr_name = va_arg(args, const char *);
  }
  
  va_end(args);
  
  // Close element
  result = memory_safe_snprintf(builder->buffer + builder->current_pos,
                               builder->buffer_size - builder->current_pos,
                               ">");
  
  if (result < 0 || (size_t)result >= (builder->buffer_size - builder->current_pos)) {
    builder->error = 1;
    return ONVIF_ERROR;
  }
  
  builder->current_pos += result;
  builder->indent_level++;
  
  return ONVIF_SUCCESS;
}

int onvif_xml_builder_start_element_with_namespace(onvif_xml_builder_t *builder, const char *prefix,
                                                   const char *name, const char *namespace_uri) {
  if (!builder || !prefix || !name || !namespace_uri || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  onvif_xml_builder_indent(builder);
  
  // Element with namespace
  int result = memory_safe_snprintf(builder->buffer + builder->current_pos,
                                   builder->buffer_size - builder->current_pos,
                                   "<%s:%s xmlns:%s=\"%s\">", prefix, name, prefix, namespace_uri);
  
  if (result < 0 || (size_t)result >= (builder->buffer_size - builder->current_pos)) {
    builder->error = 1;
    return ONVIF_ERROR;
  }
  
  builder->current_pos += result;
  builder->indent_level++;
  
  return ONVIF_SUCCESS;
}

int onvif_xml_builder_end_element(onvif_xml_builder_t *builder, const char *name) {
  if (!builder || !name || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  builder->indent_level--;
  
  onvif_xml_builder_indent(builder);
  
  // End element
  int result = memory_safe_snprintf(builder->buffer + builder->current_pos,
                                   builder->buffer_size - builder->current_pos,
                                   "</%s>", name);
  
  if (result < 0 || (size_t)result >= (builder->buffer_size - builder->current_pos)) {
    builder->error = 1;
    return ONVIF_ERROR;
  }
  
  builder->current_pos += result;
  
  return ONVIF_SUCCESS;
}

int onvif_xml_builder_self_closing_element(onvif_xml_builder_t *builder, const char *name, ...) {
  if (!builder || !name || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  onvif_xml_builder_indent(builder);
  
  int result = memory_safe_snprintf(builder->buffer + builder->current_pos,
                                   builder->buffer_size - builder->current_pos,
                                   "<%s", name);
  
  if (result < 0 || (size_t)result >= (builder->buffer_size - builder->current_pos)) {
    builder->error = 1;
    return ONVIF_ERROR;
  }
  
  builder->current_pos += result;
  
  va_list args;
  va_start(args, name);
  
  const char *attr_name = va_arg(args, const char *);
  while (attr_name != NULL) {
    const char *attr_value = va_arg(args, const char *);
    if (attr_value != NULL) {
      result = memory_safe_snprintf(builder->buffer + builder->current_pos,
                                   builder->buffer_size - builder->current_pos,
                                   " %s=\"%s\"", attr_name, attr_value);
      
      if (result < 0 || (size_t)result >= (builder->buffer_size - builder->current_pos)) {
        builder->error = 1;
        va_end(args);
        return ONVIF_ERROR;
      }
      
      builder->current_pos += result;
    }
    attr_name = va_arg(args, const char *);
  }
  
  va_end(args);
  
  // Close element
  result = memory_safe_snprintf(builder->buffer + builder->current_pos,
                               builder->buffer_size - builder->current_pos,
                               "/>");
  
  if (result < 0 || (size_t)result >= (builder->buffer_size - builder->current_pos)) {
    builder->error = 1;
    return ONVIF_ERROR;
  }
  
  builder->current_pos += result;
  
  return ONVIF_SUCCESS;
}

int onvif_xml_builder_element_with_text(onvif_xml_builder_t *builder, const char *name, const char *content, ...) {
  if (!builder || !name || !content || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  onvif_xml_builder_indent(builder);
  
  int result = memory_safe_snprintf(builder->buffer + builder->current_pos,
                                   builder->buffer_size - builder->current_pos,
                                   "<%s", name);
  
  if (result < 0 || (size_t)result >= (builder->buffer_size - builder->current_pos)) {
    builder->error = 1;
    return ONVIF_ERROR;
  }
  
  builder->current_pos += result;
  
  va_list args;
  va_start(args, content);
  
  const char *attr_name = va_arg(args, const char *);
  while (attr_name != NULL) {
    const char *attr_value = va_arg(args, const char *);
    if (attr_value != NULL) {
      result = memory_safe_snprintf(builder->buffer + builder->current_pos,
                                   builder->buffer_size - builder->current_pos,
                                   " %s=\"%s\"", attr_name, attr_value);
      
      if (result < 0 || (size_t)result >= (builder->buffer_size - builder->current_pos)) {
        builder->error = 1;
        va_end(args);
        return ONVIF_ERROR;
      }
      
      builder->current_pos += result;
    }
    attr_name = va_arg(args, const char *);
  }
  
  va_end(args);
  
  // Close element start and add content
  result = memory_safe_snprintf(builder->buffer + builder->current_pos,
                               builder->buffer_size - builder->current_pos,
                               ">%s</%s>", content, name);
  
  if (result < 0 || (size_t)result >= (builder->buffer_size - builder->current_pos)) {
    builder->error = 1;
    return ONVIF_ERROR;
  }
  
  builder->current_pos += result;
  
  return ONVIF_SUCCESS;
}

int onvif_xml_builder_element_with_formatted_text(onvif_xml_builder_t *builder, const char *name,
                                                  const char *format, ...) {
  if (!builder || !name || !format || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  onvif_xml_builder_indent(builder);
  
  int result = memory_safe_snprintf(builder->buffer + builder->current_pos,
                                   builder->buffer_size - builder->current_pos,
                                   "<%s>", name);
  
  if (result < 0 || (size_t)result >= (builder->buffer_size - builder->current_pos)) {
    builder->error = 1;
    return ONVIF_ERROR;
  }
  
  builder->current_pos += result;
  
  // Add formatted content
  va_list args;
  va_start(args, format);
  
  result = memory_safe_snprintf(builder->buffer + builder->current_pos,
                               builder->buffer_size - builder->current_pos,
                               format, args);
  
  va_end(args);
  
  if (result < 0 || (size_t)result >= (builder->buffer_size - builder->current_pos)) {
    builder->error = 1;
    return ONVIF_ERROR;
  }
  
  builder->current_pos += result;
  
  // End element
  result = memory_safe_snprintf(builder->buffer + builder->current_pos,
                               builder->buffer_size - builder->current_pos,
                               "</%s>", name);
  
  if (result < 0 || (size_t)result >= (builder->buffer_size - builder->current_pos)) {
    builder->error = 1;
    return ONVIF_ERROR;
  }
  
  builder->current_pos += result;
  
  return ONVIF_SUCCESS;
}

int onvif_xml_builder_raw_content(onvif_xml_builder_t *builder, const char *content) {
  if (!builder || !content || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  int result = memory_safe_snprintf(builder->buffer + builder->current_pos,
                                   builder->buffer_size - builder->current_pos,
                                   "%s", content);
  
  if (result < 0 || (size_t)result >= (builder->buffer_size - builder->current_pos)) {
    builder->error = 1;
    return ONVIF_ERROR;
  }
  
  builder->current_pos += result;
  return ONVIF_SUCCESS;
}

int onvif_xml_builder_formatted_content(onvif_xml_builder_t *builder, const char *format, ...) {
  if (!builder || !format || builder->error) {
    return ONVIF_ERROR_INVALID;
  }
  
  va_list args;
  va_start(args, format);
  
  int result = memory_safe_snprintf(builder->buffer + builder->current_pos,
                                   builder->buffer_size - builder->current_pos,
                                   format, args);
  
  va_end(args);
  
  if (result < 0 || (size_t)result >= (builder->buffer_size - builder->current_pos)) {
    builder->error = 1;
    return ONVIF_ERROR;
  }
  
  builder->current_pos += result;
  return ONVIF_SUCCESS;
}

const char *onvif_xml_builder_get_string(const onvif_xml_builder_t *builder) {
  return builder ? builder->buffer : NULL;
}

size_t onvif_xml_builder_get_length(const onvif_xml_builder_t *builder) {
  return builder ? builder->current_pos : 0;
}

/* ============================================================================
 * XML Parser Implementation
 * ============================================================================ */

int onvif_xml_parser_init(onvif_xml_parser_t *parser, const char *xml, size_t length,
                         const onvif_xml_parser_config_t *config) {
  if (!parser || !xml || length == 0) {
    return ONVIF_ERROR_INVALID;
  }
  
  parser->xml = xml;
  parser->length = length;
  parser->position = 0;
  parser->error = 0;
  parser->depth = 0;
  
  // Apply configuration
  if (config) {
    // Configuration can be applied here if needed
  } else {
    // Use default configuration
  }
  
  return ONVIF_SUCCESS;
}

void onvif_xml_parser_cleanup(onvif_xml_parser_t *parser) {
  if (parser) {
    parser->xml = NULL;
    parser->length = 0;
    parser->position = 0;
    parser->error = 0;
    parser->depth = 0;
  }
}

int onvif_xml_parser_has_error(const onvif_xml_parser_t *parser) {
  return parser ? parser->error : 1;
}

int onvif_xml_parser_extract_value(onvif_xml_parser_t *parser, const char *start_tag, const char *end_tag,
                                   char *value, size_t value_size) {
  if (!parser || !start_tag || !end_tag || !value || value_size == 0) {
    return ONVIF_ERROR_INVALID;
  }
  
  const char *start = strstr(parser->xml + parser->position, start_tag);
  if (!start) {
    return ONVIF_ERROR_NOT_FOUND;
  }
  
  start += strlen(start_tag);
  
  const char *end = strstr(start, end_tag);
  if (!end) {
    return ONVIF_ERROR_NOT_FOUND;
  }
  
  size_t value_len = end - start;
  if (value_len >= value_size) {
    return ONVIF_ERROR;
  }
  
  memory_safe_strncpy(value, value_size, start, value_len);
  value[value_len] = '\0';
  
  return ONVIF_SUCCESS;
}

int onvif_xml_parser_extract_int(onvif_xml_parser_t *parser, const char *start_tag, const char *end_tag, int *value) {
  if (!parser || !start_tag || !end_tag || !value) {
    return ONVIF_ERROR_INVALID;
  }
  
  char buffer[32];
  int result = onvif_xml_parser_extract_value(parser, start_tag, end_tag, buffer, sizeof(buffer));
  if (result != ONVIF_SUCCESS) {
    return result;
  }
  
  *value = atoi(buffer);
  return ONVIF_SUCCESS;
}

int onvif_xml_parser_extract_float(onvif_xml_parser_t *parser, const char *start_tag, const char *end_tag, float *value) {
  if (!parser || !start_tag || !end_tag || !value) {
    return ONVIF_ERROR_INVALID;
  }
  
  char buffer[32];
  int result = onvif_xml_parser_extract_value(parser, start_tag, end_tag, buffer, sizeof(buffer));
  if (result != ONVIF_SUCCESS) {
    return result;
  }
  
  *value = atof(buffer);
  return ONVIF_SUCCESS;
}

int onvif_xml_parser_extract_bool(onvif_xml_parser_t *parser, const char *start_tag, const char *end_tag, int *value) {
  if (!parser || !start_tag || !end_tag || !value) {
    return ONVIF_ERROR_INVALID;
  }
  
  char buffer[8];
  int result = onvif_xml_parser_extract_value(parser, start_tag, end_tag, buffer, sizeof(buffer));
  if (result != ONVIF_SUCCESS) {
    return result;
  }
  
  *value = (strcasecmp(buffer, "true") == 0) ? 1 : 0;
  return ONVIF_SUCCESS;
}

/* ============================================================================
 * XML Validation Implementation
 * ============================================================================ */

int onvif_xml_validate(const char *xml, size_t length, const onvif_xml_validation_config_t *config,
                      onvif_xml_validation_result_t *result) {
  if (!xml || length == 0) {
    return ONVIF_ERROR_INVALID;
  }
  
  const onvif_xml_validation_config_t *cfg = config ? config : &default_validation_config;
  
  // Basic length check
  if (length > cfg->max_length) {
    if (result) {
      result->is_valid = 0;
      result->error_code = ONVIF_ERROR_INVALID;
      result->error_message = "XML content too long";
    }
    return ONVIF_ERROR_INVALID;
  }
  
  // Basic structure validation
  if (cfg->enable_basic_validation) {
    if (onvif_xml_validate_basic(xml, length) != ONVIF_SUCCESS) {
      if (result) {
        result->is_valid = 0;
        result->error_code = ONVIF_ERROR_INVALID;
        result->error_message = "Invalid XML structure";
      }
      return ONVIF_ERROR_INVALID;
    }
  }
  
  // Security validation
  if (cfg->enable_security_checks) {
    if (onvif_xml_validate_security(xml, length) != ONVIF_SUCCESS) {
      if (result) {
        result->is_valid = 0;
        result->error_code = ONVIF_ERROR_INVALID;
        result->error_message = "Security validation failed";
        result->security_issues = 1;
      }
      return ONVIF_ERROR_INVALID;
    }
  }
  
  if (result) {
    result->is_valid = 1;
    result->error_code = ONVIF_SUCCESS;
    result->error_message = NULL;
    result->security_issues = 0;
    result->structure_issues = 0;
  }
  
  return ONVIF_SUCCESS;
}

int onvif_xml_validate_basic(const char *xml, size_t length) {
  if (!xml || length == 0) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Check for XML declaration or root element
  if (length < 5 || (strncmp(xml, "<?xml", 5) != 0 && strncmp(xml, "<", 1) != 0)) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Basic tag balancing check
  int open_tags = 0;
  const char *ptr = xml;
  const char *end = xml + length;
  
  while (ptr < end) {
    if (*ptr == '<') {
      if (ptr + 1 < end && *(ptr + 1) == '/') {
        // Closing tag
        open_tags--;
        if (open_tags < 0) {
          return ONVIF_ERROR_INVALID; // Unbalanced tags
        }
      } else if (ptr + 1 < end && *(ptr + 1) != '!') {
        // Opening tag (skip comments and CDATA)
        open_tags++;
      }
    }
    ptr++;
  }
  
  // All tags should be balanced
  if (open_tags != 0) {
    return ONVIF_ERROR_INVALID;
  }
  
  return ONVIF_SUCCESS;
}

int onvif_xml_validate_security(const char *xml, size_t length) {
  if (!xml || length == 0) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Check for dangerous patterns
  const char *dangerous_patterns[] = {
    "<script",
    "javascript:",
    "vbscript:",
    "onload=",
    "onerror=",
    "onclick=",
    "eval(",
    "exec(",
    "system(",
    NULL
  };
  
  for (int i = 0; dangerous_patterns[i] != NULL; i++) {
    if (strcasestr(xml, dangerous_patterns[i]) != NULL) {
      return ONVIF_ERROR_INVALID;
    }
  }
  
  // Check for XML bomb patterns
  if (strstr(xml, "!DOCTYPE") != NULL && strstr(xml, "ENTITY") != NULL) {
    return ONVIF_ERROR_INVALID;
  }
  
  return ONVIF_SUCCESS;
}

int onvif_xml_is_xml_content(const char *str) {
  if (!str) {
    return 0;
  }
  
  // Skip whitespace
  while (isspace(*str)) {
    str++;
  }
  
  // Check for XML declaration or root element
  return (strncmp(str, "<?xml", 5) == 0) || (strncmp(str, "<", 1) == 0);
}

int onvif_xml_escape_string(const char *input, char *output, size_t output_size) {
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

void onvif_xml_value_free(char *value) {
  if (value) {
    ONVIF_FREE(value);
  }
}

/* ============================================================================
 * Service-Specific Parser Functions
 * ============================================================================ */

int onvif_xml_parse_ptz_position(onvif_xml_parser_t *parser, struct ptz_vector *position) {
  if (!parser || !position) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Parse X coordinate
  if (onvif_xml_parser_extract_float(parser, "<tt:PanTilt><tt:x>", "</tt:x></tt:PanTilt>", &position->pan_tilt.x) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Parse Y coordinate
  if (onvif_xml_parser_extract_float(parser, "<tt:PanTilt><tt:y>", "</tt:y></tt:PanTilt>", &position->pan_tilt.y) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Parse Z coordinate
  if (onvif_xml_parser_extract_float(parser, "<tt:Zoom><tt:x>", "</tt:x></tt:Zoom>", &position->zoom) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  return ONVIF_SUCCESS;
}

int onvif_xml_parse_ptz_speed(onvif_xml_parser_t *parser, struct ptz_speed *speed) {
  if (!parser || !speed) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Parse pan-tilt speed
  if (onvif_xml_parser_extract_float(parser, "<tt:PanTilt><tt:x>", "</tt:x></tt:PanTilt>", &speed->pan_tilt.x) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  if (onvif_xml_parser_extract_float(parser, "<tt:PanTilt><tt:y>", "</tt:y></tt:PanTilt>", &speed->pan_tilt.y) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Parse zoom speed
  if (onvif_xml_parser_extract_float(parser, "<tt:Zoom><tt:x>", "</tt:x></tt:Zoom>", &speed->zoom) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  return ONVIF_SUCCESS;
}

int onvif_xml_parse_imaging_settings(onvif_xml_parser_t *parser, struct imaging_settings *settings) {
  if (!parser || !settings) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Parse brightness
  if (onvif_xml_parser_extract_int(parser, "<tt:Brightness>", "</tt:Brightness>", &settings->brightness) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Parse contrast
  if (onvif_xml_parser_extract_int(parser, "<tt:Contrast>", "</tt:Contrast>", &settings->contrast) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  // Parse saturation
  if (onvif_xml_parser_extract_int(parser, "<tt:ColorSaturation>", "</tt:ColorSaturation>", &settings->saturation) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  
  return ONVIF_SUCCESS;
}

int onvif_xml_parse_video_source_configuration(onvif_xml_parser_t *parser, struct video_source_configuration *config) {
  if (!parser || !config) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Parse token
  char token[64];
  if (onvif_xml_parser_extract_value(parser, "<tt:ConfigurationToken>", "</tt:ConfigurationToken>", 
                                    token, sizeof(token)) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  strncpy(config->token, token, sizeof(config->token) - 1);
  config->token[sizeof(config->token) - 1] = '\0';
  
  // Parse name
  char name[64];
  if (onvif_xml_parser_extract_value(parser, "<tt:Name>", "</tt:Name>", 
                                    name, sizeof(name)) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  strncpy(config->name, name, sizeof(config->name) - 1);
  config->name[sizeof(config->name) - 1] = '\0';
  
  return ONVIF_SUCCESS;
}

int onvif_xml_parse_video_encoder_configuration(onvif_xml_parser_t *parser, struct video_encoder_configuration *config) {
  if (!parser || !config) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Parse token
  char token[64];
  if (onvif_xml_parser_extract_value(parser, "<tt:ConfigurationToken>", "</tt:ConfigurationToken>", 
                                    token, sizeof(token)) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  strncpy(config->token, token, sizeof(config->token) - 1);
  config->token[sizeof(config->token) - 1] = '\0';
  
  // Parse name
  char name[64];
  if (onvif_xml_parser_extract_value(parser, "<tt:Name>", "</tt:Name>", 
                                    name, sizeof(name)) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  strncpy(config->name, name, sizeof(config->name) - 1);
  config->name[sizeof(config->name) - 1] = '\0';
  
  return ONVIF_SUCCESS;
}

int onvif_xml_parse_metadata_configuration(onvif_xml_parser_t *parser, struct metadata_configuration *config) {
  if (!parser || !config) {
    return ONVIF_ERROR_INVALID;
  }
  
  // Parse token
  char token[64];
  if (onvif_xml_parser_extract_value(parser, "<tt:ConfigurationToken>", "</tt:ConfigurationToken>", 
                                    token, sizeof(token)) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  strncpy(config->token, token, sizeof(config->token) - 1);
  config->token[sizeof(config->token) - 1] = '\0';
  
  // Parse name
  char name[64];
  if (onvif_xml_parser_extract_value(parser, "<tt:Name>", "</tt:Name>", 
                                    name, sizeof(name)) != ONVIF_SUCCESS) {
    return ONVIF_ERROR;
  }
  strncpy(config->name, name, sizeof(config->name) - 1);
  config->name[sizeof(config->name) - 1] = '\0';
  
  return ONVIF_SUCCESS;
}

/* ============================================================================
 * Simple parsing functions for service compatibility
 * ============================================================================ */

int onvif_xml_parse_profile_token(const char *xml, char *token, size_t token_size) {
  return onvif_xml_extract_string_value(xml, "<tt:ProfileToken>", "</tt:ProfileToken>", token, token_size);
}

int onvif_xml_parse_protocol(const char *xml, char *protocol, size_t protocol_size) {
  return onvif_xml_extract_string_value(xml, "<tt:Protocol>", "</tt:Protocol>", protocol, protocol_size);
}

int onvif_xml_parse_configuration_token(const char *xml, char *token, size_t token_size) {
  return onvif_xml_extract_string_value(xml, "<tt:ConfigurationToken>", "</tt:ConfigurationToken>", token, token_size);
}
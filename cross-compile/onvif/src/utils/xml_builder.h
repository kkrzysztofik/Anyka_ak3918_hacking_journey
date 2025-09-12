/**
 * @file xml_builder.h
 * @brief XML builder utility for eliminating duplicate XML construction patterns
 * 
 * This module provides a fluent XML builder API that eliminates the need for
 * manual string formatting and reduces code duplication in XML generation.
 */

#ifndef ONVIF_XML_BUILDER_H
#define ONVIF_XML_BUILDER_H

#include <stddef.h>
#include <stdarg.h>

/**
 * @brief XML builder state
 */
typedef struct {
  char *buffer;
  size_t buffer_size;
  size_t current_pos;
  int indent_level;
  int error;
} xml_builder_t;

/**
 * @brief Initialize XML builder
 * @param builder Builder to initialize
 * @param buffer Buffer to write XML to
 * @param buffer_size Size of the buffer
 * @return 0 on success, negative error code on failure
 */
int xml_builder_init(xml_builder_t *builder, char *buffer, size_t buffer_size);

/**
 * @brief Clean up XML builder
 * @param builder Builder to clean up
 */
void xml_builder_cleanup(xml_builder_t *builder);

/**
 * @brief Check if builder has encountered an error
 * @param builder Builder to check
 * @return 1 if error, 0 if OK
 */
int xml_builder_has_error(const xml_builder_t *builder);

/**
 * @brief Get current position in buffer
 * @param builder Builder to check
 * @return Current position
 */
size_t xml_builder_get_position(const xml_builder_t *builder);

/**
 * @brief Get remaining buffer space
 * @param builder Builder to check
 * @return Remaining space
 */
size_t xml_builder_get_remaining(const xml_builder_t *builder);

/**
 * @brief Start XML document with declaration
 * @param builder Builder to use
 * @param encoding Character encoding (e.g., "UTF-8")
 * @return 0 on success, negative error code on failure
 */
int xml_builder_start_document(xml_builder_t *builder, const char *encoding);

/**
 * @brief Start XML element with attributes
 * @param builder Builder to use
 * @param name Element name
 * @param ... Attribute pairs (name, value) terminated with NULL
 * @return 0 on success, negative error code on failure
 */
int xml_builder_start_element(xml_builder_t *builder, const char *name, ...);

/**
 * @brief Start XML element with namespace
 * @param builder Builder to use
 * @param prefix Namespace prefix
 * @param name Element name
 * @param namespace_uri Namespace URI
 * @return 0 on success, negative error code on failure
 */
int xml_builder_start_element_with_namespace(xml_builder_t *builder, const char *prefix,
                                           const char *name, const char *namespace_uri);

/**
 * @brief End XML element
 * @param builder Builder to use
 * @param name Element name (for validation)
 * @return 0 on success, negative error code on failure
 */
int xml_builder_end_element(xml_builder_t *builder, const char *name);

/**
 * @brief Add self-closing XML element
 * @param builder Builder to use
 * @param name Element name
 * @param ... Attribute pairs (name, value) terminated with NULL
 * @return 0 on success, negative error code on failure
 */
int xml_builder_self_closing_element(xml_builder_t *builder, const char *name, ...);

/**
 * @brief Add XML element with text content
 * @param builder Builder to use
 * @param name Element name
 * @param content Text content
 * @param ... Attribute pairs (name, value) terminated with NULL
 * @return 0 on success, negative error code on failure
 */
int xml_builder_element_with_text(xml_builder_t *builder, const char *name, const char *content, ...);

/**
 * @brief Add XML element with formatted text content
 * @param builder Builder to use
 * @param name Element name
 * @param format Printf-style format string
 * @param ... Format arguments
 * @return 0 on success, negative error code on failure
 */
int xml_builder_element_with_formatted_text(xml_builder_t *builder, const char *name,
                                          const char *format, ...);

/**
 * @brief Add raw XML content
 * @param builder Builder to use
 * @param content Raw XML content
 * @return 0 on success, negative error code on failure
 */
int xml_builder_raw_content(xml_builder_t *builder, const char *content);

/**
 * @brief Add formatted XML content
 * @param builder Builder to use
 * @param format Printf-style format string
 * @param ... Format arguments
 * @return 0 on success, negative error code on failure
 */
int xml_builder_formatted_content(xml_builder_t *builder, const char *format, ...);

/**
 * @brief Add indentation
 * @param builder Builder to use
 * @return 0 on success, negative error code on failure
 */
int xml_builder_indent(xml_builder_t *builder);

/**
 * @brief Add newline
 * @param builder Builder to use
 * @return 0 on success, negative error code on failure
 */
int xml_builder_newline(xml_builder_t *builder);

/**
 * @brief Increase indentation level
 * @param builder Builder to use
 */
void xml_builder_increase_indent(xml_builder_t *builder);

/**
 * @brief Decrease indentation level
 * @param builder Builder to use
 */
void xml_builder_decrease_indent(xml_builder_t *builder);

/**
 * @brief Escape XML special characters
 * @param input Input string
 * @param output Output buffer
 * @param output_size Size of output buffer
 * @return 0 on success, negative error code on failure
 */
int xml_builder_escape_string(const char *input, char *output, size_t output_size);

/**
 * @brief Add attribute to current element
 * @param builder Builder to use
 * @param name Attribute name
 * @param value Attribute value
 * @return 0 on success, negative error code on failure
 */
int xml_builder_add_attribute(xml_builder_t *builder, const char *name, const char *value);

/**
 * @brief Add formatted attribute to current element
 * @param builder Builder to use
 * @param name Attribute name
 * @param format Printf-style format string
 * @param ... Format arguments
 * @return 0 on success, negative error code on failure
 */
int xml_builder_add_formatted_attribute(xml_builder_t *builder, const char *name,
                                      const char *format, ...);

/**
 * @brief Get the final XML string
 * @param builder Builder to use
 * @return XML string (null-terminated)
 */
const char *xml_builder_get_string(const xml_builder_t *builder);

/**
 * @brief Get the length of the generated XML
 * @param builder Builder to use
 * @return Length of XML string
 */
size_t xml_builder_get_length(const xml_builder_t *builder);

#endif /* ONVIF_XML_BUILDER_H */

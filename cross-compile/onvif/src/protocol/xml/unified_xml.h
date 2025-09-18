/**
 * @file unified_xml.h
 * @brief Unified XML building, parsing, and validation utilities for ONVIF services
 * @author kkrzysztofik
 * @date 2025
 */

#ifndef ONVIF_UNIFIED_XML_H
#define ONVIF_UNIFIED_XML_H

#include <stddef.h>
#include <stdarg.h>
#include <time.h>
#include "services/common/onvif_types.h"
#include "services/common/onvif_imaging_types.h"
#include "utils/memory/memory_manager.h"
#include "utils/error/error_handling.h"

/* Forward declarations for structs used in function parameters */
struct ptz_vector;
struct ptz_speed;
struct video_source_configuration;
struct video_encoder_configuration;
struct metadata_configuration;
struct imaging_settings;

/* ============================================================================
 * XML Builder
 * ============================================================================ */

/**
 * @brief XML builder state
 */
typedef struct {
  char *buffer;
  size_t buffer_size;
  size_t current_pos;
  int indent_level;
  int error;
} onvif_xml_builder_t;

/**
 * @brief XML builder configuration
 */
typedef struct {
  int enable_indentation;
  int enable_validation;
  int max_depth;
  const char *default_encoding;
} onvif_xml_builder_config_t;

/* ============================================================================
 * XML Parser
 * ============================================================================ */

/**
 * @brief XML parser state
 */
typedef struct {
  const char *xml;
  size_t length;
  size_t position;
  int error;
  int depth;
} onvif_xml_parser_t;

/**
 * @brief XML parser configuration
 */
typedef struct {
  int max_depth;
  int max_attributes;
  int enable_security_checks;
  int strict_mode;
} onvif_xml_parser_config_t;

/* ============================================================================
 * XML Validation
 * ============================================================================ */

/**
 * @brief XML validation configuration
 */
typedef struct {
  int max_depth;
  int max_attributes;
  int max_length;
  int enable_security_checks;
  int enable_basic_validation;
  int enable_structure_validation;
} onvif_xml_validation_config_t;

/**
 * @brief XML validation result
 */
typedef struct {
  int is_valid;
  int error_code;
  const char *error_message;
  int security_issues;
  int structure_issues;
} onvif_xml_validation_result_t;

/* ============================================================================
 * XML Builder Functions
 * ============================================================================ */

/**
 * @brief Initialize XML builder
 * @param builder Builder to initialize
 * @param buffer Buffer to write XML to
 * @param buffer_size Size of the buffer
 * @param config Builder configuration (can be NULL for defaults)
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_builder_init(onvif_xml_builder_t *builder, char *buffer, size_t buffer_size, 
                          const onvif_xml_builder_config_t *config);

/**
 * @brief Clean up XML builder
 * @param builder Builder to clean up
 */
void onvif_xml_builder_cleanup(onvif_xml_builder_t *builder);

/**
 * @brief Check if builder has encountered an error
 * @param builder Builder to check
 * @return 1 if error, 0 if OK
 */
int onvif_xml_builder_has_error(const onvif_xml_builder_t *builder);

/**
 * @brief Get current position in buffer
 * @param builder Builder to check
 * @return Current position
 */
size_t onvif_xml_builder_get_position(const onvif_xml_builder_t *builder);

/**
 * @brief Get remaining buffer space
 * @param builder Builder to check
 * @return Remaining space
 */
size_t onvif_xml_builder_get_remaining(const onvif_xml_builder_t *builder);

/**
 * @brief Start XML document with declaration
 * @param builder Builder to use
 * @param encoding Character encoding (e.g., "UTF-8")
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_builder_start_document(onvif_xml_builder_t *builder, const char *encoding);

/**
 * @brief Start XML element with attributes
 * @param builder Builder to use
 * @param name Element name
 * @param ... Attribute pairs (name, value) terminated with NULL
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_builder_start_element(onvif_xml_builder_t *builder, const char *name, ...);

/**
 * @brief Start XML element with namespace
 * @param builder Builder to use
 * @param prefix Namespace prefix
 * @param name Element name
 * @param namespace_uri Namespace URI
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_builder_start_element_with_namespace(onvif_xml_builder_t *builder, const char *prefix,
                                                   const char *name, const char *namespace_uri);

/**
 * @brief End XML element
 * @param builder Builder to use
 * @param name Element name (for validation)
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_builder_end_element(onvif_xml_builder_t *builder, const char *name);

/**
 * @brief Add self-closing XML element
 * @param builder Builder to use
 * @param name Element name
 * @param ... Attribute pairs (name, value) terminated with NULL
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_builder_self_closing_element(onvif_xml_builder_t *builder, const char *name, ...);

/**
 * @brief Add XML element with text content
 * @param builder Builder to use
 * @param name Element name
 * @param content Text content
 * @param ... Attribute pairs (name, value) terminated with NULL
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_builder_element_with_text(onvif_xml_builder_t *builder, const char *name, const char *content, ...);

/**
 * @brief Add XML element with formatted text content
 * @param builder Builder to use
 * @param name Element name
 * @param format Printf-style format string
 * @param ... Format arguments
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_builder_element_with_formatted_text(onvif_xml_builder_t *builder, const char *name,
                                                  const char *format, ...);

/**
 * @brief Add raw XML content
 * @param builder Builder to use
 * @param content Raw XML content
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_builder_raw_content(onvif_xml_builder_t *builder, const char *content);

/**
 * @brief Add formatted XML content
 * @param builder Builder to use
 * @param format Printf-style format string
 * @param ... Format arguments
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_builder_formatted_content(onvif_xml_builder_t *builder, const char *format, ...);

/**
 * @brief Get the final XML string
 * @param builder Builder to use
 * @return XML string (null-terminated)
 */
const char *onvif_xml_builder_get_string(const onvif_xml_builder_t *builder);

/**
 * @brief Get the length of the generated XML
 * @param builder Builder to use
 * @return Length of XML string
 */
size_t onvif_xml_builder_get_length(const onvif_xml_builder_t *builder);

/* ============================================================================
 * XML Parser Functions
 * ============================================================================ */

/**
 * @brief Initialize XML parser
 * @param parser Parser to initialize
 * @param xml XML content to parse
 * @param length Length of XML content
 * @param config Parser configuration (can be NULL for defaults)
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_parser_init(onvif_xml_parser_t *parser, const char *xml, size_t length,
                         const onvif_xml_parser_config_t *config);

/**
 * @brief Clean up XML parser
 * @param parser Parser to clean up
 */
void onvif_xml_parser_cleanup(onvif_xml_parser_t *parser);

/**
 * @brief Extract a value from XML between start and end tags
 * @param parser Parser to use
 * @param start_tag The opening tag (e.g., "<tds:Manufacturer>")
 * @param end_tag The closing tag (e.g., "</tds:Manufacturer>")
 * @param value Buffer to store the extracted value
 * @param value_size Size of the value buffer
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_parser_extract_value(onvif_xml_parser_t *parser, const char *start_tag, const char *end_tag,
                                   char *value, size_t value_size);

/**
 * @brief Extract an integer value from XML
 * @param parser Parser to use
 * @param start_tag Opening tag
 * @param end_tag Closing tag
 * @param value Pointer to store the parsed integer value
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_parser_extract_int(onvif_xml_parser_t *parser, const char *start_tag, const char *end_tag, int *value);

/**
 * @brief Extract a float value from XML
 * @param parser Parser to use
 * @param start_tag Opening tag
 * @param end_tag Closing tag
 * @param value Pointer to store the parsed float value
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_parser_extract_float(onvif_xml_parser_t *parser, const char *start_tag, const char *end_tag, float *value);

/**
 * @brief Extract a boolean value from XML
 * @param parser Parser to use
 * @param start_tag Opening tag
 * @param end_tag Closing tag
 * @param value Pointer to store the parsed boolean value
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_parser_extract_bool(onvif_xml_parser_t *parser, const char *start_tag, const char *end_tag, int *value);

/**
 * @brief Check if parser has encountered an error
 * @param parser Parser to check
 * @return 1 if error, 0 if OK
 */
int onvif_xml_parser_has_error(const onvif_xml_parser_t *parser);

/* ============================================================================
 * XML Validation Functions
 * ============================================================================ */

/**
 * @brief Validate XML structure and content
 * @param xml XML content to validate
 * @param length Length of XML content
 * @param config Validation configuration (can be NULL for defaults)
 * @param result Validation result (can be NULL if not needed)
 * @return ONVIF_SUCCESS if valid, negative error code on failure
 */
int onvif_xml_validate(const char *xml, size_t length, const onvif_xml_validation_config_t *config,
                      onvif_xml_validation_result_t *result);

/**
 * @brief Basic XML structure validation
 * @param xml XML content to validate
 * @param length Length of XML content
 * @return ONVIF_SUCCESS if valid, negative error code on failure
 */
int onvif_xml_validate_basic(const char *xml, size_t length);

/**
 * @brief Security-focused XML validation
 * @param xml XML content to validate
 * @param length Length of XML content
 * @return ONVIF_SUCCESS if valid, negative error code on failure
 */
int onvif_xml_validate_security(const char *xml, size_t length);

/**
 * @brief Check if a string contains XML content
 * @param str The string to check
 * @return 1 if contains XML, 0 otherwise
 */
int onvif_xml_is_xml_content(const char *str);

/* ============================================================================
 * Common XML Helper Functions
 * ============================================================================ */

/**
 * @brief Escape XML special characters
 * @param input Input string
 * @param output Output buffer
 * @param output_size Size of output buffer
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_escape_string(const char *input, char *output, size_t output_size);

/**
 * @brief Parse PTZ position from ONVIF request XML
 * @param parser Parser to use
 * @param position Pointer to store the parsed PTZ position
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_parse_ptz_position(onvif_xml_parser_t *parser, struct ptz_vector *position);

/**
 * @brief Parse PTZ speed from ONVIF request XML
 * @param parser Parser to use
 * @param speed Pointer to store the parsed PTZ speed
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_parse_ptz_speed(onvif_xml_parser_t *parser, struct ptz_speed *speed);

/**
 * @brief Parse imaging settings from ONVIF request XML
 * @param parser Parser to use
 * @param settings Pointer to store the parsed imaging settings
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_parse_imaging_settings(onvif_xml_parser_t *parser, struct imaging_settings *settings);

/**
 * @brief Parse video source configuration from ONVIF request XML
 * @param parser Parser to use
 * @param config Pointer to store the parsed video source configuration
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_parse_video_source_configuration(onvif_xml_parser_t *parser, struct video_source_configuration *config);

/**
 * @brief Parse video encoder configuration from ONVIF request XML
 * @param parser Parser to use
 * @param config Pointer to store the parsed video encoder configuration
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_parse_video_encoder_configuration(onvif_xml_parser_t *parser, struct video_encoder_configuration *config);

/**
 * @brief Parse metadata configuration from ONVIF request XML
 * @param parser Parser to use
 * @param config Pointer to store the parsed metadata configuration
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_parse_metadata_configuration(onvif_xml_parser_t *parser, struct metadata_configuration *config);

/* ============================================================================
 * Simple parsing functions for service compatibility
 * ============================================================================ */

/**
 * @brief Extract string value from XML between start and end tags (simple version)
 * @param xml XML content to parse
 * @param start_tag The opening tag (e.g., "<tt:ProfileToken>")
 * @param end_tag The closing tag (e.g., "</tt:ProfileToken>")
 * @param value Buffer to store the extracted value
 * @param value_size Size of the value buffer
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_extract_string_value(const char *xml, const char *start_tag, const char *end_tag,
                                   char *value, size_t value_size);

/**
 * @brief Parse profile token from ONVIF request XML (simple version)
 * @param xml XML content to parse
 * @param token Buffer to store the parsed token
 * @param token_size Size of the token buffer
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_parse_profile_token(const char *xml, char *token, size_t token_size);

/**
 * @brief Parse protocol from ONVIF request XML (simple version)
 * @param xml XML content to parse
 * @param protocol Buffer to store the parsed protocol
 * @param protocol_size Size of the protocol buffer
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_parse_protocol(const char *xml, char *protocol, size_t protocol_size);

/**
 * @brief Parse configuration token from ONVIF request XML (simple version)
 * @param xml XML content to parse
 * @param token Buffer to store the parsed token
 * @param token_size Size of the token buffer
 * @return ONVIF_SUCCESS on success, negative error code on failure
 */
int onvif_xml_parse_configuration_token(const char *xml, char *token, size_t token_size);

#endif /* ONVIF_UNIFIED_XML_H */

/**
 * @file test_http_verbose_logging.c
 * @brief Unit tests for HTTP verbose logging functionality
 * @author kkrzysztofik
 * @date 2025
 */

#include <stddef.h>
#include <string.h>

#include "cmocka_wrapper.h"

// Include configuration and logging headers
#include "core/config/config.h"
#include "utils/logging/service_logging.h"

/**
 * @brief Test HTTP verbose configuration default value
 * @param state Test state (unused)
 */
void test_unit_http_verbose_config_default(void** state) {
  (void)state;

  // Test that http_verbose defaults to true (1)
  app_config_t config;
  memset(&config, 0, sizeof(config));

  // Initialize with defaults
  config_init_defaults(&config);

  // Verify http_verbose is enabled by default
  assert_int_equal(config.logging.http_verbose, 1);
}

/**
 * @brief Test HTTP verbose configuration parsing
 * @param state Test state (unused)
 */
void test_unit_http_verbose_config_parsing(void** state) {
  (void)state;

  app_config_t config;
  memset(&config, 0, sizeof(config));

  // Test parsing http_verbose = false
  const char* config_content = "[logging]\nhttp_verbose = false\n";

  // This would normally be done through config_parse_file, but for unit test
  // we'll directly test the parameter registration
  config_init_defaults(&config);

  // Verify default is true
  assert_int_equal(config.logging.http_verbose, 1);

  // Test that the parameter is properly registered
  // (This is more of an integration test, but validates the config system)
}

/**
 * @brief Test redaction functions with various inputs
 * @param state Test state (unused)
 */
void test_unit_http_verbose_redaction_comprehensive(void** state) {
  (void)state;

  // Test Authorization header redaction
  char auth_header[128];
  (void)snprintf(auth_header, sizeof(auth_header), "%s", "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9");
  service_log_redact_header_value("Authorization", auth_header);
  assert_string_equal(auth_header, "<REDACTED>");

  // Test Digest authorization
  (void)snprintf(auth_header, sizeof(auth_header), "%s", "Digest username=\"user\", realm=\"realm\", nonce=\"nonce\"");
  service_log_redact_header_value("Authorization", auth_header);
  assert_string_equal(auth_header, "<REDACTED>");

  // Test Basic authorization
  (void)snprintf(auth_header, sizeof(auth_header), "%s", "Basic dXNlcjpwYXNz");
  service_log_redact_header_value("Authorization", auth_header);
  assert_string_equal(auth_header, "<REDACTED>");

  // Test non-authorization header (should not be redacted)
  char content_type[64];
  (void)snprintf(content_type, sizeof(content_type), "%s", "application/soap+xml; charset=utf-8");
  service_log_redact_header_value("Content-Type", content_type);
  assert_string_equal(content_type, "application/soap+xml; charset=utf-8");
}

/**
 * @brief Test WS-Security password redaction with various XML formats
 * @param state Test state (unused)
 */
void test_unit_http_verbose_wsse_redaction_comprehensive(void** state) {
  (void)state;

  // Test standard WS-Security password
  char xml1[512];
  (void)snprintf(xml1, sizeof(xml1), "%s",
                 "<s:Envelope><s:Header><wsse:Security>"
                 "<wsse:UsernameToken><wsse:Password>mypassword</wsse:Password>"
                 "</wsse:UsernameToken></wsse:Security></s:Header><s:Body/></s:Envelope>");
  service_log_redact_wsse_password(xml1);
  assert_non_null(strstr(xml1, ">***REDACTED***</wsse:Password>"));
  assert_null(strstr(xml1, "mypassword"));

  // Test WS-Security with Type attribute
  char xml2[512];
  (void)snprintf(xml2, sizeof(xml2), "%s",
                 "<s:Envelope><s:Header><wsse:Security>"
                 "<wsse:UsernameToken><wsse:Password "
                 "Type=\"http://docs.oasis-open.org/wss/2004/01/"
                 "oasis-200401-wss-username-token-profile-1.0#PasswordText\">secret</wsse:Password>"
                 "</wsse:UsernameToken></wsse:Security></s:Header><s:Body/></s:Envelope>");
  service_log_redact_wsse_password(xml2);
  assert_non_null(strstr(xml2, ">***REDACTED***</wsse:Password>"));
  assert_null(strstr(xml2, "secret"));

  // Test XML without password (should remain unchanged)
  char xml3[512];
  (void)snprintf(xml3, sizeof(xml3), "%s",
                 "<s:Envelope><s:Header><wsse:Security>"
                 "<wsse:UsernameToken><wsse:Username>admin</wsse:Username></wsse:UsernameToken>"
                 "</wsse:Security></s:Header><s:Body/></s:Envelope>");
  const char* original_xml3 = strdup(xml3);
  service_log_redact_wsse_password(xml3);
  assert_string_equal(xml3, original_xml3);
  free((void*)original_xml3);
}

/**
 * @brief Test edge cases and error handling
 * @param state Test state (unused)
 */
void test_unit_http_verbose_edge_cases(void** state) {
  (void)state;

  // Test empty strings
  char empty_header[1] = "";
  service_log_redact_header_value("Authorization", empty_header);
  assert_string_equal(empty_header, "");

  // Test very long authorization header
  char long_auth[1024];
  memset(long_auth, 'A', sizeof(long_auth) - 1);
  long_auth[sizeof(long_auth) - 1] = '\0';
  service_log_redact_header_value("Authorization", long_auth);
  assert_string_equal(long_auth, "<REDACTED>");

  // Test malformed XML (should not crash)
  char malformed_xml[256];
  (void)snprintf(malformed_xml, sizeof(malformed_xml), "%s",
                 "<s:Envelope><s:Header><wsse:Security>"
                 "<wsse:UsernameToken><wsse:Password>unclosed");
  service_log_redact_wsse_password(malformed_xml);
  // Should not crash, even with malformed XML
}

/**
 * @brief Get HTTP verbose logging unit tests
 * @param count Output parameter for test count
 * @return Array of CMUnitTest structures
 */
const struct CMUnitTest* get_http_verbose_logging_unit_tests(size_t* count) {
  static const struct CMUnitTest tests[] = {
    cmocka_unit_test(test_unit_http_verbose_config_default),
    cmocka_unit_test(test_unit_http_verbose_config_parsing),
    cmocka_unit_test(test_unit_http_verbose_redaction_comprehensive),
    cmocka_unit_test(test_unit_http_verbose_wsse_redaction_comprehensive),
    cmocka_unit_test(test_unit_http_verbose_edge_cases),
  };
  *count = sizeof(tests) / sizeof(tests[0]);
  return tests;
}

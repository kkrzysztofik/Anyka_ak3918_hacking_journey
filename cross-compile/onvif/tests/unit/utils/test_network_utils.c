/**
 * @file test_network_utils.c
 * @brief Unit tests for network utilities
 * @author kkrzysztofik
 * @date 2025
 */

#include "cmocka_wrapper.h"

// Include the actual source files we're testing
#include "utils/network/network_utils.h"

/**
 * @brief Test network utilities initialization
 * @param state Test state (unused)
 */
static void test_network_utils_init(void** state) {
  (void)state;

  // Test network utilities initialization
  int result = onvif_network_init();
  assert_int_equal(result, ONVIF_SUCCESS);

  // Test that initialization is idempotent
  result = onvif_network_init();
  assert_int_equal(result, ONVIF_SUCCESS);

  // Cleanup
  onvif_network_cleanup();
}

/**
 * @brief Test network utilities cleanup
 * @param state Test state (unused)
 */
static void test_network_utils_cleanup(void** state) {
  (void)state;

  // Initialize first
  onvif_network_init();

  // Test cleanup (should not crash)
  onvif_network_cleanup();

  // Test multiple cleanups (should not crash)
  onvif_network_cleanup();
}

/**
 * @brief Test IP address validation
 * @param state Test state (unused)
 */
static void test_ip_address_validation(void** state) {
  (void)state;

  onvif_network_init();

  // Test valid IPv4 addresses
  assert_true(onvif_is_valid_ipv4("192.168.1.1"));
  assert_true(onvif_is_valid_ipv4("127.0.0.1"));
  assert_true(onvif_is_valid_ipv4("0.0.0.0"));
  assert_true(onvif_is_valid_ipv4("255.255.255.255"));

  // Test invalid IPv4 addresses
  assert_false(onvif_is_valid_ipv4("256.1.1.1"));
  assert_false(onvif_is_valid_ipv4("192.168.1"));
  assert_false(onvif_is_valid_ipv4("192.168.1.1.1"));
  assert_false(onvif_is_valid_ipv4("192.168.1.abc"));
  assert_false(onvif_is_valid_ipv4(""));
  assert_false(onvif_is_valid_ipv4(NULL));

  onvif_network_cleanup();
}

/**
 * @brief Test port validation
 * @param state Test state (unused)
 */
static void test_port_validation(void** state) {
  (void)state;

  onvif_network_init();

  // Test valid ports
  assert_true(onvif_is_valid_port(80));
  assert_true(onvif_is_valid_port(443));
  assert_true(onvif_is_valid_port(8080));
  assert_true(onvif_is_valid_port(1));
  assert_true(onvif_is_valid_port(65535));

  // Test invalid ports
  assert_false(onvif_is_valid_port(0));
  assert_false(onvif_is_valid_port(-1));
  assert_false(onvif_is_valid_port(65536));
  assert_false(onvif_is_valid_port(100000));

  onvif_network_cleanup();
}

/**
 * @brief Test URL validation
 * @param state Test state (unused)
 */
static void test_url_validation(void** state) {
  (void)state;

  onvif_network_init();

  // Test valid URLs
  assert_true(onvif_is_valid_url("http://192.168.1.1:80/onvif/device"));
  assert_true(onvif_is_valid_url("https://camera.local:443/service"));
  assert_true(onvif_is_valid_url("rtsp://192.168.1.100:554/stream"));

  // Test invalid URLs
  assert_false(onvif_is_valid_url("invalid://url"));
  assert_false(onvif_is_valid_url("http://"));
  assert_false(onvif_is_valid_url(""));
  assert_false(onvif_is_valid_url(NULL));

  onvif_network_cleanup();
}

/**
 * @brief Test URL parsing
 * @param state Test state (unused)
 */
static void test_url_parsing(void** state) {
  (void)state;

  onvif_network_init();

  url_components_t components;
  memset(&components, 0, sizeof(components));

  // Test parsing valid URL
  const char* test_url = "http://192.168.1.1:8080/onvif/device";
  int result = onvif_parse_url(test_url, &components);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_string_equal(components.scheme, "http");
  assert_string_equal(components.host, "192.168.1.1");
  assert_int_equal(components.port, 8080);
  assert_string_equal(components.path, "/onvif/device");

  // Test parsing URL without port
  const char* test_url2 = "https://camera.local/service";
  result = onvif_parse_url(test_url2, &components);
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_string_equal(components.scheme, "https");
  assert_string_equal(components.host, "camera.local");
  assert_int_equal(components.port, 443); // Default HTTPS port
  assert_string_equal(components.path, "/service");

  // Test parsing with NULL URL
  result = onvif_parse_url(NULL, &components);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test parsing with NULL components
  result = onvif_parse_url(test_url, NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  onvif_network_cleanup();
}

/**
 * @brief Test URL building
 * @param state Test state (unused)
 */
static void test_url_building(void** state) {
  (void)state;

  onvif_network_init();

  url_components_t components;
  char url_buffer[256];

  // Set up components
  strcpy(components.scheme, "http");
  strcpy(components.host, "192.168.1.100");
  components.port = 8080;
  strcpy(components.path, "/onvif/device");

  // Test building URL
  int result = onvif_build_url(&components, url_buffer, sizeof(url_buffer));
  assert_int_equal(result, ONVIF_SUCCESS);
  assert_string_equal(url_buffer, "http://192.168.1.100:8080/onvif/device");

  // Test building with NULL components
  result = onvif_build_url(NULL, url_buffer, sizeof(url_buffer));
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test building with NULL buffer
  result = onvif_build_url(&components, NULL, sizeof(url_buffer));
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test building with zero buffer size
  result = onvif_build_url(&components, url_buffer, 0);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test building with too small buffer
  char small_buffer[10];
  result = onvif_build_url(&components, small_buffer, sizeof(small_buffer));
  assert_int_equal(result, ONVIF_ERROR_BUFFER_TOO_SMALL);

  onvif_network_cleanup();
}

/**
 * @brief Test hostname resolution
 * @param state Test state (unused)
 */
static void test_hostname_resolution(void** state) {
  (void)state;

  onvif_network_init();

  char ip_address[16];

  // Test resolving localhost
  int result = onvif_resolve_hostname("localhost", ip_address, sizeof(ip_address));
  // Result may vary depending on system, so just check it doesn't crash
  (void)result;

  // Test resolving with NULL hostname
  result = onvif_resolve_hostname(NULL, ip_address, sizeof(ip_address));
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test resolving with NULL buffer
  result = onvif_resolve_hostname("localhost", NULL, sizeof(ip_address));
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test resolving with zero buffer size
  result = onvif_resolve_hostname("localhost", ip_address, 0);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  onvif_network_cleanup();
}

/**
 * @brief Test network interface enumeration
 * @param state Test state (unused)
 */
static void test_network_interface_enumeration(void** state) {
  (void)state;

  onvif_network_init();

  network_interface_t interfaces[10];
  int interface_count = 0;

  // Test enumerating network interfaces
  int result = onvif_enumerate_network_interfaces(interfaces, 10, &interface_count);
  // Result may vary depending on system, so just check it doesn't crash
  (void)result;
  (void)interface_count;

  // Test with NULL interfaces array
  result = onvif_enumerate_network_interfaces(NULL, 10, &interface_count);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test with NULL count pointer
  result = onvif_enumerate_network_interfaces(interfaces, 10, NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test with zero max interfaces
  result = onvif_enumerate_network_interfaces(interfaces, 0, &interface_count);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  onvif_network_cleanup();
}

/**
 * @brief Test MAC address validation
 * @param state Test state (unused)
 */
static void test_mac_address_validation(void** state) {
  (void)state;

  onvif_network_init();

  // Test valid MAC addresses
  assert_true(onvif_is_valid_mac_address("00:11:22:33:44:55"));
  assert_true(onvif_is_valid_mac_address("FF:FF:FF:FF:FF:FF"));
  assert_true(onvif_is_valid_mac_address("aa:bb:cc:dd:ee:ff"));

  // Test invalid MAC addresses
  assert_false(onvif_is_valid_mac_address("00:11:22:33:44"));       // Too short
  assert_false(onvif_is_valid_mac_address("00:11:22:33:44:55:66")); // Too long
  assert_false(onvif_is_valid_mac_address("00:11:22:33:44:GG"));    // Invalid hex
  assert_false(onvif_is_valid_mac_address("00-11-22-33-44-55"));    // Wrong separator
  assert_false(onvif_is_valid_mac_address(""));
  assert_false(onvif_is_valid_mac_address(NULL));

  onvif_network_cleanup();
}

/**
 * @brief Test socket creation and binding
 * @param state Test state (unused)
 */
static void test_socket_operations(void** state) {
  (void)state;

  onvif_network_init();

  // Test creating TCP socket
  int tcp_socket = onvif_create_tcp_socket();
  assert_true(tcp_socket >= 0 || tcp_socket == ONVIF_ERROR_SOCKET_CREATE);

  // Test creating UDP socket
  int udp_socket = onvif_create_udp_socket();
  assert_true(udp_socket >= 0 || udp_socket == ONVIF_ERROR_SOCKET_CREATE);

  // Test binding socket (may fail if address is in use)
  if (tcp_socket >= 0) {
    int result = onvif_bind_socket(tcp_socket, "127.0.0.1", 0); // Port 0 = any available
    (void)result; // May succeed or fail depending on system state

    // Close socket
    onvif_close_socket(tcp_socket);
  }

  if (udp_socket >= 0) {
    onvif_close_socket(udp_socket);
  }

  // Test binding with invalid parameters
  int result = onvif_bind_socket(-1, "127.0.0.1", 8080);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  result = onvif_bind_socket(0, NULL, 8080);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  result = onvif_bind_socket(0, "127.0.0.1", 0);
  // May succeed (port 0 = any) or fail depending on socket validity
  (void)result;

  onvif_network_cleanup();
}

/**
 * @brief Test network timeout operations
 * @param state Test state (unused)
 */
static void test_timeout_operations(void** state) {
  (void)state;

  onvif_network_init();

  // Test setting socket timeout
  int socket_fd = onvif_create_tcp_socket();
  if (socket_fd >= 0) {
    int result = onvif_set_socket_timeout(socket_fd, 5000); // 5 seconds
    assert_int_equal(result, ONVIF_SUCCESS);

    // Test setting with zero timeout
    result = onvif_set_socket_timeout(socket_fd, 0);
    assert_int_equal(result, ONVIF_SUCCESS);

    // Test setting with negative timeout
    result = onvif_set_socket_timeout(socket_fd, -1);
    assert_int_equal(result, ONVIF_ERROR_INVALID);

    onvif_close_socket(socket_fd);
  }

  // Test setting timeout on invalid socket
  int result = onvif_set_socket_timeout(-1, 5000);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  onvif_network_cleanup();
}

/**
 * @brief Test network statistics
 * @param state Test state (unused)
 */
static void test_network_statistics(void** state) {
  (void)state;

  onvif_network_init();

  network_stats_t stats;
  memset(&stats, 0, sizeof(stats));

  // Test getting network statistics
  int result = onvif_get_network_statistics(&stats);
  // Result may vary depending on system capabilities
  (void)result;

  // Test with NULL stats pointer
  result = onvif_get_network_statistics(NULL);
  assert_int_equal(result, ONVIF_ERROR_INVALID);

  // Test resetting statistics
  result = onvif_reset_network_statistics();
  // Result may vary depending on system capabilities
  (void)result;

  onvif_network_cleanup();
}

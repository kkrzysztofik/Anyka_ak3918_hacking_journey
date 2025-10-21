/**
 * @file rtsp_auth.c
 * @brief RTSP Authentication Implementation
 * @author kkrzysztofik
 * @date 2025
 *
 * This file contains all authentication-related functions for the RTSP server,
 * including Basic and Digest authentication support.
 */

#include "rtsp_auth.h"

#include <bits/types.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "platform/platform.h"
#include "rtsp_session.h"
#include "rtsp_types.h"
#include "utils/error/error_handling.h"
#include "utils/security/hash_utils.h"

/* ==================== Authentication Functions ==================== */

/**
 * Initialize authentication configuration
 */
int rtsp_auth_init(struct rtsp_auth_config* auth_config) {
  if (!auth_config) {
    return -1;
  }

  memset(auth_config, 0, sizeof(struct rtsp_auth_config));
  auth_config->auth_type = RTSP_AUTH_NONE;
  auth_config->enabled = false;
  auth_config->users = NULL;
  strcpy(auth_config->realm, "RTSP Server");

  return 0;
}

/**
 * Cleanup authentication configuration
 */
void rtsp_auth_cleanup(struct rtsp_auth_config* auth_config) {
  if (!auth_config) {
    return;
  }

  struct rtsp_user* user = auth_config->users;
  while (user) {
    struct rtsp_user* next = user->next;
    free(user);
    user = next;
  }
  auth_config->users = NULL;
}

/**
 * Add user to authentication system
 */
int rtsp_auth_add_user(struct rtsp_auth_config* auth_config, const char* username, const char* password) {
  if (!auth_config || !username || !password) {
    return -1;
  }

  // Check if user already exists
  struct rtsp_user* existing = auth_config->users;
  while (existing) {
    if (strcmp(existing->username, username) == 0) {
      // Update existing user
      strncpy(existing->password, password, sizeof(existing->password) - 1);
      existing->password[sizeof(existing->password) - 1] = '\0';
      return 0;
    }
    existing = existing->next;
  }

  // Create new user
  struct rtsp_user* user = malloc(sizeof(struct rtsp_user));
  if (!user) {
    return -1;
  }

  strncpy(user->username, username, sizeof(user->username) - 1);
  user->username[sizeof(user->username) - 1] = '\0';
  strncpy(user->password, password, sizeof(user->password) - 1);
  user->password[sizeof(user->password) - 1] = '\0';

  user->next = auth_config->users;
  auth_config->users = user;

  return 0;
}

/**
 * Remove user from authentication system
 */
int rtsp_auth_remove_user(struct rtsp_auth_config* auth_config, const char* username) {
  if (!auth_config || !username) {
    return -1;
  }

  struct rtsp_user* user = auth_config->users;
  struct rtsp_user* prev = NULL;

  while (user) {
    if (strcmp(user->username, username) == 0) {
      if (prev) {
        prev->next = user->next;
      } else {
        auth_config->users = user->next;
      }
      free(user);
      return 0;
    }
    prev = user;
    user = user->next;
  }

  return -1; // User not found
}

/**
 * Generate random nonce for digest authentication
 */
void rtsp_auth_generate_nonce(char* nonce, size_t nonce_size) {
  if (!nonce || nonce_size < MD5_HASH_SIZE) {
    return;
  }

  /* Generate cryptographically secure random bytes */
  size_t byte_count = (nonce_size - 1) / 2; /* 2 hex chars per byte */
  uint8_t* random_bytes = malloc(byte_count);
  if (!random_bytes) {
    platform_log_error("Failed to allocate memory for nonce generation\n");
    return;
  }

  if (onvif_generate_random_bytes(random_bytes, byte_count) != ONVIF_SUCCESS) {
    platform_log_error("Failed to generate secure random bytes for nonce\n");
    free(random_bytes);
    return;
  }

  /* Convert bytes to hex string */
  const char hex_chars[] = "0123456789abcdef";
  for (size_t i = 0; i < byte_count && (i * 2 + 1) < nonce_size; i++) {
    nonce[i * 2] = hex_chars[(random_bytes[i] >> HEX_DIGIT_SHIFT) & HEX_DIGIT_MASK];
    nonce[i * 2 + 1] = hex_chars[random_bytes[i] & HEX_DIGIT_MASK];
  }
  nonce[byte_count * 2] = '\0';

  free(random_bytes);
}

/**
 * @brief Extract quoted value from token
 * Helper function to extract value from format: key="value"
 * @return Pointer to the start of the unquoted value
 */
static char* extract_quoted_value(char* value) {
  if (*value == '"') {
    value++;
  }
  char* end = strchr(value, '"');
  if (end) {
    *end = '\0';
  }
  return value;
}

/**
 * @brief Parse Basic authentication credentials
 * Decodes base64-encoded username:password
 */
static int parse_basic_credentials(const char* credentials, char* username, char* password) {
  // Decode base64
  size_t len = strlen(credentials);
  char* decoded = malloc(len + 1);
  if (!decoded) {
    return -1;
  }

  // Simple base64 decode (in production, use proper base64 library)
  int decoded_len = 0;
  for (size_t i = 0; i < len; i += 4) {
    if (i + 3 < len) {
      // Simple base64 decode implementation
      char c1 = credentials[i];     // NOLINT(readability-identifier-length) - standard base64 variable
      char c2 = credentials[i + 1]; // NOLINT(readability-identifier-length) - standard base64 variable
      char c3 = credentials[i + 2]; // NOLINT(readability-identifier-length) - standard base64 variable
      char c4 = credentials[i + 3]; // NOLINT(readability-identifier-length) - standard base64 variable

      // Convert base64 to binary (simplified)
      if (c1 != '=' && c2 != '=') {
        decoded[decoded_len++] = ((c1 - 'A') << 2) | ((c2 - 'A') >> 4);
      }
      if (c2 != '=' && c3 != '=') {
        decoded[decoded_len++] = ((c2 - 'A') << HEX_DIGIT_SHIFT) | ((c3 - 'A') >> 2);
      }
      if (c3 != '=' && c4 != '=') {
        decoded[decoded_len++] = ((c3 - 'A') << BASE64_BITS_PER_CHAR) | (c4 - 'A');
      }
    }
  }
  decoded[decoded_len] = '\0';

  // Find colon separator
  char* colon = strchr(decoded, ':');
  if (colon) {
    *colon = '\0';
    strncpy(username, decoded, RTSP_MAX_USERNAME_LEN - 1);
    strncpy(password, colon + 1, RTSP_MAX_PASSWORD_LEN - 1);
    username[RTSP_MAX_USERNAME_LEN - 1] = '\0';
    password[RTSP_MAX_PASSWORD_LEN - 1] = '\0';
  }

  free(decoded);
  return 0;
}

/**
 * @brief Parse single digest parameter
 * Extracts parameter value based on prefix match
 */
static void parse_digest_parameter(const char* token, char* username, char* realm, char* nonce, char* response) {
  if (strncmp(token, "username=", AUTH_USERNAME_KEY_LEN) == 0) {
    char* value = extract_quoted_value((char*)(token + AUTH_USERNAME_KEY_LEN));
    strncpy(username, value, RTSP_MAX_USERNAME_LEN - 1);
    username[RTSP_MAX_USERNAME_LEN - 1] = '\0';
  } else if (strncmp(token, "realm=", AUTH_REALM_KEY_LEN) == 0 && realm) {
    char* value = extract_quoted_value((char*)(token + AUTH_REALM_KEY_LEN));
    strncpy(realm, value, RTSP_MAX_REALM_LEN - 1);
  } else if (strncmp(token, "nonce=", AUTH_NONCE_KEY_LEN) == 0 && nonce) {
    char* value = extract_quoted_value((char*)(token + AUTH_NONCE_KEY_LEN));
    strncpy(nonce, value, RTSP_MAX_NONCE_LEN - 1);
  } else if (strncmp(token, "response=", AUTH_RESPONSE_KEY_LEN) == 0 && response) {
    char* value = extract_quoted_value((char*)(token + AUTH_RESPONSE_KEY_LEN));
    strncpy(response, value, RTSP_MAX_RESPONSE_LEN - 1);
  }
}

/**
 * @brief Parse Digest authentication credentials
 * Extracts username, realm, nonce, and response from Digest header
 */
static int parse_digest_credentials(const char* credentials, char* username, char* realm, char* nonce, char* response) {
  // Parse digest parameters - need to make a copy for strtok
  size_t len = strlen(credentials);
  char* credentials_copy = malloc(len + 1);
  if (!credentials_copy) {
    return -1;
  }
  strcpy(credentials_copy, credentials);

  char* token = strtok(credentials_copy, ",");
  while (token) {
    // Skip leading whitespace
    while (*token == ' ') {
      token++;
    }

    parse_digest_parameter(token, username, realm, nonce, response);
    token = strtok(NULL, ",");
  }

  free(credentials_copy);
  return 0;
}

/**
 * Parse authentication credentials from header
 */
// NOLINTNEXTLINE(bugprone-easily-swappable-parameters) - standard authentication parameter order
int rtsp_auth_parse_credentials(const char* auth_header, char* username, char* password, char* realm, char* nonce, char* response) {
  if (!auth_header || !username || !password) {
    return -1;
  }

  // Determine authentication type and parse accordingly
  if (strncmp(auth_header, "Basic ", AUTH_BASIC_PREFIX_LEN) == 0) {
    return parse_basic_credentials(auth_header + AUTH_BASIC_PREFIX_LEN, username, password);
  }
  if (strncmp(auth_header, "Digest ", AUTH_DIGEST_PREFIX_LEN) == 0) {
    return parse_digest_credentials(auth_header + AUTH_DIGEST_PREFIX_LEN, username, realm, nonce, response);
  }

  return -1;
}

/**
 * Validate Basic authentication
 */
int rtsp_auth_validate_basic(rtsp_session_t* session, const char* auth_header) {
  if (!session || !auth_header) {
    return -1;
  }

  char username[RTSP_MAX_USERNAME_LEN];
  char password[RTSP_MAX_PASSWORD_LEN];

  if (rtsp_auth_parse_credentials(auth_header, username, password, NULL, NULL, NULL) < 0) {
    return -1;
  }

  // Check against user database
  struct rtsp_user* user = session->server->auth_config.users;
  while (user) {
    if (strcmp(user->username, username) == 0 && strcmp(user->password, password) == 0) {
      session->authenticated = true;
      strncpy(session->auth_username, username, sizeof(session->auth_username) - 1);
      session->auth_username[sizeof(session->auth_username) - 1] = '\0';
      return 0;
    }
    user = user->next;
  }

  return -1; // Authentication failed
}

/**
 * Verify digest authentication response
 */
int rtsp_auth_verify_digest(const char* username, const char* password, const char* realm, const char* nonce, const char* method, const char* uri,
                            const char* response) {
  if (!username || !password || !realm || !nonce || !method || !uri || !response) {
    return -1;
  }

  // Generate expected response
  char ha1_input[DIGEST_AUTH_BUFFER_SIZE];
  char ha2_input[DIGEST_AUTH_BUFFER_SIZE];
  char response_input[DIGEST_AUTH_BUFFER_SIZE];

  // HA1 = MD5(username:realm:password)
  (void)snprintf(ha1_input, sizeof(ha1_input), "%s:%s:%s", username, realm, password);

  // HA2 = MD5(method:uri)
  (void)snprintf(ha2_input, sizeof(ha2_input), "%s:%s", method, uri);

  // Response = MD5(HA1:nonce:HA2)
  (void)snprintf(response_input, sizeof(response_input), "%s:%s:%s", ha1_input, nonce, ha2_input);

  // Simple MD5 comparison (in production, use proper MD5 library)
  // For now, just do string comparison as placeholder
  return (strcmp(response, response_input) == 0) ? 0 : -1;
}

/**
 * Validate Digest authentication
 */
int rtsp_auth_validate_digest(rtsp_session_t* session, const char* auth_header, const char* method, const char* uri) {
  if (!session || !auth_header || !method || !uri) {
    return -1;
  }

  char username[RTSP_MAX_USERNAME_LEN];
  char password[RTSP_MAX_PASSWORD_LEN];
  char realm[RTSP_MAX_REALM_LEN];
  char nonce[RTSP_MAX_NONCE_LEN];
  char response[RTSP_MAX_RESPONSE_LEN];

  if (rtsp_auth_parse_credentials(auth_header, username, password, realm, nonce, response) < 0) {
    return -1;
  }

  // Check if nonce matches
  if (strcmp(nonce, session->server->auth_config.nonce) != 0) {
    return -1;
  }

  // Find user in database
  struct rtsp_user* user = session->server->auth_config.users;
  while (user) {
    if (strcmp(user->username, username) == 0) {
      if (rtsp_auth_verify_digest(username, user->password, realm, nonce, method, uri, response) == 0) {
        session->authenticated = true;
        strncpy(session->auth_username, username, sizeof(session->auth_username) - 1);
        session->auth_username[sizeof(session->auth_username) - 1] = '\0';
        return 0;
      }
      break;
    }
    user = user->next;
  }

  return -1; // Authentication failed
}

/**
 * Check if authentication is required
 */
int rtsp_auth_require_auth(rtsp_session_t* session) {
  if (!session || !session->server) {
    return 0;
  }

  if (!session->server->auth_config.enabled) {
    return 0; // No auth required
  }

  if (session->authenticated) {
    return 0; // Already authenticated
  }

  return 1; // Auth required
}

/**
 * Handle authentication required response
 */
int rtsp_handle_auth_required(rtsp_session_t* session) {
  if (!session) {
    return -1;
  }

  char www_auth_header[WWW_AUTH_HEADER_SIZE];
  if (rtsp_generate_www_authenticate_header(session, www_auth_header, sizeof(www_auth_header)) < 0) {
    return -1;
  }

  return rtsp_send_error_response(session, RTSP_UNAUTHORIZED, www_auth_header);
}

/**
 * Generate WWW-Authenticate header
 */
int rtsp_generate_www_authenticate_header(rtsp_session_t* session, char* header, size_t header_size) {
  if (!session || !header || header_size < WWW_AUTH_MIN_SIZE) {
    return -1;
  }

  if (session->server->auth_config.auth_type == RTSP_AUTH_BASIC) {
    (void)snprintf(header, header_size, "WWW-Authenticate: Basic realm=\"%s\"\r\n", session->server->auth_config.realm);
  } else if (session->server->auth_config.auth_type == RTSP_AUTH_DIGEST) {
    // Generate new nonce
    rtsp_auth_generate_nonce(session->server->auth_config.nonce, sizeof(session->server->auth_config.nonce));
    strncpy(session->auth_nonce, session->server->auth_config.nonce, sizeof(session->auth_nonce) - 1);
    session->auth_nonce[sizeof(session->auth_nonce) - 1] = '\0';

    (void)snprintf(header, header_size,
                   "WWW-Authenticate: Digest realm=\"%s\", nonce=\"%s\", "
                   "algorithm=MD5\r\n",
                   session->server->auth_config.realm, session->server->auth_config.nonce);
  } else {
    return -1;
  }

  return 0;
}

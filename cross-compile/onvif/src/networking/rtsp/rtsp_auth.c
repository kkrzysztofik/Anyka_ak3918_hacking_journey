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

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "rtsp_session.h"
#include "rtsp_types.h"

/* ==================== Authentication Functions ==================== */

/**
 * Initialize authentication configuration
 */
int rtsp_auth_init(struct rtsp_auth_config* auth_config) {
  if (!auth_config)
    return -1;

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
  if (!auth_config)
    return;

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
int rtsp_auth_add_user(struct rtsp_auth_config* auth_config, const char* username,
                       const char* password) {
  if (!auth_config || !username || !password)
    return -1;

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
  if (!user)
    return -1;

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
  if (!auth_config || !username)
    return -1;

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
  if (!nonce || nonce_size < 16)
    return;

  const char charset[] = "0123456789abcdef";
  srand((unsigned int)time(NULL));

  for (size_t i = 0; i < nonce_size - 1; i++) {
    nonce[i] = charset[rand() % (sizeof(charset) - 1)];
  }
  nonce[nonce_size - 1] = '\0';
}

/**
 * Parse authentication credentials from header
 */
int rtsp_auth_parse_credentials(const char* auth_header, char* username, char* password,
                                char* realm, char* nonce, char* response) {
  if (!auth_header || !username || !password)
    return -1;

  // Skip "Basic " or "Digest " prefix
  const char* credentials = auth_header;
  if (strncmp(credentials, "Basic ", 6) == 0) {
    credentials += 6;

    // Decode base64
    size_t len = strlen(credentials);
    char* decoded = malloc(len + 1);
    if (!decoded)
      return -1;

    // Simple base64 decode (in production, use proper base64 library)
    int decoded_len = 0;
    for (size_t i = 0; i < len; i += 4) {
      if (i + 3 < len) {
        // Simple base64 decode implementation
        char c1 = credentials[i];
        char c2 = credentials[i + 1];
        char c3 = credentials[i + 2];
        char c4 = credentials[i + 3];

        // Convert base64 to binary (simplified)
        if (c1 != '=' && c2 != '=') {
          decoded[decoded_len++] = ((c1 - 'A') << 2) | ((c2 - 'A') >> 4);
        }
        if (c2 != '=' && c3 != '=') {
          decoded[decoded_len++] = ((c2 - 'A') << 4) | ((c3 - 'A') >> 2);
        }
        if (c3 != '=' && c4 != '=') {
          decoded[decoded_len++] = ((c3 - 'A') << 6) | (c4 - 'A');
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

  } else if (strncmp(credentials, "Digest ", 7) == 0) {
    credentials += 7;

    // Parse digest parameters
    char* token = strtok((char*)credentials, ",");
    while (token) {
      // Skip whitespace
      while (*token == ' ')
        token++;

      if (strncmp(token, "username=", 9) == 0) {
        char* value = token + 9;
        if (*value == '"')
          value++;
        char* end = strchr(value, '"');
        if (end)
          *end = '\0';
        strncpy(username, value, RTSP_MAX_USERNAME_LEN - 1);
        username[RTSP_MAX_USERNAME_LEN - 1] = '\0';
      } else if (strncmp(token, "realm=", 6) == 0) {
        char* value = token + 6;
        if (*value == '"')
          value++;
        char* end = strchr(value, '"');
        if (end)
          *end = '\0';
        if (realm)
          strncpy(realm, value, RTSP_MAX_REALM_LEN - 1);
      } else if (strncmp(token, "nonce=", 6) == 0) {
        char* value = token + 6;
        if (*value == '"')
          value++;
        char* end = strchr(value, '"');
        if (end)
          *end = '\0';
        if (nonce)
          strncpy(nonce, value, RTSP_MAX_NONCE_LEN - 1);
      } else if (strncmp(token, "response=", 9) == 0) {
        char* value = token + 9;
        if (*value == '"')
          value++;
        char* end = strchr(value, '"');
        if (end)
          *end = '\0';
        if (response)
          strncpy(response, value, RTSP_MAX_RESPONSE_LEN - 1);
      }

      token = strtok(NULL, ",");
    }

    return 0;
  }

  return -1;
}

/**
 * Validate Basic authentication
 */
int rtsp_auth_validate_basic(rtsp_session_t* session, const char* auth_header) {
  if (!session || !auth_header)
    return -1;

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
int rtsp_auth_verify_digest(const char* username, const char* password, const char* realm,
                            const char* nonce, const char* method, const char* uri,
                            const char* response) {
  if (!username || !password || !realm || !nonce || !method || !uri || !response)
    return -1;

  // Generate expected response
  char ha1_input[512];
  char ha2_input[512];
  char response_input[512];

  // HA1 = MD5(username:realm:password)
  snprintf(ha1_input, sizeof(ha1_input), "%s:%s:%s", username, realm, password);

  // HA2 = MD5(method:uri)
  snprintf(ha2_input, sizeof(ha2_input), "%s:%s", method, uri);

  // Response = MD5(HA1:nonce:HA2)
  snprintf(response_input, sizeof(response_input), "%s:%s:%s", ha1_input, nonce, ha2_input);

  // Simple MD5 comparison (in production, use proper MD5 library)
  // For now, just do string comparison as placeholder
  return (strcmp(response, response_input) == 0) ? 0 : -1;
}

/**
 * Validate Digest authentication
 */
int rtsp_auth_validate_digest(rtsp_session_t* session, const char* auth_header, const char* method,
                              const char* uri) {
  if (!session || !auth_header || !method || !uri)
    return -1;

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
      if (rtsp_auth_verify_digest(username, user->password, realm, nonce, method, uri, response) ==
          0) {
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
  if (!session || !session->server)
    return 0;

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
  if (!session)
    return -1;

  char www_auth_header[512];
  if (rtsp_generate_www_authenticate_header(session, www_auth_header, sizeof(www_auth_header)) <
      0) {
    return -1;
  }

  return rtsp_send_error_response(session, RTSP_UNAUTHORIZED, www_auth_header);
}

/**
 * Generate WWW-Authenticate header
 */
int rtsp_generate_www_authenticate_header(rtsp_session_t* session, char* header,
                                          size_t header_size) {
  if (!session || !header || header_size < 64)
    return -1;

  if (session->server->auth_config.auth_type == RTSP_AUTH_BASIC) {
    snprintf(header, header_size, "WWW-Authenticate: Basic realm=\"%s\"\r\n",
             session->server->auth_config.realm);
  } else if (session->server->auth_config.auth_type == RTSP_AUTH_DIGEST) {
    // Generate new nonce
    rtsp_auth_generate_nonce(session->server->auth_config.nonce,
                             sizeof(session->server->auth_config.nonce));
    strncpy(session->auth_nonce, session->server->auth_config.nonce,
            sizeof(session->auth_nonce) - 1);
    session->auth_nonce[sizeof(session->auth_nonce) - 1] = '\0';

    snprintf(header, header_size,
             "WWW-Authenticate: Digest realm=\"%s\", nonce=\"%s\", "
             "algorithm=MD5\r\n",
             session->server->auth_config.realm, session->server->auth_config.nonce);
  } else {
    return -1;
  }

  return 0;
}

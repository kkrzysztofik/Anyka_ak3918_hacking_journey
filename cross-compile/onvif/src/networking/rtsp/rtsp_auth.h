/**
 * @file rtsp_auth.h
 * @brief RTSP Authentication Functions
 * @author kkrzysztofik
 * @date 2025
 *
 * This file contains all authentication-related function declarations
 * for the RTSP server, including Basic and Digest authentication support.
 */

#ifndef RTSP_AUTH_H
#define RTSP_AUTH_H

#include "rtsp_types.h"

/* Authentication functions */
/**
 * @brief Initialize authentication configuration
 * @param auth_config Authentication configuration to initialize
 * @return 0 on success, -1 on error
 */
int rtsp_auth_init(struct rtsp_auth_config* auth_config);

/**
 * @brief Cleanup authentication configuration
 * @param auth_config Authentication configuration to cleanup
 */
void rtsp_auth_cleanup(struct rtsp_auth_config* auth_config);

/**
 * @brief Add a user to the authentication system
 * @param auth_config Authentication configuration
 * @param username Username to add
 * @param password Password for the user
 * @return 0 on success, -1 on error
 */
int rtsp_auth_add_user(struct rtsp_auth_config* auth_config, const char* username,
                       const char* password);

/**
 * @brief Remove a user from the authentication system
 * @param auth_config Authentication configuration
 * @param username Username to remove
 * @return 0 on success, -1 on error
 */
int rtsp_auth_remove_user(struct rtsp_auth_config* auth_config, const char* username);

/**
 * @brief Validate Basic authentication credentials
 * @param session RTSP session
 * @param auth_header Authorization header value
 * @return 0 on success, -1 on error
 */
int rtsp_auth_validate_basic(rtsp_session_t* session, const char* auth_header);

/**
 * @brief Validate Digest authentication credentials
 * @param session RTSP session
 * @param auth_header Authorization header value
 * @param method HTTP method used
 * @param uri Request URI
 * @return 0 on success, -1 on error
 */
int rtsp_auth_validate_digest(rtsp_session_t* session, const char* auth_header, const char* method,
                              const char* uri);

/**
 * @brief Check if authentication is required for the session
 * @param session RTSP session
 * @return 0 if auth required, -1 if not required
 */
int rtsp_auth_require_auth(rtsp_session_t* session);

/**
 * @brief Generate a nonce for Digest authentication
 * @param nonce Buffer to store the nonce
 * @param nonce_size Size of the nonce buffer
 */
void rtsp_auth_generate_nonce(char* nonce, size_t nonce_size);

/**
 * @brief Parse authentication credentials from header
 * @param auth_header Authorization header value
 * @param username Buffer to store username
 * @param password Buffer to store password
 * @param realm Buffer to store realm
 * @param nonce Buffer to store nonce
 * @param response Buffer to store response
 * @return 0 on success, -1 on error
 */
int rtsp_auth_parse_credentials(const char* auth_header, char* username, char* password,
                                char* realm, char* nonce, char* response);

/**
 * @brief Verify Digest authentication response
 * @param username Username
 * @param password Password
 * @param realm Realm
 * @param nonce Nonce
 * @param method HTTP method
 * @param uri Request URI
 * @param response Response to verify
 * @return 0 on success, -1 on error
 */
int rtsp_auth_verify_digest(const char* username, const char* password, const char* realm,
                            const char* nonce, const char* method, const char* uri,
                            const char* response);

#endif /* RTSP_AUTH_H */

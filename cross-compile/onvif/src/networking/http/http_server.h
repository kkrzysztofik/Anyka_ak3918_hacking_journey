/**
 * @file http_server.h
 * @brief Minimal blocking HTTP server exposing ONVIF SOAP endpoints
 * @author kkrzysztofik
 * @date 2025
 */
#ifndef HTTP_SERVER_H
#define HTTP_SERVER_H

/**
 * @brief Start the ONVIF HTTP/SOAP server.
 * @param port TCP port to bind.
 * @return 0 on success, negative on failure.
 */
int http_server_start(int port);

/**
 * @brief Stop the HTTP server and release resources.
 * @return 0 on success, negative on failure.
 */
int http_server_stop(void);

/**
 * @brief Process a single connection (used by thread pool)
 * @param conn Connection to process
 */
void process_connection(void *conn);

#endif
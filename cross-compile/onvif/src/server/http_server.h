/**
 * @file http_server.h
 * @brief Minimal blocking HTTP server exposing ONVIF SOAP endpoints.
 *
 * The HTTP server listens on a configurable TCP port and dispatches
 * incoming SOAP POST requests to the ONVIF service handlers (Device,
 * Media, PTZ, Imaging). It is intentionally lightweight and single
 * threaded per accepted connection; each connection is processed in
 * the accept loop thread (no keep-alive support currently).
 *
 * Typical lifecycle:
 *  - http_server_start(port)
 *  - (server thread handles requests)
 *  - http_server_stop()
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

#endif
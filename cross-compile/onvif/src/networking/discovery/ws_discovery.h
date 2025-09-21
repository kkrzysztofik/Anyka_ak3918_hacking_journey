/**
 * @file ws_discovery.h
 * @brief Minimal WS-Discovery (ONVIF) responder
 * @author kkrzysztofik
 * @date 2025
 */
#ifndef WS_DISCOVERY_H
#define WS_DISCOVERY_H

/* Start WS-Discovery responder.
 * http_port: port where the ONVIF device_service is exposed.
 * Returns 0 on success, <0 on error.
 */
int ws_discovery_start(int http_port);

/* Stop responder (idempotent). */
int ws_discovery_stop(void);

#endif /* WS_DISCOVERY_H */

#ifndef VENDOR_DAEMON_HANDLERS_ISP_H
#define VENDOR_DAEMON_HANDLERS_ISP_H

#include <stdint.h>

/*
 * handle_isp_effect - generic ISP effect setter.
 * effect_type is passed as int to avoid pulling ak_vpss.h into this header;
 * the caller (dispatcher.c) includes ak_vpss.h to obtain VPSS_EFFECT_* values.
 */
int handle_isp_effect(int fd, const uint8_t *req, uint32_t req_len,
                      int effect_type, const char *name);

int handle_isp_set_wdr(int fd, const uint8_t *req, uint32_t req_len);
int handle_isp_set_ir_filter(int fd, const uint8_t *req, uint32_t req_len);

#endif /* VENDOR_DAEMON_HANDLERS_ISP_H */

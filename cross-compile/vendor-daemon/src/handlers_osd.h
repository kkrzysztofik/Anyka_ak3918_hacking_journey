#ifndef VENDOR_DAEMON_HANDLERS_OSD_H
#define VENDOR_DAEMON_HANDLERS_OSD_H

#include <stdint.h>

int handle_osd_init(int fd, const uint8_t *req, uint32_t req_len);
int handle_osd_set_rect(int fd, const uint8_t *req, uint32_t req_len);
int handle_osd_draw_str(int fd, const uint8_t *req, uint32_t req_len);
int handle_osd_set_enable(int fd, const uint8_t *req, uint32_t req_len);
int handle_osd_set_style(int fd, const uint8_t *req, uint32_t req_len);

/** Tear down OSD state.  Call from the VI close path, before ak_vi_close. */
void osd_shutdown(void);

#endif /* VENDOR_DAEMON_HANDLERS_OSD_H */

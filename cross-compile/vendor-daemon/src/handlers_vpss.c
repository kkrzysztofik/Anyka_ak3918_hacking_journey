#include "handlers_vpss.h"
#include "ipc.h"
#include "protocol.h"
#include "log.h"

/**
 * handle_vpss_init - IPC handler for CMD_VPSS_INIT (no-op).
 *
 * ak_vpss_init() is not exported by the libre_anyka_app SDK variant; this
 * function always responds STATUS_OK without calling any SDK function.
 * VPSS lifecycle is managed internally by the VI subsystem in this SDK
 * variant — no explicit init call is needed or possible.
 *
 * @param fd      Client socket file descriptor.
 * @param req     Ignored.
 * @param req_len Ignored.
 * @return        0 on success, -1 on write error.
 */
int handle_vpss_init(int fd, const uint8_t *req, uint32_t req_len)
{
    (void)req;
    (void)req_len;
    log_debug("[vpss] init: no-op (libre_anyka_app SDK)");
    return send_response(fd, STATUS_OK, NULL, 0);
}

/**
 * handle_vpss_destroy - IPC handler for CMD_VPSS_DESTROY (no-op).
 *
 * ak_vpss_destroy() is not exported by the libre_anyka_app SDK variant; this
 * function always responds STATUS_OK without calling any SDK function.
 * VPSS teardown is managed internally by the VI subsystem in this SDK variant.
 *
 * @param fd      Client socket file descriptor.
 * @param req     Ignored.
 * @param req_len Ignored.
 * @return        0 on success, -1 on write error.
 */
int handle_vpss_destroy(int fd, const uint8_t *req, uint32_t req_len)
{
    (void)req;
    (void)req_len;
    log_debug("[vpss] destroy: no-op (libre_anyka_app SDK)");
    return send_response(fd, STATUS_OK, NULL, 0);
}

/**
 * vendor-daemon - Anyka SDK IPC bridge
 *
 * Receives binary IPC commands from the Rust onvif-rust server over a Unix
 * domain socket (/tmp/vendor-daemon.sock) and dispatches them to the real
 * Anyka SDK C API calls.
 *
 * Protocol (little-endian):
 *   Request:  [i32 cmd_id][u32 req_len][req_data bytes]
 *   Response: [i32 status][u32 resp_len][resp_data bytes]
 *
 * Dual Socket Mode:
 *   - Control socket: /tmp/vd-ctrl.sock (formerly /tmp/vendor-daemon.sock)
 *     Handles all IPC commands including lifecycle operations
 *   - Frame main socket: /tmp/vd-frame-main.sock
 *     Dedicated notification channel for main stream push frames
 *   - Frame sub socket: /tmp/vd-frame-sub.sock
 *     Dedicated notification channel for sub stream push frames
 *
 * Shared Memory Ring Buffer (Approach A):
 *   - Zero-copy frame delivery via shared memory
 *   - 12-byte notification protocol on frame socket
 *   - Falls back to socket-based delivery on ring buffer overflow
 *
 * Connection model:
 *   poll()-based multiplexing of up to MAX_CLIENTS concurrent connections.
 *   The first client to issue a lifecycle command (vi_open, venc_open, etc.)
 *   becomes the "control client" — only it may call lifecycle commands.
 *   Additional clients are restricted to streaming ops (set_iframe, set_rc)
 *   and queries (get_error_*, isp_*).  When the control client
 *   disconnects, the slot opens for the next lifecycle command.
 *
 * Build (old Anyka toolchain):
 *   arm-anykav200-linux-uclibcgnueabi-gcc -std=gnu99 -march=armv5te \
 *     -mfloat-abi=soft -o vendor-daemon main.c \
 *     -I<sdk-include-dir> -L<sdk-lib-dir> \
 *     -lplat_vi -lplat_vpss -lplat_venc_cb -lmpi_venc ...
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <errno.h>
#include <signal.h>
#include <unistd.h>
#include <sys/socket.h>
#include <poll.h>
#include <pthread.h>
#include "log.h"
#include "ak_common.h"
#include "ak_global.h"
#include "globals.h"
#include "ipc.h"
#include "dispatcher.h"
#include "push.h"
#include "protocol.h"
#include "vd_ring_buffer.h"

/**
 * signal_handler - SIGINT/SIGTERM signal handler.
 *
 * Sets the global g_shutdown flag to 1, which causes the main event loop
 * to exit cleanly on the next poll() iteration.
 *
 * @param sig  Signal number received; unused.
 */
static void signal_handler(int sig)
{
    (void)sig;
    g_shutdown = 1;
}

/* ---- main ---------------------------------------------------------------- */

/* NOTE: all handlers, the dispatcher, IPC helpers, push threads, and global
 * variable definitions have been moved to the following modules:
 *   globals.c       - global variable storage
 *   ipc.c           - read_exact, write_exact, send_response, socket helpers
 *   push.c          - push-mode frame delivery and push thread
 *   handlers_vi.c   - VI command handlers
 *   handlers_vpss.c - VPSS command handlers
 *   handlers_venc.c - VENC command handlers (non-push)
 *   handlers_audio.c - AI/AENC command handlers
 *   handlers_isp.c  - ISP/imaging command handlers
 *   dispatcher.c    - process_request, release_control
 */

/**
 * main - Daemon entry point.
 *
 * Initialises logging (respecting the VENDOR_DAEMON_LOG_FILE environment
 * variable), configures SIGINT/SIGTERM handling, initialises the shared
 * memory ring buffer, creates the control and frame Unix domain sockets,
 * then runs the poll()-based event loop to multiplex client connections
 * until g_shutdown is set.
 *
 * @param argc  Argument count; unused.
 * @param argv  Argument vector; unused.
 * @return      0 on clean shutdown.
 */
int main(int argc, char *argv[])
{
    (void)argc;
    (void)argv;

    /* ================================================================
     * LOG FILE PATH INITIALIZATION
     * ================================================================ */

    /* Determine log file path from environment variable or use default.
     * Env var: VENDOR_DAEMON_LOG_FILE
     * Default: /mnt/logs/vendor_daemon.log
     */
    char log_file_path[LOG_FILE_PATH_MAX];
    const char *env_log_path = getenv("VENDOR_DAEMON_LOG_FILE");
    if (env_log_path && env_log_path[0] != '\0') {
        strncpy(log_file_path, env_log_path, LOG_FILE_PATH_MAX - 1);
        log_file_path[LOG_FILE_PATH_MAX - 1] = '\0';
    } else {
        strncpy(log_file_path, LOG_FILE_PATH_DEFAULT, LOG_FILE_PATH_MAX - 1);
        log_file_path[LOG_FILE_PATH_MAX - 1] = '\0';
    }

    /* ================================================================
     * LOGGING INITIALIZATION
     * ================================================================ */

    /* Open log file */
    g_log_fp = fopen(log_file_path, "a");
    if (!g_log_fp) {
        fprintf(stderr, "Failed to open log file %s: %s\n",
                log_file_path, strerror(errno));
        g_log_fp = NULL;
    }

    /* Save original stdout/stderr for restoration on shutdown */
    g_saved_stdout = dup(STDOUT_FILENO);
    g_saved_stderr = dup(STDERR_FILENO);

    /* Redirect stdout/stderr to log file to capture Anyka SDK ak_print() */
    if (g_log_fp) {
        dup2(fileno(g_log_fp), STDOUT_FILENO);
        dup2(fileno(g_log_fp), STDERR_FILENO);
    }

    /* Initialize log.c - quiet mode suppresses stderr (we redirected it) */
    log_set_level(LOG_DEBUG);
    if (g_log_fp) {
        log_add_fp(g_log_fp, LOG_DEBUG);
        log_set_quiet(true);  /* Only write via file callback, not stderr */
    }

    log_info("========================================");
    log_info("vendor-daemon starting");
    log_info("log file: %s", log_file_path);

    /* Configure Anyka SDK logging:
     * - Set print level to DEBUG so all SDK messages go to stdout
     *   (which we redirected to our log file)
     * - Disable syslog output to avoid duplicate messages
     */
    ak_print_set_level(LOG_LEVEL_DEBUG);
    ak_print_set_syslog_level(0);
    log_info("SDK logging: level=DEBUG, syslog=disabled");

    /* ================================================================
     * SIGNAL HANDLING
     * ================================================================ */

    struct sigaction sa;
    memset(&sa, 0, sizeof(sa));
    sa.sa_handler = signal_handler;
    sigemptyset(&sa.sa_mask);
    sigaction(SIGINT,  &sa, NULL);
    sigaction(SIGTERM, &sa, NULL);

    /* Ignore SIGPIPE so write errors return EPIPE instead of killing us */
    signal(SIGPIPE, SIG_IGN);

    /* Initialise push stream state */
    memset(g_push_streams, 0, sizeof(g_push_streams));

    /* ================================================================
     * SHARED MEMORY RING BUFFER INITIALIZATION (Approach A)
     * ================================================================ */

    g_ring_buffer = vd_ring_create();
    if (g_ring_buffer) {
        log_info("[daemon] shared memory ring buffer initialized (%d slots, %d bytes/slot)",
                 VD_SHM_SLOT_COUNT, VD_SHM_SLOT_DATA_SIZE);
    } else {
        log_warn("[daemon] shared memory init failed, using socket-only mode");
    }

    /* ================================================================
     * SOCKET SETUP
     * ================================================================ */

    /* Control socket: handles all IPC commands */
    g_ctrl_server_fd = create_unix_socket(CTRL_SOCKET_PATH);
    if (g_ctrl_server_fd < 0) {
        log_fatal("[daemon] control socket creation failed");
        goto shutdown;
    }
    log_info("[daemon] control socket listening on %s", CTRL_SOCKET_PATH);

    /* Dedicated notification channel for main stream. */
    g_frame_main_server_fd = create_unix_socket(FRAME_MAIN_SOCKET_PATH);
    if (g_frame_main_server_fd < 0) {
        log_fatal("[daemon] main frame socket creation failed");
        goto shutdown;
    }
    log_info("[daemon] main frame socket listening on %s", FRAME_MAIN_SOCKET_PATH);

    /* Dedicated notification channel for sub stream. */
    g_frame_sub_server_fd = create_unix_socket(FRAME_SUB_SOCKET_PATH);
    if (g_frame_sub_server_fd < 0) {
        log_fatal("[daemon] sub frame socket creation failed");
        goto shutdown;
    }
    log_info("[daemon] sub frame socket listening on %s", FRAME_SUB_SOCKET_PATH);

    /* ================================================================
     * MAIN EVENT LOOP (poll-based multiplexing)
     * ================================================================
     *
     * fds[0]         = ctrl_server_fd
     * fds[1]         = frame_main_server_fd
     * fds[2]         = frame_sub_server_fd
     * fds[3..nfds-1] = connected client fds
     *
     * This replaces the old blocking accept()-then-inner-loop model so
     * multiple clients can be serviced without queuing in the kernel
     * backlog.  Each client with pending data is serviced round-robin.
     *
     * Each frame socket accepts at most 1 client.
     */

    /* pollfd array: [ctrl_server, frame_main_server, frame_sub_server, clients...] */
    struct pollfd fds[MAX_CLIENTS + 3];
    int nfds = 0;

    memset(fds, 0, sizeof(fds));

    /* Add control server socket */
    fds[nfds].fd = g_ctrl_server_fd;
    fds[nfds].events = POLLIN;
    nfds++;

    /* Add frame main server socket */
    fds[nfds].fd = g_frame_main_server_fd;
    fds[nfds].events = POLLIN;
    nfds++;

    /* Add frame sub server socket */
    fds[nfds].fd = g_frame_sub_server_fd;
    fds[nfds].events = POLLIN;
    nfds++;

    while (!g_shutdown) {
        int ready = poll(fds, nfds, 1000); /* 1s timeout for shutdown check */
        if (ready < 0) {
            if (errno == EINTR)
                continue;
            log_error("poll: %s", strerror(errno));
            break;
        }
        if (ready == 0)
            continue; /* timeout – re-check g_shutdown */

        /* ── Accept new connections on control socket ─────────────────── */
        if (fds[0].revents & POLLIN) {
            int client_fd = accept(g_ctrl_server_fd, NULL, NULL);
            if (client_fd >= 0) {
                if (nfds < MAX_CLIENTS + 3) {
                    fds[nfds].fd = client_fd;
                    fds[nfds].events = POLLIN;
                    fds[nfds].revents = 0;
                    nfds++;
                    log_info("[daemon] control client connected fd=%d (total=%d)",
                             client_fd, nfds - 1);
                } else {
                    log_error("[daemon] max clients (%d) reached, rejecting fd=%d",
                              MAX_CLIENTS, client_fd);
                    close(client_fd);
                }
            } else if (errno != EINTR) {
                log_error("accept (control): %s", strerror(errno));
            }
        }

        /* ── Accept new connection on main frame socket ──────────────── */
        if (fds[1].revents & POLLIN) {
            int client_fd = accept(g_frame_main_server_fd, NULL, NULL);
            if (client_fd >= 0) {
                int has_existing = 0;
                pthread_mutex_lock(&g_frame_main_client_lock);
                has_existing = (g_frame_main_client_fd >= 0);
                if (!has_existing) {
                    g_frame_main_client_fd = client_fd;
                }
                pthread_mutex_unlock(&g_frame_main_client_lock);

                if (has_existing) {
                    log_warn("[daemon] main frame client already connected, rejecting fd=%d", client_fd);
                    close(client_fd);
                } else if (nfds < MAX_CLIENTS + 3) {
                    fds[nfds].fd = client_fd;
                    fds[nfds].events = POLLIN;
                    fds[nfds].revents = 0;
                    nfds++;
                    log_info("[daemon] main frame client connected fd=%d", client_fd);
                } else {
                    pthread_mutex_lock(&g_frame_main_client_lock);
                    if (g_frame_main_client_fd == client_fd) {
                        g_frame_main_client_fd = -1;
                    }
                    pthread_mutex_unlock(&g_frame_main_client_lock);
                    log_error("[daemon] max clients (%d) reached, rejecting main frame fd=%d",
                              MAX_CLIENTS, client_fd);
                    close(client_fd);
                }
            } else if (errno != EINTR) {
                log_error("accept (frame-main): %s", strerror(errno));
            }
        }

        /* ── Accept new connection on sub frame socket ───────────────── */
        if (fds[2].revents & POLLIN) {
            int client_fd = accept(g_frame_sub_server_fd, NULL, NULL);
            if (client_fd >= 0) {
                int has_existing = 0;
                pthread_mutex_lock(&g_frame_sub_client_lock);
                has_existing = (g_frame_sub_client_fd >= 0);
                if (!has_existing) {
                    g_frame_sub_client_fd = client_fd;
                }
                pthread_mutex_unlock(&g_frame_sub_client_lock);

                if (has_existing) {
                    log_warn("[daemon] sub frame client already connected, rejecting fd=%d", client_fd);
                    close(client_fd);
                } else if (nfds < MAX_CLIENTS + 3) {
                    fds[nfds].fd = client_fd;
                    fds[nfds].events = POLLIN;
                    fds[nfds].revents = 0;
                    nfds++;
                    log_info("[daemon] sub frame client connected fd=%d", client_fd);
                } else {
                    pthread_mutex_lock(&g_frame_sub_client_lock);
                    if (g_frame_sub_client_fd == client_fd) {
                        g_frame_sub_client_fd = -1;
                    }
                    pthread_mutex_unlock(&g_frame_sub_client_lock);
                    log_error("[daemon] max clients (%d) reached, rejecting sub frame fd=%d",
                              MAX_CLIENTS, client_fd);
                    close(client_fd);
                }
            } else if (errno != EINTR) {
                log_error("accept (frame-sub): %s", strerror(errno));
            }
        }

        /* ── Service clients with pending data (round-robin) ────────── */
        int i;
        int first_client_idx = 3;  /* fds[0..2] are server sockets */

        for (i = first_client_idx; i < nfds; i++) {
            if (!(fds[i].revents & (POLLIN | POLLHUP | POLLERR)))
                continue;

            {
                int client_fd = fds[i].fd;
                int main_client_fd = -1;
                int sub_client_fd = -1;
                int is_main_frame_client;
                int is_sub_frame_client;

                pthread_mutex_lock(&g_frame_main_client_lock);
                main_client_fd = g_frame_main_client_fd;
                pthread_mutex_unlock(&g_frame_main_client_lock);

                pthread_mutex_lock(&g_frame_sub_client_lock);
                sub_client_fd = g_frame_sub_client_fd;
                pthread_mutex_unlock(&g_frame_sub_client_lock);

                is_main_frame_client = (client_fd == main_client_fd);
                is_sub_frame_client = (client_fd == sub_client_fd);

                if (is_main_frame_client || is_sub_frame_client) {
                    if (fds[i].revents & (POLLHUP | POLLERR)) {
                        if (is_main_frame_client) {
                            pthread_mutex_lock(&g_frame_main_client_lock);
                            if (client_fd == g_frame_main_client_fd) {
                                g_frame_main_client_fd = -1;
                            }
                            pthread_mutex_unlock(&g_frame_main_client_lock);
                        }
                        if (is_sub_frame_client) {
                            pthread_mutex_lock(&g_frame_sub_client_lock);
                            if (client_fd == g_frame_sub_client_fd) {
                                g_frame_sub_client_fd = -1;
                            }
                            pthread_mutex_unlock(&g_frame_sub_client_lock);
                        }
                        log_info("[daemon] %s frame client disconnected fd=%d",
                                 is_sub_frame_client ? "sub" : "main", client_fd);
                        close(client_fd);
                        fds[i] = fds[nfds - 1];
                        nfds--;
                        i--;
                    }
                    continue;
                }

                {
                    int ret = process_request(client_fd);
                    if (ret == -1) {
                        /* Control client disconnected or I/O error —
                         * stop push threads before releasing control so
                         * the SDK state is clean for the next client. */
                        log_info("[daemon] control client fd=%d disconnected, stopping push threads", client_fd);
                        stop_push_slot(0);
                        stop_push_slot(1);
                        release_control(client_fd);
                        close(client_fd);
                        fds[i] = fds[nfds - 1];
                        nfds--;
                        i--;
                    } else if (ret == -2) {
                        /* CMD_SHUTDOWN */
                        g_shutdown = 1;
                        break;
                    }
                }
            }
        }
    }

    /* Clean up any remaining client connections */
    {
        int i;
        int first_client_idx = 3;
        for (i = first_client_idx; i < nfds; i++) {
            release_control(fds[i].fd);
            close(fds[i].fd);
        }
    }

shutdown:
    /* ================================================================
     * SHUTDOWN
     * ================================================================ */

    log_info("[daemon] shutting down");

    /* Close frame sockets */
    if (g_frame_main_server_fd >= 0) {
        close(g_frame_main_server_fd);
        g_frame_main_server_fd = -1;
        unlink(FRAME_MAIN_SOCKET_PATH);
    }
    if (g_frame_sub_server_fd >= 0) {
        close(g_frame_sub_server_fd);
        g_frame_sub_server_fd = -1;
        unlink(FRAME_SUB_SOCKET_PATH);
    }

    /* Close frame clients */
    pthread_mutex_lock(&g_frame_main_client_lock);
    if (g_frame_main_client_fd >= 0) {
        close(g_frame_main_client_fd);
        g_frame_main_client_fd = -1;
    }
    pthread_mutex_unlock(&g_frame_main_client_lock);

    pthread_mutex_lock(&g_frame_sub_client_lock);
    if (g_frame_sub_client_fd >= 0) {
        close(g_frame_sub_client_fd);
        g_frame_sub_client_fd = -1;
    }
    pthread_mutex_unlock(&g_frame_sub_client_lock);

    /* Close control socket */
    if (g_ctrl_server_fd >= 0) {
        close(g_ctrl_server_fd);
        unlink(CTRL_SOCKET_PATH);
    }

    /* Stop push threads before destroying ring buffer */
    log_info("event=push_thread_lifecycle state=shutdown_stop_begin diag_monotonic_ms=%llu",
             (unsigned long long)diag_monotonic_ms());
    stop_push_slot(0);
    stop_push_slot(1);
    log_info("event=push_thread_lifecycle state=shutdown_stop_done diag_monotonic_ms=%llu",
             (unsigned long long)diag_monotonic_ms());

    /* Clean up shared memory ring buffer */
    if (g_ring_buffer) {
        vd_ring_shutdown(g_ring_buffer);
        vd_ring_destroy(g_ring_buffer, 1);
        g_ring_buffer = NULL;
    }

    log_info("vendor-daemon stopped");
    log_info("========================================");

    /* Restore original stdout/stderr */
    if (g_saved_stdout >= 0) {
        dup2(g_saved_stdout, STDOUT_FILENO);
        close(g_saved_stdout);
    }
    if (g_saved_stderr >= 0) {
        dup2(g_saved_stderr, STDERR_FILENO);
        close(g_saved_stderr);
    }
    if (g_log_fp) {
        fclose(g_log_fp);
    }

    return 0;
}
